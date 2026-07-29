use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use github_copilot_sdk::handler::ApproveAllHandler;
use github_copilot_sdk::types::{ProviderConfig, SessionConfig};
use github_copilot_sdk::{Client, ClientOptions, MessageOptions};

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

    pub async fn chat_once(&self, user_message: String) -> Result<String, ProcessError> {
        let session_config = SessionConfig::default()
            .with_permission_handler(Arc::new(ApproveAllHandler))
            .with_model(self.config.model.clone())
            .with_provider(build_provider_config(&self.config));
        let session = self.client.create_session(session_config).await.map_err(|e| ProcessError::Sdk(e.to_string()))?;
        let response = session
            .send_and_wait(MessageOptions::new(user_message).with_wait_timeout(Duration::from_secs(60)))
            .await
            .map_err(|e| ProcessError::Sdk(e.to_string()))?;
        let _ = session.disconnect().await;
        Ok(response
            .and_then(|event| event.data.get("content").and_then(|v| v.as_str()).map(str::to_string))
            .unwrap_or_default())
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
    // Normalize: User koennen die URL mit oder ohne `/v1`-Suffix eingeben.
    // Das SDK haengt `/v1` fuer `wire_api = "completions"` (Default) selbst an.
    // Wir strippen trailing `/v1` (mit oder ohne Slash), damit weder ein
    // doppeltes `/v1/v1/...` noch ein abgeschnittenes `/v1/models` entsteht.
    let endpoint = strip_v1_suffix(&config.endpoint);

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

/// Entfernt ein angehaengtes `/v1` (mit oder ohne abschliessenden Slash) vom
/// Endpoint, damit er sowohl mit `https://api.openai.com/v1` als auch mit
/// `https://api.openai.com/v1/` als bare base URL verwendbar ist.
fn strip_v1_suffix(endpoint: &str) -> String {
    let trimmed = endpoint.trim_end_matches('/');
    match trimmed.strip_suffix("/v1") {
        Some(base) => base.to_string(),
        None => trimmed.to_string(),
    }
}