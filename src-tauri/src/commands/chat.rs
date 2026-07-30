//! `chat_send` + `chat_cancel`-Commands (v2 Streaming).
//!
//! Architektur-Update (siehe SPEC-004 § Streaming-Optimierung +
//! `commands::chat::chat_send` Doc):
//!
//! - **Persistent bridge**: `CopilotBridge` (und damit der SDK-Client
//!   + CLI-Subprozess) lebt in `AppState.bridge` und wird zwischen
//!   Message-Calls reused. Vorher (v1) wurde pro Chat-Message ein
//!   neuer `Client::start()` -> `client.stop()` Roundtrip gemacht,
//!   was bei schnellem Chat spuerbar Latenz kostet.
//! - **Per-Message Sessions**: Jede User-Message bekommt eine eigene
//!   `Session`. Sessions kapseln den CLI-State (History, Tools, ...)
//!   pro Request. `subscribe() -> send() -> stream-loop -> disconnect()`
//!   ist jetzt ein klares Lifecycle-Pattern.
//! - **Event-Streaming**: `Session::subscribe()` liefert
//!   `assistant.message_delta`-Events, die als `chat_chunk` Tauri-
//!   Events ans Frontend emittiert werden. `session.idle` -> `chat_done`,
//!   `session.error` -> `chat_error`. User-Message wird VOR dem Stream
//!   persistiert, Assistant-Message NACH `session.idle`.
//! - **Synchronous Cancellation**: `chat_cancel` ruft `session.abort()`
//!   auf der aktiven Session (Arc<Mutex<Session>> in AppState). Der
//!   Stream-Loop sieht darauf `session.idle` bzw. `session.error` und
//!   räumt auf.

use std::path::PathBuf;
use std::sync::Arc;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};

use github_copilot_sdk::session::Session;
use github_copilot_sdk::subscription::EventSubscription;
use github_copilot_sdk::MessageOptions;

use crate::state::{ActiveSession, AppState};
use crate::copilot::CopilotBridge;

// ---------------------------------------------------------------------------
// Event-Payloads
// ---------------------------------------------------------------------------

/// Payload fuer `chat_chunk`-Event: ein einzelnes Text-Delta der
/// Assistant-Response. Frontend appended `delta` an die aktuelle
/// Assistant-Bubble. `accumulated` ist der bisherige Full-Text (fuer
/// Frontends, die nicht selbst inkrementell appenden wollen).
#[derive(Debug, Clone, Serialize)]
pub struct ChatChunkPayload {
    pub request_id: String,
    pub delta: String,
    pub accumulated: String,
}

/// Payload fuer `chat_done`-Event: vollstaendige Assistant-Antwort.
#[derive(Debug, Clone, Serialize)]
pub struct ChatDonePayload {
    pub request_id: String,
    pub content: String,
}

/// Payload fuer `chat_error`-Event: Fehlermeldung der Session.
#[derive(Debug, Clone, Serialize)]
pub struct ChatErrorPayload {
    pub request_id: String,
    pub error: String,
}

// ---------------------------------------------------------------------------
// Tauri-Commands
// ---------------------------------------------------------------------------

/// Response auf `chat_send`: `session_id` ist die **stabile** Session-ID
/// (mehrere Messages teilen dieselbe ID), `request_id` ist die
/// **transiente** Korrelations-ID fuer die Streaming-Events dieses
/// einzelnen Calls.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ChatSendResponse {
    pub session_id: String,
    pub request_id: String,
}

