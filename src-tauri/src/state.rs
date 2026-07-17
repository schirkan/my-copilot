//! Globaler App-State für die Tauri-Runtime.
//!
//! Wird via `app.manage()` in lib.rs::run() registriert. Tauri-Commands
//! erhalten ihn als `State<'_, AppState>`-Parameter (siehe commands/).

use std::path::PathBuf;
use std::sync::atomic::AtomicBool;

use tokio::sync::Mutex;

use crate::copilot::{ByokConfig, CopilotBridge};

/// Globaler App-State, der von Tauri-Commands geteilt wird.
///
/// Enthält:
/// - `exe_dir`: Pfad zum exe-Verzeichnis (für `CopilotCliProcess::start`)
/// - `config`: aktuelle BYOK-Konfiguration. Wird in Card #4 aus
///   `config.json` + DPAPI geladen, hier zunächst in-memory gesetzt via
///   `config_set`-Command.
/// - `bridge`: aktive Bridge zum CLI-Subprozess (lazy erzeugt beim ersten
///   `chat_send`-Call). Wird nach Gebrauch gedroppt → kill_on_drop=true
///   killt den Subprozess sauber.
/// - `healthy`: Health-Flag, von `process_health` ausgewertet.
#[derive(Default)]
pub struct AppState {
    pub exe_dir: PathBuf,
    pub config: Mutex<Option<ByokConfig>>,
    pub bridge: Mutex<Option<CopilotBridge>>,
    pub healthy: AtomicBool,
}