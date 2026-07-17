pub mod copilot;

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
        .setup(|_app| Ok(()))
        .invoke_handler(tauri::generate_handler![])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}