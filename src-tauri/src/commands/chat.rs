//! `chat_send` + `chat_cancel`-Commands.
//!
//! `chat_send` ist v1-non-streaming: spawnt pro Call eine neue
//! `CopilotBridge` (mit eigenem Subprozess), sammelt alle Stream-Chunks
//! in einen einzigen Response-String und killt den Subprozess beim Drop.
//!
//! v2 könnte das auf persistente Bridge + request/response-correlation
//! umstellen (signifikante Komplexitäts-Reduktion pro Message).

use tauri::State;
use tokio_stream::StreamExt;

use crate::state::AppState;

/// Sendet eine Chat-Message und gibt die vollständige Antwort zurück
/// (non-streaming). Für Echtzeit-Streaming käme in v2 ein
/// Tauri-Event-Channel (`chat.chunk`).
#[tauri::command]
pub async fn chat_send(
    message: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let config = state
        .config
        .lock()
        .await
        .clone()
        .ok_or_else(|| "no config — please configure first".to_string())?;

    let mut bridge = crate::copilot::spawn_bridge(&state.exe_dir, config)
        .await
        .map_err(|e| format!("spawn bridge: {}", e))?;

    let mut stream = bridge
        .chat_streaming(message)
        .await
        .map_err(|e| format!("chat_streaming: {}", e))?;

    let mut response = String::new();
    while let Some(chunk) = stream.next().await {
        response.push_str(&chunk.text);
    }

    // Bridge wird gedroppt → kill_on_drop=true killt Subprozess sauber.
    drop(bridge);

    Ok(response)
}

/// Bricht eine laufende Chat-Anfrage ab. v1: No-op (Log-Warnung).
/// v2 könnte einen AbortController einsetzen.
#[tauri::command]
pub async fn chat_cancel(
    _request_id: String,
    _state: State<'_, AppState>,
) -> Result<(), String> {
    log::warn!("chat_cancel called but not yet implemented (v1 no-op)");
    Ok(())
}