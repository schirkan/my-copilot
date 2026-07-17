//! JSON-RPC-Bridge zur Copilot CLI.
//!
//! Verschickt Requests via Stdin (eine JSON-RPC-Nachricht pro Zeile)
//! und konsumiert Responses/Notifications via Stdout.
//!
//! Streaming-Pattern:
//! - `chat_streaming()` sendet einen `chat`-Request (mit id)
//! - CLI antwortet mit `result` (mit derselben id) — wird stillschweigend
//!   konsumiert (Acknowledge)
//! - CLI sendet parallel `chat.chunk`-Notifications (ohne id) — diese
//!   werden als Stream-Chunks zurückgegeben
//!
//! Siehe SPEC-004 § IPC-Architektur und § Copilot SDK Rust.

use std::path::Path;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::ChildStdout;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tokio_stream::Stream;

use super::process::{CopilotCliProcess, ProcessError};

// -----------------------------------------------------------------------------
// Config-Typen (Platzhalter: vollständige Persistenz folgt in Card #4)
// -----------------------------------------------------------------------------

/// BYOK-Konfiguration (wird in Card #4 aus config.json + DPAPI geladen).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ByokConfig {
    pub endpoint: String,
    pub api_key: String,
    pub model: String,
    #[serde(default)]
    pub system_prompt: Option<String>,
    #[serde(default)]
    pub mcp_servers: Vec<McpServer>,
}

/// MCP-Server-Konfiguration (siehe SPEC-006 § 6).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServer {
    pub name: String,
    pub transport: String,
    #[serde(default)]
    pub command: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub env: std::collections::HashMap<String, String>,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

fn default_enabled() -> bool { true }

// -----------------------------------------------------------------------------
// JSON-RPC-Envelope
// -----------------------------------------------------------------------------

/// Generisches JSON-RPC-2.0-Envelope (Request, Response oder Notification).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcMessage {
    pub jsonrpc: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub method: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<Value>,
}

/// Stream-Chunk: Text-Snippet aus dem LLM-Response.
#[derive(Debug, Clone)]
pub struct ChatChunk {
    pub text: String,
}

// -----------------------------------------------------------------------------
// Bridge
// -----------------------------------------------------------------------------

/// High-Level-Wrapper um den CLI-Subprozess für Chat-Operationen.
///
/// Wird vom Tauri-IPC-Layer (Card #3) instanziiert, sobald die App
/// eine Config geladen hat und der User eine Chat-Message abschickt.
pub struct CopilotBridge {
    pub(crate) process: CopilotCliProcess,
    pub(crate) config: ByokConfig,
    next_id: u64,
}

impl CopilotBridge {
    pub fn new(process: CopilotCliProcess, config: ByokConfig) -> Self {
        Self {
            process,
            config,
            next_id: 0,
        }
    }

    fn next_id(&mut self) -> u64 {
        self.next_id += 1;
        self.next_id
    }

    /// Sendet einen `chat`-Request und gibt einen Stream von [`ChatChunk`]s
    /// zurück. Die eigentlichen Tokens kommen als JSON-RPC-Notifications
    /// (kein `id`) auf stdout.
    ///
    /// Erfordert `&mut self`, weil `stdin` und `stdout` exklusiv genutzt
    /// werden.
    pub async fn chat_streaming(
        &mut self,
        user_message: String,
    ) -> Result<impl Stream<Item = ChatChunk>, ProcessError> {
        let request_id = self.next_id();

        let request = JsonRpcMessage {
            jsonrpc: "2.0".into(),
            id: Some(request_id),
            method: Some("chat".into()),
            params: Some(serde_json::json!({
                "message": user_message,
                "model": self.config.model,
                "api_key": self.config.api_key,
                "endpoint": self.config.endpoint,
                "system_prompt": self.config.system_prompt,
                "mcp_servers": self.config.mcp_servers,
            })),
            result: None,
            error: None,
        };

        let request_str = serde_json::to_string(&request)
            .map_err(|e| ProcessError::StdinWrite(format!("serialize: {}", e)))?;

        self.process.stdin.write_all(request_str.as_bytes()).await
            .map_err(|e| ProcessError::StdinWrite(format!("write: {}", e)))?;
        self.process.stdin.write_all(b"\n").await
            .map_err(|e| ProcessError::StdinWrite(format!("newline: {}", e)))?;
        self.process.stdin.flush().await
            .map_err(|e| ProcessError::StdinWrite(format!("flush: {}", e)))?;

        // stdout nach außen geben — ab hier exklusiv für den Stream.
        let stdout = self.process.stdout.take()
            .ok_or_else(|| ProcessError::StdoutTake("stdout already taken".into()))?;

        Ok(parse_jsonrpc_stream(BufReader::new(stdout)))
    }

