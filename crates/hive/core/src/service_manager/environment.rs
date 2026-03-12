use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashMap;
use tracing::{debug, trace, warn};

use crate::hive_config::EnvironmentConfig;

#[async_trait]
pub trait EnvPlugin: Send + Sync {
    /// e.g. "static", "dotenv", "vault"
    fn name(&self) -> &str;

    async fn load(&self, config: &serde_json::Value) -> Result<HashMap<String, String>>;
}

pub struct StaticEnvPlugin;

impl Default for StaticEnvPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl StaticEnvPlugin {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl EnvPlugin for StaticEnvPlugin {
    fn name(&self) -> &str {
        "static"
    }

    async fn load(&self, config: &serde_json::Value) -> Result<HashMap<String, String>> {
        let mut env = HashMap::new();

        if let Some(obj) = config.as_object() {
            for (key, value) in obj {
                if let Some(v) = value.as_str() {
                    env.insert(key.clone(), v.to_string());
                }
            }
        }

        Ok(env)
    }
}

pub struct EnvironmentResolver {
    plugins: Vec<Box<dyn EnvPlugin>>,
}

impl Default for EnvironmentResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl EnvironmentResolver {
    pub fn new() -> Self {
        Self {
            plugins: vec![Box::new(StaticEnvPlugin::new())],
        }
    }

    pub fn register(&mut self, plugin: Box<dyn EnvPlugin>) {
        self.plugins.push(plugin);
    }

    pub async fn resolve(&self, config: &EnvironmentConfig) -> Result<HashMap<String, String>> {
        let mut env = HashMap::new();
        debug!(
            providers = config.providers.len(),
            has_static = config.static_env.is_some(),
            "Resolving environment variables"
        );

        if let Some(static_env) = &config.static_env {
            trace!(
                vars = static_env.len(),
                "Loading static environment variables"
            );
            env.extend(static_env.clone());
        }

        for (provider_name, provider_config) in &config.providers {
            // "static" is already handled above
            if provider_name == "static" {
                continue;
            }

            if let Some(plugin) = self.plugins.iter().find(|p| p.name() == provider_name) {
                debug!(provider = %provider_name, "Loading environment from provider");
                let provider_env = plugin.load(provider_config).await?;
                trace!(provider = %provider_name, vars = provider_env.len(), "Provider loaded variables");
                env.extend(provider_env);
            } else {
                warn!(
                    provider = %provider_name,
                    "Unknown environment provider. Install hive.env.{} plugin.",
                    provider_name
                );
            }
        }

        debug!(total_vars = env.len(), "Environment resolution complete");
        Ok(env)
    }
}
