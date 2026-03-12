//! AWS Secrets Manager Environment Plugin for Hive
//!
//! Loads environment variables from AWS Secrets Manager.
//!
//! ## Configuration
//!
//! ```yaml
//! environment:
//!   aws-secrets:
//!     secret_id: adi/production/db
//!     region: us-east-1
//! ```

use aws_sdk_secretsmanager::Client;
use lib_plugin_abi_v3::{
    async_trait, env::EnvProvider, Plugin, PluginCategory, PluginContext, PluginMetadata,
    PluginType, Result as PluginResult, SERVICE_ENV_PROVIDER,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::debug;

pub struct AwsSecretsProvider {
    /// Cached AWS client (lazily initialized on first load)
    cached_client: tokio::sync::OnceCell<(Option<String>, Client)>,
}

impl Default for AwsSecretsProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl AwsSecretsProvider {
    pub fn new() -> Self {
        Self {
            cached_client: tokio::sync::OnceCell::new(),
        }
    }

    /// Cached after first creation to avoid repeated credential chain resolution.
    async fn get_client(&self, region: Option<&str>) -> PluginResult<&Client> {
        let (cached_region, client) = self
            .cached_client
            .get_or_init(|| async {
                let config = if let Some(region) = region {
                    aws_config::defaults(aws_config::BehaviorVersion::latest())
                        .region(aws_config::Region::new(region.to_string()))
                        .load()
                        .await
                } else {
                    aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await
                };

                (region.map(|s| s.to_string()), Client::new(&config))
            })
            .await;

        // Check if region changed - if so, we need to recreate client
        // For simplicity, log a warning but use cached client
        if cached_region.as_deref() != region {
            tracing::warn!(
                "AWS region changed from {:?} to {:?} - using cached client. \
                 Restart service to use new region.",
                cached_region,
                region
            );
        }

        Ok(client)
    }
}

#[async_trait]
impl Plugin for AwsSecretsProvider {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            id: "hive.env.aws-secrets".to_string(),
            name: "AWS Secrets Manager".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            plugin_type: PluginType::Orchestration,
            author: Some("ADI Team".to_string()),
            description: Some("Load secrets from AWS Secrets Manager".to_string()),
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
impl EnvProvider for AwsSecretsProvider {
    async fn load(&self, config: &serde_json::Value) -> PluginResult<HashMap<String, String>> {
        let aws_config: AwsSecretsConfig = serde_json::from_value(config.clone())
            .map_err(|e| lib_plugin_abi_v3::PluginError::Config(format!("Failed to parse AWS Secrets Manager configuration: {}", e)))?;

        let client = self.get_client(aws_config.region.as_deref()).await?;

        debug!("Fetching secret: {}", aws_config.secret_id);

        let mut request = client
            .get_secret_value()
            .secret_id(&aws_config.secret_id);

        if let Some(version_id) = &aws_config.version_id {
            request = request.version_id(version_id);
        }
        if let Some(version_stage) = &aws_config.version_stage {
            request = request.version_stage(version_stage);
        }

        let response = request
            .send()
            .await
            .map_err(|e| lib_plugin_abi_v3::PluginError::Runtime(format!("Failed to fetch secret from AWS Secrets Manager: {}", e)))?;

        let secret_string = response
            .secret_string()
            .ok_or_else(|| lib_plugin_abi_v3::PluginError::Runtime("Secret has no string value (binary secrets not supported)".to_string()))?;

        let secrets: HashMap<String, serde_json::Value> = serde_json::from_str(secret_string)
            .map_err(|e| lib_plugin_abi_v3::PluginError::Runtime(format!("Failed to parse secret as JSON: {}", e)))?;

        let mut env = HashMap::new();

        if let Some(mappings) = &aws_config.keys {
            for (secret_key, env_key) in mappings {
                if let Some(value) = secrets.get(secret_key) {
                    let value_str = match value {
                        serde_json::Value::String(s) => s.clone(),
                        _ => value.to_string(),
                    };
                    env.insert(env_key.clone(), value_str);
                }
            }
        } else {
            let prefix = aws_config.prefix.as_deref().unwrap_or("");
            for (key, value) in secrets {
                let env_key = if aws_config.uppercase.unwrap_or(true) {
                    format!("{}{}", prefix, key.to_uppercase().replace('-', "_"))
                } else {
                    format!("{}{}", prefix, key)
                };

                let value_str = match value {
                    serde_json::Value::String(s) => s,
                    _ => value.to_string(),
                };
                env.insert(env_key, value_str);
            }
        }

        Ok(env)
    }

    async fn refresh(&self, config: &serde_json::Value) -> PluginResult<HashMap<String, String>> {
        self.load(config).await
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AwsSecretsConfig {
    pub secret_id: String,
    /// AWS region; uses default credential chain region if absent
    pub region: Option<String>,
    /// Key mappings (secret_key -> env_key)
    pub keys: Option<HashMap<String, String>>,
    /// Prefix for environment variable names
    pub prefix: Option<String>,
    /// Convert keys to uppercase (default: true)
    pub uppercase: Option<bool>,
    pub version_id: Option<String>,
    /// Defaults to AWSCURRENT when absent
    pub version_stage: Option<String>,
}

#[no_mangle]
pub fn plugin_create() -> Box<dyn Plugin> {
    Box::new(AwsSecretsProvider::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metadata() {
        let plugin = AwsSecretsProvider::new();
        let meta = plugin.metadata();
        assert_eq!(meta.id, "hive.env.aws-secrets");
        assert_eq!(meta.name, "AWS Secrets Manager");
    }

    #[test]
    fn test_config_parse() {
        let config = serde_json::json!({
            "secret_id": "adi/production/db",
            "region": "us-east-1",
            "prefix": "DB_"
        });

        let aws_config: AwsSecretsConfig = serde_json::from_value(config).unwrap();
        assert_eq!(aws_config.secret_id, "adi/production/db");
        assert_eq!(aws_config.region, Some("us-east-1".to_string()));
    }

    #[test]
    fn test_provides_service() {
        let plugin = AwsSecretsProvider::new();
        assert!(plugin.provides().contains(&SERVICE_ENV_PROVIDER));
    }
}
