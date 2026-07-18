//! JSONL Chat-History (eine Datei pro Session).
//!
//! Schema pro Zeile: `{id, request_id, role, content, ts, model, tokens}`.
//! Append-only mit tokio fs.
//!
//! Siehe SPEC-004 § Persistenz, DECISIONS.md § Persistenz-Format.

pub mod append;
pub mod read;
pub mod session;

pub use append::append_message;
pub use read::{delete_session, list_sessions, load_session};
pub use session::{now_iso, session_path, sessions_dir, Message, SessionMeta};