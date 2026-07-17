//! Tauri-Commands (Frontend → Backend IPC).
//!
//! Registriert in src-tauri/src/lib.rs via `tauri::generate_handler![]`.
//! Siehe SPEC-004 § IPC-Methoden für die vollständige API-Definition.

pub mod chat;
pub mod config;
pub mod process;