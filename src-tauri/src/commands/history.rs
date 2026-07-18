//! History-IPC-Commands: Session-Liste, Session laden, Session löschen,
//! Message appenden.

use tauri::State;

use crate::persistence::{delete_session, list_sessions, load_session, Message, SessionMeta};
use crate::state::AppState;

#[tauri::command]
pub async fn history_list_sessions(
    state: State<'_, AppState>,
) -> Result<Vec<SessionMeta>, String> {
    let exe_dir = state.exe_dir.clone();
    list_sessions(&exe_dir).await
}

#[tauri::command]
pub async fn history_load_session(
    session_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<Message>, String> {
    let exe_dir = state.exe_dir.clone();
    load_session(&exe_dir, &session_id).await
}

#[tauri::command]
pub async fn history_delete_session(
    session_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let exe_dir = state.exe_dir.clone();
    delete_session(&exe_dir, &session_id).await
}

#[tauri::command]
pub async fn history_append_message(
    message: Message,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let exe_dir = state.exe_dir.clone();
    crate::persistence::append_message(&exe_dir, &message).await
}