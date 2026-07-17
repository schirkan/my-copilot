//! Tauri-Rust Bridge zur Copilot CLI.
//!
//! Diese Modulgruppe kapselt den gesamten Lebenszyklus und die
//! JSON-RPC-Kommunikation mit der Copilot CLI (Node.js-Subprozess).
//! Siehe SPEC-004 für die Architektur und SPEC-005 für das Frontend.

pub mod bridge;
pub mod process;

pub use bridge::{spawn_bridge, ByokConfig, ChatChunk, CopilotBridge, JsonRpcMessage, McpServer};
pub use process::{CopilotCliProcess, ProcessError};