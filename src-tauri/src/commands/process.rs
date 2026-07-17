//! `process_health` + `process_restart`-Commands.

use serde::Serialize;
use tauri::State;

use crate::state::AppState;

#[derive(Serialize, Clone, Debug)]
pub struct HealthDto {
    pub cli_running: bool,
    pub cli_ready: bool,
    pub bridge_initialized: bool,
}

/// Liefert Health-Informationen über den CLI-Subprozess.
#[tauri::command]
pub async fn process_health(
    state: State<'_, AppState>,
) -> Result<HealthDto, String> {
    let mut bridge_guard = state.bridge.lock().await;
    match bridge_guard.as_mut() {
        Some(bridge) => {
            let running = bridge
                .process_mut()
                .try_wait()
                .map_err(|e| format!("try_wait: {}", e))?
                .is_none();
            Ok(HealthDto {
                cli_running: running,
                cli_ready: running,
                bridge_initialized: true,
            })
        }
        None => Ok(HealthDto {
            cli_running: false,
            cli_ready: false,
            bridge_initialized: false,
        }),
    }
}

/// Killt den CLI-Subprozess (Bridge-Drop → kill_on_drop).
/// v1: kein Auto-Restart; nächster chat_send spawnt eine neue Bridge.
#[tauri::command]
pub async fn process_restart(
    state: State<'_, AppState>,
) -> Result<(), String> {
    let mut bridge_guard = state.bridge.lock().await;
    if let Some(mut bridge) = bridge_guard.take() {
        bridge
            .process_mut()
            .kill()
            .await
            .map_err(|e| format!("kill: {}", e))?;
        log::info!("CLI-Subprozess per process_restart gekillt");
    } else {
        log::info!("process_restart: keine aktive Bridge, nichts zu tun");
    }
    Ok(())
}