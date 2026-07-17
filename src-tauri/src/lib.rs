pub mod commands;
pub mod copilot;
pub mod state;

use state::AppState;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Logger initialisieren (logs gehen nach stderr — Card #4 schreibt
    // sie zusätzlich in data/logs/).
    let _ = env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("info")
    )
    .format_timestamp_millis()
    .try_init();

    tauri::Builder::default()
        .setup(|app| {
            // exe-Verzeichnis bestimmen (für CopilotCliProcess::start)
            let exe_dir = std::env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(|p| p.to_path_buf()))
                .unwrap_or_else(|| std::path::PathBuf::from("."));
            let mut state = AppState::default();
            state.exe_dir = exe_dir;
            log::info!("AppState initialisiert: exe_dir = {:?}", state.exe_dir);
            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::chat::chat_send,
            commands::chat::chat_cancel,
            commands::config::config_get,
            commands::config::config_set,
            commands::config::config_test,
            commands::process::process_health,
            commands::process::process_restart,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}