//! `process_health` + `process_restart`-Commands.
//!
//! Architektur-Stand (2026-07-30): `CopilotBridge` (SDK-Client +
//! CLI-Subprozess) lebt persistent in `AppState.bridge` und wird
//! zwischen Message-Calls reused. Per-Request gibt es eine frische
//! `Session` (siehe `commands/chat::chat_send`).
//!
//! `process_health` liefert Status-Infos über die Bridge (initialised
//! oder nicht). `process_restart` killt die aktive Session und
//! drop't die Bridge -> der naechste `chat_send` spawnt einen
//! frischen Client.

use serde::Serialize;
use tauri::State;

use crate::state::AppState;

#[derive(Serialize, Clone, Debug)]
pub struct HealthDto {
    pub config_loaded: bool,
    pub bridge_initialised: bool,
    pub active_session: bool,
    pub note: &'static str,
}

/// Liefert Health-Informationen über die Bridge/AppState.
#[tauri::command]
pub async fn process_health(
    state: State<'_, AppState>,
) -> Result<HealthDto, String> {
    let config_loaded = state.config.lock().await.is_some();
    let bridge_initialised = state.bridge.lock().await.is_some();
    let active_session = state.active_session.lock().await.is_some();

    Ok(HealthDto {
        config_loaded,
        bridge_initialised,
        active_session,
        note: "Persistent SDK-Client + Per-Message Sessions",
    })
}

/// Killt die aktive Session (sofern vorhanden) und drop't die
/// Bridge. Der naechste `chat_send` spawnt einen frischen Client.
#[tauri::command]
pub async fn process_restart(
    state: State<'_, AppState>,
) -> Result<(), String> {
    // Session disconnecten (falls aktiv)
    {
        let mut active = state.active_session.lock().await;
        if let Some(active) = active.take() {
            let session = active.session.lock().await;
            if let Err(e) = session.abort().await {
                log::warn!("process_restart: session.abort failed: {}", e);
            }
            if let Err(e) = session.disconnect().await {
                log::warn!("process_restart: session.disconnect failed: {}", e);
            }
        }
    }
    // Bridge droppen -> Drop-Impl ruft client.stop()
    {
        let mut bridge = state.bridge.lock().await;
        *bridge = None;
    }
    log::info!("process_restart: bridge cleared, next chat_send will respawn");
    Ok(())
}