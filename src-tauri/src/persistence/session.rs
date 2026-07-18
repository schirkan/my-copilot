//! Session- und Message-Datenstrukturen.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub request_id: String,
    pub role: String,
    pub content: String,
    pub ts: String,
    pub model: String,
    pub tokens: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMeta {
    pub session_id: String,
    pub created_at: String,
    pub updated_at: String,
    pub message_count: u64,
    pub model: String,
    pub title: String,
}

pub fn sessions_dir(exe_dir: &Path) -> PathBuf {
    exe_dir.join("data").join("sessions")
}

pub fn session_path(exe_dir: &Path, session_id: &str) -> PathBuf {
    sessions_dir(exe_dir).join(format!("{}.jsonl", session_id))
}

pub fn now_iso() -> String {
    Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

#[allow(dead_code)]
pub fn parse_iso(s: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(s).ok().map(|d| d.with_timezone(&Utc))
}