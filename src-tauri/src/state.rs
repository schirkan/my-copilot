//! Globaler App-State für die Tauri-Runtime.
//!
//! Wird via `app.manage()` in lib.rs::run() registriert. Tauri-Commands
//! erhalten ihn als `State<'_, AppState>`-Parameter (siehe commands/).

use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use tauri::AppHandle;
use tokio::sync::Mutex;

use crate::copilot::{ByokConfig, CopilotBridge};
use github_copilot_sdk::session::Session;

/// Aktive Chat-Session, die gerade einen Stream verarbeitet. Wird in
/// `AppState.active_session` gehalten, damit `chat_cancel` parallel zum
/// Streaming-Task `session.abort()` aufrufen kann.
///
/// `Arc<Mutex<Session>>` ist noetig, weil `Session` selbst nicht
/// `Clone` ist und wir von zwei Tasks (Stream-Loop + Cancel-Handler) aus
/// shareable darauf zugreifen muessen. `Session`-Methoden (`abort`,
/// `disconnect`, etc.) nehmen alle `&self`, der Mutex dient nur dem
/// Ownership-Sharing.
pub struct ActiveSession {
    pub request_id: String,
    pub session: Arc<Mutex<Session>>,
}

/// Globaler App-State, der von Tauri-Commands geteilt wird.
///
/// Enthält:
/// - `app_handle`: für Tauri-Event-Emission (`chat_chunk`, `chat_done`,
///   `chat_error`) und Tauri-APIs.
/// - `exe_dir`: Pfad zum exe-Verzeichnis (für Working-Dir-Defaults).
/// - `config`: aktuelle BYOK-Konfiguration. Wird aus `config.json`
///   geladen (`lib.rs::run`) und per `config_set`-Command aktualisiert.
/// - `bridge`: persistente SDK-Client-Bridge. Wird beim ersten
///   `chat_send` lazy erzeugt und für die App-Lifetime in
///   `Mutex<Option<Arc<CopilotBridge>>>` gehalten (persistent -- nicht
///   mehr pro Message neu spawnen, siehe `commands::chat::chat_send`).
///   `Arc` noetig, weil `CopilotBridge` selbst nicht `Clone` ist und
///   wir den Client-Bridge zwischen Streaming-Task und Cancel-Handler
///   teilen wollen.
/// - `active_session`: aktuell laufende Chat-Session (während
///   Streaming). `None` wenn kein Chat aktiv. Wird vom Streaming-Task
///   gesetzt und beim `session.idle`/`session.error` wieder geleert.
///   Genutzt von `chat_cancel` zum `session.abort()`.
/// - `healthy`: Health-Flag, von `process_health` ausgewertet.
pub struct AppState {
    pub app_handle: AppHandle,
    pub exe_dir: PathBuf,
    pub config: Mutex<Option<ByokConfig>>,
    pub bridge: Mutex<Option<Arc<CopilotBridge>>>,
    pub active_session: Mutex<Option<ActiveSession>>,
    pub healthy: AtomicBool,
}

impl AppState {
    pub fn new(app_handle: AppHandle, exe_dir: PathBuf) -> Self {
        Self {
            app_handle,
            exe_dir,
            config: Mutex::new(None),
            bridge: Mutex::new(None),
            active_session: Mutex::new(None),
            healthy: AtomicBool::new(false),
        }
    }
}