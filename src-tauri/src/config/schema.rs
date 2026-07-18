//! Config-Schema + Konvertierungen zu/von ByokConfig.
//!
//! v1: `apiKey` wird im Klartext in `config.json` gespeichert.
//! v2-TODO: DPAPI/OS-Keychain-Verschlüsselung (siehe DECISIONS.md
//! § Config-Storage-v1-plaintext).

use serde::{Deserialize, Serialize};

use crate::copilot::{ByokConfig, McpServer};

/// Aktuelle Schema-Version. Bei Inkompatibilität → `ConfigError::Schema`.
pub const CURRENT_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub version: u32,
    pub endpoint: EndpointConfig,
    pub model: ModelConfig,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub system_prompt: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub mcp_servers: Vec<McpServer>,
    #[serde(default = "default_log_level")]
    pub log_level: String,
    #[serde(default)]
    pub telemetry: bool,
}

fn default_log_level() -> String {
    "info".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndpointConfig {
    /// `azure-openai` | `openai` | `openai-compatible`
    pub r#type: String,
    pub base_url: String,
    /// v1: Klartext in `config.json`. v2-TODO: DPAPI/OS-Keychain-
    /// Verschlüsselung (siehe DECISIONS.md § Config-Storage-v1-plaintext).
    pub api_key: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deployment_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    pub default: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fallback: Option<String>,
}

#[derive(Debug)]
pub enum ConfigError {
    Io(String),
    Json(String),
    /// v1: nicht in Verwendung (apiKey ist Klartext). v2-TODO für DPAPI.
    Schema(String),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "I/O: {}", e),
            Self::Json(e) => write!(f, "JSON: {}", e),
            Self::Schema(e) => write!(f, "Schema: {}", e),
        }
    }
}

impl std::error::Error for ConfigError {}

impl From<std::io::Error> for ConfigError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e.to_string())
    }
}

impl From<serde_json::Error> for ConfigError {
    fn from(e: serde_json::Error) -> Self {
        Self::Json(e.to_string())
    }
}

impl Config {
    /// Erstellt eine persistable Config aus einer in-memory ByokConfig.
    /// v1: Klartext-Pass-through.
    pub fn from_byok_config(byok: &ByokConfig) -> Result<Self, ConfigError> {
        Ok(Self {
            version: CURRENT_VERSION,
            endpoint: EndpointConfig {
                r#type: "openai-compatible".to_string(),
                base_url: byok.endpoint.clone(),
                api_key: byok.api_key.clone(),
                deployment_name: None,
            },
            model: ModelConfig {
                default: byok.model.clone(),
                fallback: None,
            },
            system_prompt: byok.system_prompt.clone(),
            mcp_servers: byok.mcp_servers.clone(),
            log_level: "info".to_string(),
            telemetry: false,
        })
    }

    /// Konvertiert zu ByokConfig. v1: Klartext-Pass-through.
    pub fn to_byok_config(&self) -> Result<ByokConfig, ConfigError> {
        if self.version != CURRENT_VERSION {
            return Err(ConfigError::Schema(format!(
                "unsupported config version: {} (expected {})",
                self.version, CURRENT_VERSION
            )));
        }
        Ok(ByokConfig {
            endpoint: self.endpoint.base_url.clone(),
            api_key: self.endpoint.api_key.clone(),
            model: self.model.default.clone(),
            system_prompt: self.system_prompt.clone(),
            mcp_servers: self.mcp_servers.clone(),
        })
    }
}