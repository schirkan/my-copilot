//! Globaler App-State für die Tauri-Runtime.
//!
//! Wird via `app.manage()` in lib.rs::run() registriert. Tauri-Commands
//! erhalten ihn als `State<'_, AppState>`-Parameter (siehe commands/).

use std::path::PathBuf;
use std::sync::atomic::AtomicBool;

use tauri::AppHandle;
use tokio::sync::Mutex;

use crate::copilot::{ByokConfig, CopilotBridge};

/// Globaler App-State, der von Tauri-Commands geteilt wird.
///
/// Enthält:
/// - `app_handle`: für Tauri-APIs (z. B. Event-Emit)
/// - `exe_dir`: Pfad zum exe-Verzeichnis (für Working-Dir-Defaults)
/// - `config`: aktuelle BYOK-Konfiguration. Wird aus `config.json` geladen
///   (`lib.rs::run`) und per `config_set`-Command aktualisiert.
/// - `bridge`: aktive Bridge zum CLI-Subprozess (lazy erzeugt beim ersten
///   `chat_send`-Call). Wird nach Gebrauch gedroppt → kill_on_drop=true
///   killt den Subprozess sauber.
/// - `healthy`: Health-Flag, von `process_health` ausgewertet.
pub struct AppState {
    pub app_handle: AppHandle,
    pub exe_dir: PathBuf,
    pub config: Mutex<Option<ByokConfig>>,
    pub bridge: Mutex<Option<CopilotBridge>>,
    pub healthy: AtomicBool,
}

impl AppState {
    pub fn new(app_handle: AppHandle, exe_dir: PathBuf) -> Self {
        Self {
            app_handle,
            exe_dir,
            config: Mutex::new(None),
            bridge: Mutex::new(None),
            healthy: AtomicBool::new(false),
        }
    }
}