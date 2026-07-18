//! Append-Operationen für Chat-Messages.

use std::path::Path;
use tokio::io::AsyncWriteExt;

use super::session::{session_path, sessions_dir, Message};

pub async fn append_message(exe_dir: &Path, message: &Message) -> Result<(), String> {
    let dir = sessions_dir(exe_dir);
    tokio::fs::create_dir_all(&dir)
        .await
        .map_err(|e| format!("create_dir_all: {}", e))?;

    let path = session_path(exe_dir, &message.request_id);
    let mut file = tokio::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .await
        .map_err(|e| format!("open: {}", e))?;

    let line = serde_json::to_string(message)
        .map_err(|e| format!("serialize: {}", e))?;
    file.write_all(line.as_bytes())
        .await
        .map_err(|e| format!("write: {}", e))?;
    file.write_all(b"\n")
        .await
        .map_err(|e| format!("newline: {}", e))?;
    file.flush()
        .await
        .map_err(|e| format!("flush: {}", e))?;
    Ok(())
}