/// Sendet eine Chat-Message und returnt **sofort** `{session_id, request_id}`.
/// Die eigentliche Verarbeitung laeuft asynchron im Hintergrund; der
/// Client konsumiert `chat_chunk`/`chat_done`/`chat_error`-Events.
///
/// Argument-Shape: Tauri 2 mappt Top-Level-Argumente mit camelCase ->
/// snake_case automatisch. Frontend muss also `{ message, sessionId }`
/// schicken (NICHT `{ args: { ... } }` wickeln). Wenn `session_id`
/// `None` ist, generiert Rust eine neue UUID und returnt sie.
#[tauri::command]
pub async fn chat_send(
    message: String,
    session_id: Option<String>,
    state: State<'_, AppState>,
) -> Result<ChatSendResponse, String> {
    log::info!(
        "chat_send called: message_len={}, session_id={:?}",
        message.len(),
        session_id
    );
    let session_id = session_id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    let request_id = uuid::Uuid::new_v4().to_string();
    let exe_dir = state.exe_dir.clone();
    let app_handle = state.app_handle.clone();

    // Config aus State holen (clonen, da wir sie hier konsumieren)
    let config = state
        .config
        .lock()
        .await
        .clone()
        .ok_or_else(|| "no config — please configure first".to_string())?;

    let model_name = config.model.clone();

    // User-Message persistieren (VOR Streaming -- so bleibt die
    // History auch bei spaeterem Crash erhalten). Persistenz-Key
    // ist `session_id` (stabil ueber Messages), nicht `request_id`
    // (pro Message neu).
    let user_msg = crate::persistence::Message {
        id: uuid::Uuid::new_v4().to_string(),
        request_id: session_id.clone(),
        role: "user".to_string(),
        content: message.clone(),
        ts: crate::persistence::now_iso(),
        model: model_name.clone(),
        tokens: 0,
    };
    crate::persistence::append_message(&exe_dir, &user_msg)
        .await
        .map_err(|e| format!("append user message: {}", e))?;
    log::info!("user message persisted for session_id={}", session_id);

    // Persistent bridge holen (lazy init) oder bei config-Mismatch
    // neu erzeugen.
    let bridge = ensure_bridge(&state, &exe_dir, config).await?;
    log::info!("bridge ready for session_id={}", session_id);

    // Session fuer diesen Request erstellen
    let session = bridge
        .create_session()
        .await
        .map_err(|e| format!("create session: {}", e))?;
    log::info!("session created for request_id={}", request_id);

    // WICHTIG: subscribe() VOR send() aufrufen, sonst verpasst der
    // Subscriber fruehe Events (siehe SDK `Session::subscribe` Docs).
    let events = session.subscribe();
    let session_arc = Arc::new(tokio::sync::Mutex::new(session));

    // Session in AppState registrieren, damit chat_cancel sie findet
    // und .abort() aufrufen kann.
    {
        let mut active = state.active_session.lock().await;
        *active = Some(ActiveSession {
            request_id: request_id.clone(),
            session: session_arc.clone(),
        });
    }

    // User-Message an die CLI senden (fire-and-forget; returnt sofort
    // die Message-ID). Fehler hier propagated an den Caller, da die
    // Session noch nicht in den Stream-Modus gewechselt ist.
    {
        let session = session_arc.lock().await;
        session
            .send(MessageOptions::new(message.clone()))
            .await
            .map_err(|e| format!("send: {}", e))?;
    }
    log::info!(
        "session.send ok for request_id={} (streaming now)",
        request_id
    );

    // Streaming-Task im Hintergrund starten. Er konsumiert Events
    // und emittiert sie als Tauri-Events ans Frontend.
    let session_id_for_task = session_id.clone();
    let request_id_for_task = request_id.clone();
    let exe_dir_for_task = exe_dir.clone();
    let model_for_task = model_name.clone();
    tauri::async_runtime::spawn(async move {
        run_stream_loop(
            app_handle,
            session_arc,
            events,
            session_id_for_task,
            request_id_for_task,
            exe_dir_for_task,
            model_for_task,
        )
        .await;
    });

    Ok(ChatSendResponse {
        session_id,
        request_id,
    })
}

