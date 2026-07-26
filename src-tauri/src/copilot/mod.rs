//! Tauri-Rust Bridge zur GitHub Copilot SDK Runtime.
//!
//! Diese Modulgruppe kapselt die lokale BYOK-Konfiguration und den
//! offiziellen Rust-SDK-Zugriff auf die Copilot-CLI-Runtime.

pub mod bridge;
pub mod process;

pub use bridge::{ByokConfig, CopilotBridge, McpServer};
pub use process::ProcessError;