    pub fn config(&self) -> &ByokConfig { &self.config }
    pub fn process_mut(&mut self) -> &mut CopilotCliProcess { &mut self.process }
    pub fn process(&self) -> &CopilotCliProcess { &self.process }
}

/// Parsed JSON-RPC-Notifications als ChatChunk-Stream.
///
/// Implementiert via mpsc-Channel + Background-Task (statt
/// `tokio_stream::wrappers::LinesStream`, das in neueren Versionen
/// entfernt wurde). Die Background-Task liest zeilenweise von
/// stdout, parst jede Zeile, und sendet extrahierte Chunks an den
/// Channel. Der Stream bricht ab, wenn stdout EOF erreicht oder
/// der Receiver gedroppt wird.
///
/// - Notifications (kein `id`) → ChatChunk (extrahiert text/content/delta)
/// - Responses (mit `id`) → stillschweigend konsumiert (Acknowledge)
/// - Fehler-Responses (mit `error`) → loggen, Chunk wird übersprungen
fn parse_jsonrpc_stream(
    reader: BufReader<ChildStdout>,
) -> impl Stream<Item = ChatChunk> {
    let (tx, rx) = mpsc::channel::<ChatChunk>(64);

    tokio::spawn(async move {
        let mut lines = reader.lines();
        while let Ok(Some(line)) = lines.next_line().await {
            if let Some(chunk) = parse_line_to_chunk(&line) {
                if tx.send(chunk).await.is_err() {
                    break; // Receiver gedroppt → Stream beendet
                }
            }
        }
    });

    ReceiverStream::new(rx)
}

/// Parst eine einzelne stdout-Zeile zu einem optionalen [`ChatChunk`].
/// None für Responses/Acks/Fehler (siehe oben).
fn parse_line_to_chunk(line: &str) -> Option<ChatChunk> {
    let msg: JsonRpcMessage = serde_json::from_str(line).ok()?;
    if let Some(err) = &msg.error {
        log::error!("JSON-RPC-Fehler von CLI: {}", err);
        return None;
    }
    if msg.id.is_some() {
        return None; // Response-Ack, ignorieren
    }
    let params = msg.params?;
    let text = params.get("text")
        .or_else(|| params.get("content"))
        .or_else(|| params.get("delta"))
        .and_then(|v| v.as_str())?;
    Some(ChatChunk { text: text.to_string() })
}

/// Convenience: spawnt CLI und erzeugt direkt eine Bridge.
///
/// Wird vom Tauri-Setup-Hook (lib.rs) aufgerufen, sobald die App
/// weiß, wo ihr exe-Verzeichnis liegt.
pub async fn spawn_bridge(
    exe_dir: &Path,
    config: ByokConfig,
) -> Result<CopilotBridge, ProcessError> {
    let process = CopilotCliProcess::start(exe_dir)?;
    let mut bridge = CopilotBridge::new(process, config);
    bridge.process_mut().wait_for_ready(Duration::from_secs(10)).await?;
    Ok(bridge)
}