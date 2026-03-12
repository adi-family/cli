//! HashiCorp Vault Environment Plugin for Hive
//!
//! Loads environment variables from HashiCorp Vault secrets.
//!
//! ## Configuration
//!
//! ```yaml
//! environment:
//!   vault:
//!     address: https://vault.example.com
//!     path: secret/data/adi/auth
//!     token: ${env.VAULT_TOKEN}
//! ```

use anyhow::{anyhow, Context};
use lib_plugin_abi_v3::{
    async_trait, env::EnvProvider, Plugin, PluginCategory, PluginContext, PluginMetadata,
    PluginType, Result as PluginResult, SERVICE_ENV_PROVIDER,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use tracing::{debug, warn};

use lib_env_parse::{env_require, env_vars};

env_vars! {
    VaultAddr => "VAULT_ADDR",
    VaultToken => "VAULT_TOKEN",
}

pub struct VaultPlugin {
    client: reqwest::Client,
}

impl Default for VaultPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl VaultPlugin {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl Plugin for VaultPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            id: "hive.env.vault".to_string(),
            name: "HashiCorp Vault".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            plugin_type: PluginType::Orchestration,
            author: Some("ADI Team".to_string()),
            description: Some("Load secrets from HashiCorp Vault".to_string()),
            category: Some(PluginCategory::Env),
        }
    }

    async fn init(&mut self, _ctx: &PluginContext) -> PluginResult<()> {
        Ok(())
    }

    async fn shutdown(&self) -> PluginResult<()> {
        Ok(())
    }

    fn provides(&self) -> Vec<&'static str> {
        vec![SERVICE_ENV_PROVIDER]
    }
}

#[async_trait]
impl EnvProvider for VaultPlugin {
    async fn load(&self, config: &Value) -> PluginResult<HashMap<String, String>> {
        let vault_config: VaultConfig = serde_json::from_value(config.clone())
            .context("Failed to parse Vault configuration")?;

        let address = vault_config
            .address
            .map(Ok)
            .unwrap_or_else(|| env_require(EnvVar::VaultAddr.as_str()).map_err(|e| anyhow!("{e}")))?;

        let token = vault_config
            .token
            .map(Ok)
            .unwrap_or_else(|| env_require(EnvVar::VaultToken.as_str()).map_err(|e| anyhow!("{e}")))?;

        let url = format!("{}/v1/{}", address.trim_end_matches('/'), vault_config.path);

        debug!("Fetching secrets from Vault: {}", url);

        let response = self
            .client
            .get(&url)
            .header("X-Vault-Token", &token)
            .send()
            .await
            .context("Failed to connect to Vault")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow!("Vault request failed ({}): {}", status, body).into());
        }

        let vault_response: VaultResponse = response
            .json()
            .await
            .context("Failed to parse Vault response")?;

        // Extract data (Vault KV v2 has data nested under data.data)
        let secrets = if let Some(data) = vault_response.data.data {
            data
        } else {
            // For KV v1, data is directly under data
            vault_response
                .data
                .raw
                .unwrap_or_default()
                .into_iter()
                .filter_map(|(k, v)| v.as_str().map(|s| (k, s.to_string())))
                .collect()
        };

        let mut env = HashMap::new();
        if let Some(mappings) = vault_config.keys {
            for (vault_key, env_key) in mappings {
                if let Some(value) = secrets.get(&vault_key) {
                    env.insert(env_key, value.clone());
                } else {
                    warn!("Vault key '{}' not found in secret", vault_key);
                }
            }
        } else {
            for (key, value) in secrets {
                let env_key = if vault_config.uppercase.unwrap_or(true) {
                    key.to_uppercase()
                } else {
                    key
                };
                env.insert(env_key, value);
            }
        }

        Ok(env)
    }

    async fn refresh(&self, config: &Value) -> PluginResult<HashMap<String, String>> {
        self.load(config).await
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultConfig {
    /// Vault server address; falls back to VAULT_ADDR env var
    pub address: Option<String>,
    /// Secret path (e.g., "secret/data/adi/auth")
    pub path: String,
    /// Vault token; falls back to VAULT_TOKEN env var
    pub token: Option<String>,
    /// Key mappings (vault_key -> env_key)
    pub keys: Option<HashMap<String, String>>,
    /// Convert keys to uppercase (default: true)
    pub uppercase: Option<bool>,
    /// Namespace (for Vault Enterprise)
    pub namespace: Option<String>,
}

#[derive(Debug, Deserialize)]
struct VaultResponse {
    data: VaultData,
}

#[derive(Debug, Deserialize)]
struct VaultData {
    /// KV v2: nested under data.data
    data: Option<HashMap<String, String>>,
    /// KV v1: directly under data
    #[serde(flatten)]
    raw: Option<HashMap<String, serde_json::Value>>,
}

#[no_mangle]
pub fn plugin_create() -> Box<dyn Plugin> {
    Box::new(VaultPlugin::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metadata() {
        let plugin = VaultPlugin::new();
        let meta = plugin.metadata();
        assert_eq!(meta.id, "hive.env.vault");
        assert_eq!(meta.name, "HashiCorp Vault");
    }

    #[test]
    fn test_config_parse() {
        let config = serde_json::json!({
            "address": "https://vault.example.com",
            "path": "secret/data/test",
            "token": "test-token"
        });

        let vault_config: VaultConfig = serde_json::from_value(config).unwrap();
        assert_eq!(
            vault_config.address,
            Some("https://vault.example.com".to_string())
        );
        assert_eq!(vault_config.path, "secret/data/test");
    }

    #[test]
    fn test_provides() {
        let plugin = VaultPlugin::new();
        let services = plugin.provides();
        assert!(services.contains(&SERVICE_ENV_PROVIDER));
    }
}
