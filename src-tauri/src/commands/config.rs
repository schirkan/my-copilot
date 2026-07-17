//! `config_get` + `config_set` + `config_test`-Commands.

use std::time::Duration;

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::copilot::{ByokConfig, McpServer};
use crate::state::AppState;

/// DTO für die Config-IPC — separater Typ statt direkt `ByokConfig`,
/// damit das Frontend nicht die internen Felder (z. B. zukünftiges
/// `apiKeyCipher`-Feld) sehen kann.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ConfigDto {
    pub endpoint: String,
    pub api_key: String,
    pub model: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub system_prompt: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub mcp_servers: Vec<McpServer>,
}

impl From<&ByokConfig> for ConfigDto {
    fn from(c: &ByokConfig) -> Self {
        Self {
            endpoint: c.endpoint.clone(),
            api_key: c.api_key.clone(),
            model: c.model.clone(),
            system_prompt: c.system_prompt.clone(),
            mcp_servers: c.mcp_servers.clone(),
        }
    }
}

impl From<ConfigDto> for ByokConfig {
    fn from(d: ConfigDto) -> Self {
        Self {
            endpoint: d.endpoint,
            api_key: d.api_key,
            model: d.model,
            system_prompt: d.system_prompt,
            mcp_servers: d.mcp_servers,
        }
    }
}

#[tauri::command]
pub async fn config_get(state: State<'_, AppState>) -> Result<ConfigDto, String> {
    let config = state.config.lock().await;
    config
        .as_ref()
        .map(ConfigDto::from)
        .ok_or_else(|| "no config — please configure first".to_string())
}

#[tauri::command]
pub async fn config_set(
    config: ConfigDto,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let byok: ByokConfig = config.into();
    *state.config.lock().await = Some(byok);
    log::info!("Config gesetzt (apiKey-Länge: {} Zeichen)", state
        .config
        .lock()
        .await
        .as_ref()
        .map(|c| c.api_key.len())
        .unwrap_or(0));
    Ok(())
}

#[derive(Serialize, Clone, Debug)]
pub struct ConfigTestResult {
    pub ok: bool,
    pub models: Vec<String>,
    pub error: Option<String>,
}

/// Testet einen Endpoint durch GET `{endpoint}/v1/models` mit
/// Bearer-Auth. v1: simples 200-OK-Check + Model-Liste parsen.
#[tauri::command]
pub async fn config_test(
    endpoint: String,
    api_key: String,
) -> Result<ConfigTestResult, String> {
    let url = if endpoint.ends_with('/') {
        format!("{}v1/models", endpoint)
    } else {
        format!("{}/v1/models", endpoint)
    };

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| format!("client build: {}", e))?;

    let resp = match client.get(&url).bearer_auth(&api_key).send().await {
        Ok(r) => r,
        Err(e) => {
            return Ok(ConfigTestResult {
                ok: false,
                models: vec![],
                error: Some(format!("request: {}", e)),
            });
        }
    };

    if !resp.status().is_success() {
        return Ok(ConfigTestResult {
            ok: false,
            models: vec![],
            error: Some(format!("HTTP {}", resp.status())),
        });
    }

    let body: serde_json::Value = match resp.json().await {
        Ok(b) => b,
        Err(e) => {
            return Ok(ConfigTestResult {
                ok: false,
                models: vec![],
                error: Some(format!("parse: {}", e)),
            });
        }
    };

    let models: Vec<String> = body
        .get("data")
        .and_then(|d| d.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|m| m.get("id").and_then(|v| v.as_str()).map(String::from))
                .collect()
        })
        .unwrap_or_default();

    Ok(ConfigTestResult {
        ok: true,
        models,
        error: None,
    })
}