//! 1Password Environment Plugin for Hive
//!
//! Loads environment variables from 1Password via the `op` CLI.
//!
//! ## Configuration
//!
//! ```yaml
//! environment:
//!   1password:
//!     vault: Development
//!     item: api-secrets
//!     fields:
//!       - api_key
//!       - db_password
//! ```

use lib_plugin_abi_v3::{
    async_trait, env::EnvProvider, Plugin, PluginCategory, PluginContext, PluginMetadata,
    PluginType, Result as PluginResult, SERVICE_ENV_PROVIDER,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::process::Stdio;
use tokio::process::Command;
use tracing::{debug, warn};

pub struct OnePasswordProvider;

impl Default for OnePasswordProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl OnePasswordProvider {
    pub fn new() -> Self {
        Self
    }

    async fn op_command(&self, args: &[&str]) -> Result<String, lib_plugin_abi_v3::PluginError> {
        let output = Command::new("op")
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| {
                lib_plugin_abi_v3::PluginError::Internal(format!(
                    "Failed to execute 'op' command - is 1Password CLI installed? {}",
                    e
                ))
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(lib_plugin_abi_v3::PluginError::Internal(format!(
                "1Password CLI error: {}",
                stderr
            )));
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    async fn get_field(
        &self,
        vault: Option<&str>,
        item: &str,
        field: &str,
    ) -> Result<String, lib_plugin_abi_v3::PluginError> {
        let reference = if let Some(v) = vault {
            format!("op://{}/{}/{}", v, item, field)
        } else {
            format!("op://{}/{}", item, field)
        };

        let args = vec!["read", &reference];

        debug!("Reading 1Password field: {}", reference);
        self.op_command(&args).await.map(|s| s.trim().to_string())
    }

    async fn get_item(
        &self,
        vault: Option<&str>,
        item: &str,
    ) -> Result<serde_json::Value, lib_plugin_abi_v3::PluginError> {
        let mut args = vec!["item", "get", item, "--format", "json"];

        if let Some(v) = vault {
            args.push("--vault");
            args.push(v);
        }

        debug!("Reading 1Password item: {}", item);
        let output = self.op_command(&args).await?;
        serde_json::from_str(&output).map_err(|e| {
            lib_plugin_abi_v3::PluginError::Internal(format!(
                "Failed to parse 1Password item JSON: {}",
                e
            ))
        })
    }
}

#[async_trait]
impl Plugin for OnePasswordProvider {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            id: "hive.env.1password".to_string(),
            name: "1Password Secrets".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            plugin_type: PluginType::Orchestration,
            author: Some("ADI Team".to_string()),
            description: Some("Load secrets from 1Password".to_string()),
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
impl EnvProvider for OnePasswordProvider {
    async fn load(&self, config: &Value) -> PluginResult<HashMap<String, String>> {
        let op_config: OnePasswordConfig =
            serde_json::from_value(config.clone()).map_err(|e| {
                lib_plugin_abi_v3::PluginError::Config(format!(
                    "Failed to parse 1Password configuration: {}",
                    e
                ))
            })?;

        let mut env = HashMap::new();
        let vault = op_config.vault.as_deref();

        if let Some(fields) = &op_config.fields {
            for field in fields {
                match self.get_field(vault, &op_config.item, field).await {
                    Ok(value) => {
                        let env_key = if op_config.uppercase.unwrap_or(true) {
                            field.to_uppercase().replace('-', "_")
                        } else {
                            field.clone()
                        };
                        env.insert(env_key, value);
                    }
                    Err(e) => {
                        warn!("Failed to get 1Password field '{}': {}", field, e);
                    }
                }
            }
        } else if let Some(mappings) = &op_config.keys {
            for (op_field, env_key) in mappings {
                match self.get_field(vault, &op_config.item, op_field).await {
                    Ok(value) => {
                        env.insert(env_key.clone(), value);
                    }
                    Err(e) => {
                        warn!("Failed to get 1Password field '{}': {}", op_field, e);
                    }
                }
            }
        } else {
            let item = self.get_item(vault, &op_config.item).await?;

            if let Some(fields) = item.get("fields").and_then(|f| f.as_array()) {
                for field in fields {
                    if let (Some(label), Some(value)) = (
                        field.get("label").and_then(|l| l.as_str()),
                        field.get("value").and_then(|v| v.as_str()),
                    ) {
                        // Skip concealed fields that are empty
                        if value.is_empty() {
                            continue;
                        }

                        let env_key = if op_config.uppercase.unwrap_or(true) {
                            label.to_uppercase().replace('-', "_").replace(' ', "_")
                        } else {
                            label.to_string()
                        };
                        env.insert(env_key, value.to_string());
                    }
                }
            }
        }

        Ok(env)
    }

    async fn refresh(&self, config: &Value) -> PluginResult<HashMap<String, String>> {
        self.load(config).await
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnePasswordConfig {
    /// Vault name (optional if item is unique across all vaults)
    pub vault: Option<String>,
    pub item: String,
    /// Specific fields to fetch; if absent, fetches all fields
    pub fields: Option<Vec<String>>,
    /// Key mappings (1password_field -> env_key)
    pub keys: Option<HashMap<String, String>>,
    /// Convert keys to uppercase (default: true)
    pub uppercase: Option<bool>,
}

#[no_mangle]
pub fn plugin_create() -> Box<dyn Plugin> {
    Box::new(OnePasswordProvider::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metadata() {
        let plugin = OnePasswordProvider::new();
        let meta = plugin.metadata();
        assert_eq!(meta.id, "hive.env.1password");
        assert_eq!(meta.name, "1Password Secrets");
    }

    #[test]
    fn test_config_parse() {
        let config = serde_json::json!({
            "vault": "Development",
            "item": "api-secrets",
            "fields": ["api_key", "db_password"]
        });

        let op_config: OnePasswordConfig = serde_json::from_value(config).unwrap();
        assert_eq!(op_config.vault, Some("Development".to_string()));
        assert_eq!(op_config.item, "api-secrets");
        assert_eq!(op_config.fields.unwrap().len(), 2);
    }
}