/// Bricht eine laufende Chat-Anfrage ab. Ruft `session.abort()` auf
/// der aktiven Session, falls die `request_id` matcht. Der
/// Stream-Loop sieht darauf `session.idle`/`session.error` und
/// emittiert `chat_done` (mit partial content) oder `chat_error`.
#[tauri::command]
pub async fn chat_cancel(
    request_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let active = state.active_session.lock().await;
    if let Some(active) = active.as_ref() {
        if active.request_id != request_id {
            return Err(format!(
                "no active session with request_id={} (current={})",
                request_id, active.request_id
            ));
        }
        let session = active.session.lock().await;
        session.abort().await.map_err(|e| format!("abort: {}", e))?;
        log::info!("chat_cancel: aborted session for request_id={}", request_id);
        Ok(())
    } else {
        Err("no active chat session".to_string())
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Holt den persistenten `CopilotBridge` aus `AppState` oder erzeugt
/// einen neuen, falls (a) noch keiner existiert oder (b) sich
/// Endpoint/Model geaendert haben.
///
/// Lock wird sofort wieder freigegeben, sobald der `Arc<CopilotBridge>`
/// aus dem Mutex herausgenommen ist -- der eigentliche Streaming-Task
/// haelt dann nur die Arc-Referenz.
async fn ensure_bridge(
    state: &State<'_, AppState>,
    exe_dir: &PathBuf,
    config: crate::copilot::ByokConfig,
) -> Result<Arc<CopilotBridge>, String> {
    let mut bridge_guard = state.bridge.lock().await;
    if let Some(existing) = bridge_guard.as_ref() {
        if existing.config().endpoint == config.endpoint
            && existing.config().model == config.model
        {
            return Ok(existing.clone());
        }
        log::info!(
            "config mismatch (e={}, m={}) -> recreating bridge",
            config.endpoint,
            config.model
        );
    }
    let new_bridge = Arc::new(
        CopilotBridge::new(exe_dir, config)
            .await
            .map_err(|e| format!("spawn bridge: {}", e))?,
    );
    *bridge_guard = Some(new_bridge.clone());
    Ok(new_bridge)
}

/// Stream-Loop: konsumiert `SessionEvent`s aus `events`, emittiert
/// Tauri-Events an das Frontend, und persistiert die fertige
/// Assistant-Message am Ende. Wird in `tauri::async_runtime::spawn`
/// gestartet und laeuft bis `session.idle`, `session.error` oder
/// Subscriber-Closed.
async fn run_stream_loop(
    app_handle: AppHandle,
    session_arc: Arc<tokio::sync::Mutex<Session>>,
    mut events: EventSubscription,
    session_id: String,
    request_id: String,
    exe_dir: PathBuf,
    model_name: String,
) {
    log::info!(
        "run_stream_loop started for request_id={} session_id={}",
        request_id,
        session_id
    );
    let mut accumulated = String::new();
    let mut last_error: Option<String> = None;
    let mut event_count: u64 = 0;

    while let Ok(event) = events.recv().await {
        event_count += 1;
        log::debug!(
            "recv event #{} for request_id={}: type={} data={}",
            event_count,
            request_id,
            event.event_type,
            event.data
        );
        match event.event_type.as_str() {
            "assistant.message_delta" => {
                // Streaming-Chunk: an Frontend durchreichen.
                // Manche Provider senden `deltaContent` (neuere Schema),
                // andere `delta` (aelteres Schema). Wir akzeptieren beide.
                let delta = event
                    .data
                    .get("deltaContent")
                    .or_else(|| event.data.get("delta"))
                    .and_then(|v| v.as_str());
                if let Some(delta) = delta {
                    accumulated.push_str(delta);
                    let _ = app_handle.emit(
                        "chat_chunk",
                        ChatChunkPayload {
                            request_id: request_id.clone(),
                            delta: delta.to_string(),
                            accumulated: accumulated.clone(),
                        },
                    );
                }
            }
            "assistant.message" => {
                // Fallback: Manche Provider (offenbar MiniMax M3) senden
                // NICHT message_delta-Events, sondern das vollstaendige
                // `assistant.message` als ein Event. Wir uebernehmen den
                // Content als accumulated und emittieren ein chat_chunk,
                // damit das Frontend ein Update bekommt.
                if let Some(content) = event
                    .data
                    .get("content")
                    .and_then(|v| v.as_str())
                {
                    if accumulated.is_empty() {
                        accumulated = content.to_string();
                        let _ = app_handle.emit(
                            "chat_chunk",
                            ChatChunkPayload {
                                request_id: request_id.clone(),
                                delta: content.to_string(),
                                accumulated: accumulated.clone(),
                            },
                        );
                    }
                }
            }
            "session.idle" | "assistant.idle" => {
                // Antwort fertig -> chat_done. Manche Provider senden
                // `session.idle`, andere `assistant.idle` (am Ende jedes
                // Turns). Wir akzeptieren beides.
                log::info!(
                    "{} recv for request_id={}, accumulated_len={}",
                    event.event_type,
                    request_id,
                    accumulated.len()
                );
                let _ = app_handle.emit(
                    "chat_done",
                    ChatDonePayload {
                        request_id: request_id.clone(),
                        content: accumulated.clone(),
                    },
                );
                break;
            }
            "session.error" => {
                // Fehler vom Agent -> chat_error.
                let error_msg = event
                    .data
                    .get("errorMessage")
                    .or_else(|| event.data.get("message"))
                    .or_else(|| event.data.get("error"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("session error")
                    .to_string();
                log::warn!(
                    "session.error for request_id={}: {}",
                    request_id,
                    error_msg
                );
                last_error = Some(error_msg);
                break;
            }
            _ => {
                // Andere Events (tool.*, session.*, ...) werden
                // bewusst ignoriert. TODO v2: separater Event-Channel
                // fuer Tool-Progress etc.
            }
        }
    }
    log::info!(
        "run_stream_loop exiting for request_id={}, total_events={}, accumulated_len={}, last_error={:?}",
        request_id,
        event_count,
        accumulated.len(),
        last_error
    );

    // Cleanup: Session disconnecten (sendet session.destroy RPC) und
    // aus AppState entfernen, damit der naechste chat_send wieder
    // eine frische Session aufmachen kann.
    {
        let session = session_arc.lock().await;
        if let Err(e) = session.disconnect().await {
            log::warn!(
                "session.disconnect failed for request_id={}: {}",
                request_id,
                e
            );
        }
    }
    {
        let state = app_handle.state::<AppState>();
        let mut active = state.active_session.lock().await;
        if active
            .as_ref()
            .map(|a| a.request_id == request_id)
            .unwrap_or(false)
        {
            *active = None;
        }
    }

    // Assistant-Message persistieren (oder Fehler-Bubble ans Frontend).
    match last_error {
        Some(err) => {
            let _ = app_handle.emit(
                "chat_error",
                ChatErrorPayload {
                    request_id: request_id.clone(),
                    error: err.clone(),
                },
            );
            // Keine Persistenz der Assistant-Message bei Fehler --
            // History zeigt nur den User-Prompt, das Error-Marker-
            // Rendering uebernimmt das Frontend.
        }
        None => {
            let assistant_msg = crate::persistence::Message {
                id: uuid::Uuid::new_v4().to_string(),
                // Persistenz-Key ist die **stabile** Session-ID, damit
                // User + Assistant beider Messages im selben JSONL landen.
                request_id: session_id.clone(),
                role: "assistant".to_string(),
                content: accumulated.clone(),
                ts: crate::persistence::now_iso(),
                model: model_name.clone(),
                tokens: 0, // v1: Placeholder; echte Token-Count kommt wenn Copilot-CLI usage-Stats liefert
            };
            if let Err(e) =
                crate::persistence::append_message(&exe_dir, &assistant_msg).await
            {
                log::error!(
                    "append assistant message failed for session_id={}: {}",
                    session_id,
                    e
                );
            }
        }
    }
}
