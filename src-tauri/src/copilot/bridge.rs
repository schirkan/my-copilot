//! Persistente Tauri-Rust Bridge zum offiziellen `github-copilot-sdk`.
//!
//! Architektur-Update (Streaming-Optimierung, siehe
//! `commands::chat::chat_send`):
//!
//! - **Eine `Client` pro App-Lifetime**: Der SDK-Client (und damit der
//!   CLI-Subprozess) wird beim ersten `chat_send` lazy erzeugt und in
//!   `AppState.bridge` gecached. Folge-Calls reuse den Client -- kein
//!   `Client::start()`/`stop()`-Roundtrip pro Message.
//! - **Eine `Session` pro Chat-Message**: Sessions kapseln den
//!   CLI-State pro Chat. Jede User-Message bekommt eine eigene Session,
//!   damit `subscribe()`, `abort()` und `disconnect()` sauber pro
//!   Request lifecyclebar sind.
//! - **Event-basiertes Streaming**: `Session::subscribe()` liefert
//!   `assistant.message_delta`-Events, die wir pro Chunk via
//!   `app_handle.emit("chat_chunk", ...)` ans Frontend durchreichen.
//!
//! Vorher (v1-non-streaming): pro `chat_send` wurde ein neuer
//! `Client::start()` gemacht, eine `Session` erzeugt, auf
//! `send_and_wait()` gewartet, dann `client.stop()` im Drop. Viel
//! Process-Setup-Overhead pro Message, keine Echtzeit-Chunks.

use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

use github_copilot_sdk::handler::ApproveAllHandler;
use github_copilot_sdk::session::Session;
use github_copilot_sdk::types::{ProviderConfig, SessionConfig};
use github_copilot_sdk::{Client, ClientOptions};

use super::process::ProcessError;
// Pfad-Auflösung der CLI entfällt -- siehe process.rs.

/// BYOK-Konfiguration (geladen aus `config.json`, v1: Klartext).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ByokConfig {
    pub endpoint: String,
    pub api_key: String,
    pub model: String,
    #[serde(default)]
    pub provider_wire_api: Option<String>,
    #[serde(default)]
    pub provider_bearer_token: Option<String>,
    #[serde(default)]
    pub provider_headers: Option<String>,
    #[serde(default)]
    pub provider_model_id: Option<String>,
    #[serde(default)]
    pub provider_wire_model: Option<String>,
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
    pub env: HashMap<String, String>,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

fn default_enabled() -> bool { true }

pub struct CopilotBridge {
    client: Client,
    config: ByokConfig,
}

impl CopilotBridge {
    pub async fn new(_exe_dir: &PathBuf, config: ByokConfig) -> Result<Self, ProcessError> {
        // Mit Feature `bundled-cli` bringt das SDK die CLI selbst mit
        // (build.rs laedt sie aus den GitHub-Releases, verifiziert SHA256,
        // entpackt nach OUT_DIR). Wir uebergeben daher keinen expliziten
        // Program-Pfad -- Client::start nimmt die vom SDK gebundelte Binary.
        let _ = _exe_dir; // Argument bleibt fuer API-Stabilitaet, ist ungenutzt.
        let client = Client::start(ClientOptions::default())
            .await
            .map_err(|e| ProcessError::Sdk(e.to_string()))?;
        Ok(Self { client, config })
    }

    /// Erzeugt eine neue Session fuer einen einzelnen Chat-Request.
    ///
    /// Sessions kapseln den CLI-State (History, Tools, ...) pro Request.
    /// `Session::subscribe()` MUSS vor `Session::send()` aufgerufen
    /// werden, damit alle `assistant.message_delta`-Events ankommen
    /// (siehe `commands::chat::chat_send`).
    ///
    /// **Streaming aktiviert**: `with_streaming(true)` schaltet das
    /// Token-Delta-Routing ein. Ohne dieses Setting senden die meisten
    /// Provider (z. B. MiniMax M3) nur ein einziges `assistant.message`-
    /// Event mit dem vollstaendigen Content statt inkrementeller
    /// `assistant.message_delta`-Events. Mit Streaming werden Tokens
    /// einzeln gerendert (Echtzeit-Typewriter-Effekt im Frontend).
    ///
    /// Die Session selbst wird per `tokio::task::spawn` in einen
    /// Hintergrund-Thread verschoben, der die Event-Loop laeuft. Der
    /// Caller haelt die Session in `AppState.current_session` fuer
    /// spaeteres `abort()` / `disconnect()`.
    pub async fn create_session(&self) -> Result<Session, ProcessError> {
        let session_config = SessionConfig::default()
            .with_permission_handler(Arc::new(ApproveAllHandler))
            .with_model(self.config.model.clone())
            .with_provider(build_provider_config(&self.config))
            // Siehe SDK 1.0.8 SessionConfig::with_streaming: aktiviert
            // `assistant.message_delta` Token-Events. Provider, die das
            // nicht unterstuetzen, senden weiterhin nur ein
            // `assistant.message` (unser Stream-Loop behandelt beide).
            .with_streaming(true);
        self.client
            .create_session(session_config)
            .await
            .map_err(|e| ProcessError::Sdk(e.to_string()))
    }

