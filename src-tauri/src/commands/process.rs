//! `process_health` + `process_restart`-Commands.
//!
//! In der neuen Architektur (per-Request ACP-Subprozess) gibt es
//! keine persistente Bridge im AppState mehr. `process_health`
//! liefert daher einfache Status-Infos; `process_restart` ist ein
//! No-op (der nächste `chat_send`-Call spawnt ohnehin einen
//! frischen Subprozess).

use serde::Serialize;
use tauri::State;

use crate::state::AppState;

#[derive(Serialize, Clone, Debug)]
pub struct HealthDto {
    pub config_loaded: bool,
    pub last_chat_ok: bool,
    pub note: &'static str,
}

/// Liefert Health-Informationen über die Bridge/AppState.
#[tauri::command]
pub async fn process_health(
    state: State<'_, AppState>,
) -> Result<HealthDto, String> {
    let config_loaded = state.config.lock().await.is_some();

    Ok(HealthDto {
        config_loaded,
        last_chat_ok: false, // wird in v2 mit echter Telemetry ersetzt
        note: "Per-Request Rust-SDK-Session (kein persistent state)",
    })
}

/// Killt laufende Subprozesse. In der aktuellen Architektur
/// existiert keine persistente Bridge; jeder `chat_send`-Call
/// spawnt einen frischen Subprozess und killt ihn am Ende.
#[tauri::command]
pub async fn process_restart(
    _state: State<'_, AppState>,
) -> Result<(), String> {
    log::info!("process_restart: no-op (per-request spawn model)");
    Ok(())
}