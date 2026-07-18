//! Read-Operationen für Chat-Messages und Session-Liste.

use std::path::Path;
use tokio::io::AsyncBufReadExt;

use super::session::{session_path, sessions_dir, Message, SessionMeta};

pub async fn load_session(exe_dir: &Path, session_id: &str) -> Result<Vec<Message>, String> {
    let path = session_path(exe_dir, session_id);
    if !path.exists() {
        return Ok(Vec::new());
    }

    let file = tokio::fs::File::open(&path)
        .await
        .map_err(|e| format!("open: {}", e))?;
    let reader = tokio::io::BufReader::new(file);
    let mut lines = reader.lines();

    let mut messages = Vec::new();
    while let Some(line) = lines
        .next_line()
        .await
        .map_err(|e| format!("line: {}", e))?
    {
        if line.trim().is_empty() {
            continue;
        }
        let msg: Message = serde_json::from_str(&line)
            .map_err(|e| format!("parse: {} (line: {})", e, line))?;
        messages.push(msg);
    }
    Ok(messages)
}

pub async fn list_sessions(exe_dir: &Path) -> Result<Vec<SessionMeta>, String> {
    let dir = sessions_dir(exe_dir);
    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut entries = tokio::fs::read_dir(&dir)
        .await
        .map_err(|e| format!("read_dir: {}", e))?;
    let mut metas = Vec::new();

    while let Some(entry) = entries
        .next_entry()
        .await
        .map_err(|e| format!("next_entry: {}", e))?
    {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("jsonl") {
            continue;
        }
        let session_id = match path.file_stem().and_then(|s| s.to_str()) {
            Some(s) if !s.is_empty() => s.to_string(),
            _ => continue,
        };

        let content = match tokio::fs::read_to_string(&path).await {
            Ok(c) => c,
            Err(_) => continue,
        };

        let messages: Vec<Message> = content
            .lines()
            .filter(|l| !l.trim().is_empty())
            .filter_map(|l| serde_json::from_str(l).ok())
            .collect();

        if messages.is_empty() {
            continue;
        }

        let created_at = messages.first().map(|m| m.ts.clone()).unwrap_or_default();
        let updated_at = messages.last().map(|m| m.ts.clone()).unwrap_or_default();
        let model = messages.first().map(|m| m.model.clone()).unwrap_or_default();
        let title = messages
            .iter()
            .find(|m| m.role == "user")
            .map(|m| {
                if m.content.chars().count() > 50 {
                    let truncated: String = m.content.chars().take(50).collect();
                    format!("{}…", truncated)
                } else {
                    m.content.clone()
                }
            })
            .unwrap_or_else(|| session_id.clone());
        let message_count = messages.len() as u64;

        metas.push(SessionMeta {
            session_id,
            created_at,
            updated_at,
            message_count,
            model,
            title,
        });
    }

    metas.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    Ok(metas)
}

pub async fn delete_session(exe_dir: &Path, session_id: &str) -> Result<(), String> {
    let path = session_path(exe_dir, session_id);
    if path.exists() {
        tokio::fs::remove_file(&path)
            .await
            .map_err(|e| format!("remove_file: {}", e))?;
    }
    Ok(())
}