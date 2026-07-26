use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use github_copilot_sdk::handler::ApproveAllHandler;
use github_copilot_sdk::types::{ProviderConfig, SessionConfig};
use github_copilot_sdk::{CliProgram, Client, ClientOptions, MessageOptions};

use super::process::ProcessError;

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
    pub async fn new(exe_dir: &PathBuf, config: ByokConfig) -> Result<Self, ProcessError> {
        let binary_path = super::process::resolve_copilot_binary_path(exe_dir)?;
        let mut options = ClientOptions::default();
        options.program = CliProgram::Path(binary_path);
        let client = Client::start(options)
        .await
        .map_err(|e| ProcessError::Sdk(e.to_string()))?;
        Ok(Self { client, config })
    }

    pub async fn chat_once(&self, user_message: String) -> Result<String, ProcessError> {
        let session_config = SessionConfig::default()
            .with_handler(Arc::new(ApproveAllHandler))
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
    let mut provider = ProviderConfig::new(config.endpoint.clone()).with_provider_type("openai");
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