    /// Read-only Zugriff auf die aktuelle ByokConfig (z. B. fuer
    /// `chat_send` zum Persistieren der Model-Metadaten in der
    /// JSONL-History).
    pub fn config(&self) -> &ByokConfig {
        &self.config
    }
}

impl Drop for CopilotBridge {
    fn drop(&mut self) {
        let client = self.client.clone();
        tauri::async_runtime::spawn(async move {
            let _ = client.stop().await;
        });
    }
}

fn build_provider_config(config: &ByokConfig) -> ProviderConfig {
    // Normalize: Wir entfernen nur genau dann ein `/v1`, wenn der Endpoint
    // bereits mit `/v1/v1` endet (versehentliches Doppel-Suffix durch den
    // User). Ein einzelnes `/v1` bleibt unangetastet -- viele OpenAI-Provider
    // (z. B. MiniMax M3) verlangen `/v1` als Teil ihrer Basis-URL, weil das
    // SDK selbst kein `/v1`-Prefix mehr anhaengt.
    let endpoint = dedupe_v1_suffix(&config.endpoint);

    let mut provider = ProviderConfig::new(endpoint).with_provider_type("openai");
    if let Some(value) = config.provider_bearer_token.as_deref() {
        provider = provider.with_bearer_token(value.to_string());
    } else {
        provider = provider.with_api_key(config.api_key.clone());
    }
    if let Some(value) = config.provider_wire_api.as_deref() {
        provider = provider.with_wire_api(value.to_string());
    }
    if let Some(value) = config.provider_model_id.as_deref() {
        provider = provider.with_model_id(value.to_string());
    }
    if let Some(value) = config.provider_wire_model.as_deref() {
        provider = provider.with_wire_model(value.to_string());
    }
    if let Some(value) = config.provider_headers.as_deref() {
        let headers = value
            .lines()
            .filter_map(|line| line.split_once(':'))
            .map(|(key, val)| (key.trim().to_string(), val.trim().to_string()))
            .filter(|(key, val)| !key.is_empty() && !val.is_empty())
            .collect::<HashMap<_, _>>();
        if !headers.is_empty() {
            provider = provider.with_headers(headers);
        }
    }
    provider
}

/// Reduziert ein versehentliches Doppel-Suffix `/v1/v1` (oder `/v1/v1/`) auf
/// genau ein `/v1`. Ein einzelnes `/v1` (oder keines) bleibt unveraendert,
/// weil viele OpenAI-kompatible Provider `/v1` als Teil ihrer Basis-URL
/// erwarten (z. B. MiniMax M3).
fn dedupe_v1_suffix(endpoint: &str) -> String {
    let trimmed = endpoint.trim_end_matches('/');
    if let Some(stripped) = trimmed.strip_suffix("/v1") {
        // Pruefen, ob das, was VOR dem `/v1` steht, selbst mit `/v1` endet.
        // Beispiele:
        //   "https://api.openai.com/v1/v1" -> strip -> "https://api.openai.com/v1"
        //   "https://api.openai.com/v1"     -> strip -> "https://api.openai.com" (KEIN match)
        //   "https://api.minimax.io/v1"     -> strip -> "https://api.minimax.io" (KEIN match)
        if stripped.to_ascii_lowercase().ends_with("/v1") {
            return stripped.to_string();
        }
    }
    trimmed.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dedupe_v1_handles_double_suffix() {
        assert_eq!(
            dedupe_v1_suffix("https://api.openai.com/v1/v1"),
            "https://api.openai.com/v1"
        );
        assert_eq!(
            dedupe_v1_suffix("https://api.openai.com/v1/v1/"),
            "https://api.openai.com/v1"
        );
    }

    #[test]
    fn dedupe_v1_keeps_single_suffix() {
        // MiniMax M3 erwartet /v1 als Teil der Basis-URL.
        assert_eq!(
            dedupe_v1_suffix("https://api.minimax.io/v1"),
            "https://api.minimax.io/v1"
        );
        // OpenAI direkter Endpoint.
        assert_eq!(
            dedupe_v1_suffix("https://api.openai.com/v1"),
            "https://api.openai.com/v1"
        );
    }

    #[test]
    fn dedupe_v1_keeps_no_suffix() {
        assert_eq!(
            dedupe_v1_suffix("https://api.openai.com"),
            "https://api.openai.com"
        );
        assert_eq!(
            dedupe_v1_suffix("http://localhost:1234"),
            "http://localhost:1234"
        );
    }
}