//! Plugin Loader
//!
//! Provides functionality to load plugins from dynamic libraries.
//! This module is used by hive-core to load external plugins.

use crate::{
    env::EnvPlugin,
    health::HealthPlugin,
    obs::ObsPlugin,
    proxy::ProxyPlugin,
    runner::RunnerPlugin,
    PluginCategory, PluginMetadata,
};
use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Plugin status
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PluginStatus {
    /// Plugin is built-in and always available
    BuiltIn,
    /// Plugin is installed and loaded
    Loaded,
    /// Plugin is available but not loaded
    Available,
    /// Plugin is not installed
    NotInstalled,
    /// Plugin loading failed
    Failed(String),
}

/// Plugin info for registry
#[derive(Debug, Clone)]
pub struct PluginInfo {
    pub metadata: PluginMetadata,
    pub status: PluginStatus,
    pub path: Option<PathBuf>,
}

/// Plugin registry manages available plugins
pub struct PluginLoader {
    /// Registered plugins by ID
    plugins: Arc<RwLock<HashMap<String, PluginInfo>>>,
    /// Loaded runner plugins
    runners: Arc<RwLock<HashMap<String, Box<dyn RunnerPlugin>>>>,
    /// Loaded env plugins
    envs: Arc<RwLock<HashMap<String, Box<dyn EnvPlugin>>>>,
    /// Loaded health plugins
    healths: Arc<RwLock<HashMap<String, Box<dyn HealthPlugin>>>>,
    /// Loaded proxy plugins
    proxies: Arc<RwLock<HashMap<String, Box<dyn ProxyPlugin>>>>,
    /// Loaded obs plugins
    obs: Arc<RwLock<HashMap<String, Box<dyn ObsPlugin>>>>,
    /// Auto-install enabled
    auto_install: bool,
}

impl Default for PluginLoader {
    fn default() -> Self {
        Self::new()
    }
}

impl PluginLoader {
    /// Create a new plugin loader
    pub fn new() -> Self {
        Self {
            plugins: Arc::new(RwLock::new(HashMap::new())),
            runners: Arc::new(RwLock::new(HashMap::new())),
            envs: Arc::new(RwLock::new(HashMap::new())),
            healths: Arc::new(RwLock::new(HashMap::new())),
            proxies: Arc::new(RwLock::new(HashMap::new())),
            obs: Arc::new(RwLock::new(HashMap::new())),
            auto_install: std::env::var("HIVE_AUTO_INSTALL")
                .map(|v| v != "false" && v != "0")
                .unwrap_or(true),
        }
    }

    /// Register a built-in runner plugin
    pub async fn register_runner(&self, plugin: Box<dyn RunnerPlugin>) {
        let metadata = plugin.metadata();
        let id = metadata.id.clone();

        let mut plugins = self.plugins.write().await;
        plugins.insert(
            id.clone(),
            PluginInfo {
                metadata,
                status: PluginStatus::BuiltIn,
                path: None,
            },
        );

        let mut runners = self.runners.write().await;
        runners.insert(id, plugin);
    }

    /// Register a built-in env plugin
    pub async fn register_env(&self, plugin: Box<dyn EnvPlugin>) {
        let metadata = plugin.metadata();
        let id = metadata.id.clone();

        let mut plugins = self.plugins.write().await;
        plugins.insert(
            id.clone(),
            PluginInfo {
                metadata,
                status: PluginStatus::BuiltIn,
                path: None,
            },
        );

        let mut envs = self.envs.write().await;
        envs.insert(id, plugin);
    }

    /// Register a built-in health plugin
    pub async fn register_health(&self, plugin: Box<dyn HealthPlugin>) {
        let metadata = plugin.metadata();
        let id = metadata.id.clone();

        let mut plugins = self.plugins.write().await;
        plugins.insert(
            id.clone(),
            PluginInfo {
                metadata,
                status: PluginStatus::BuiltIn,
                path: None,
            },
        );

        let mut healths = self.healths.write().await;
        healths.insert(id, plugin);
    }

    /// Register a built-in proxy plugin
    pub async fn register_proxy(&self, plugin: Box<dyn ProxyPlugin>) {
        let metadata = plugin.metadata();
        let id = metadata.id.clone();

        let mut plugins = self.plugins.write().await;
        plugins.insert(
            id.clone(),
            PluginInfo {
                metadata,
                status: PluginStatus::BuiltIn,
                path: None,
            },
        );

        let mut proxies = self.proxies.write().await;
        proxies.insert(id, plugin);
    }

    /// Register a built-in obs plugin
    pub async fn register_obs(&self, plugin: Box<dyn ObsPlugin>) {
        let metadata = plugin.metadata();
        let id = metadata.id.clone();

        let mut plugins = self.plugins.write().await;
        plugins.insert(
            id.clone(),
            PluginInfo {
                metadata,
                status: PluginStatus::BuiltIn,
                path: None,
            },
        );

        let mut obs = self.obs.write().await;
        obs.insert(id, plugin);
    }

    /// Check if a plugin is available
    pub async fn is_available(&self, plugin_id: &str) -> bool {
        let plugins = self.plugins.read().await;
        plugins
            .get(plugin_id)
            .map(|p| matches!(p.status, PluginStatus::BuiltIn | PluginStatus::Loaded))
            .unwrap_or(false)
    }

    /// Get plugin info
    pub async fn get_info(&self, plugin_id: &str) -> Option<PluginInfo> {
        let plugins = self.plugins.read().await;
        plugins.get(plugin_id).cloned()
    }

    /// List all plugins
    pub async fn list_all(&self) -> Vec<PluginInfo> {
        let plugins = self.plugins.read().await;
        plugins.values().cloned().collect()
    }

    /// List plugins by category
    pub async fn list_by_category(&self, category: PluginCategory) -> Vec<PluginInfo> {
        let plugins = self.plugins.read().await;
        plugins
            .values()
            .filter(|p| p.metadata.category == category)
            .cloned()
            .collect()
    }

    /// Get a runner plugin
    pub async fn get_runner(&self, _plugin_id: &str) -> Option<Arc<dyn RunnerPlugin>> {
        let _runners = self.runners.read().await;
        // Note: This is a simplified version. In a real implementation,
        // we'd need to handle the ownership differently (Arc wrapper or similar)
        None // Placeholder - actual implementation would need refactoring
    }

    /// Ensure a plugin is available, auto-installing if needed
    pub async fn ensure_available(&self, plugin_id: &str) -> Result<()> {
        if self.is_available(plugin_id).await {
            return Ok(());
        }

        if !self.auto_install {
            return Err(anyhow!(
                "Plugin '{}' is not installed and auto-install is disabled. \
                Install it manually with: adi plugin install {}",
                plugin_id,
                plugin_id
            ));
        }

        self.install(plugin_id).await
    }

    /// Install a plugin via ADI CLI
    pub async fn install(&self, plugin_id: &str) -> Result<()> {
        let (category, name) = PluginCategory::from_plugin_id(plugin_id)
            .ok_or_else(|| anyhow!("Invalid plugin ID: {}", plugin_id))?;

        tracing::info!("Installing plugin: {} (category: {:?})", plugin_id, category);

        let output = tokio::process::Command::new("adi")
            .args(["plugin", "install", plugin_id])
            .output()
            .await;

        match output {
            Ok(output) if output.status.success() => {
                let mut plugins = self.plugins.write().await;
                plugins.insert(
                    plugin_id.to_string(),
                    PluginInfo {
                        metadata: PluginMetadata {
                            id: plugin_id.to_string(),
                            name: name.to_string(),
                            version: "unknown".to_string(),
                            description: String::new(),
                            category,
                        },
                        status: PluginStatus::Available,
                        path: None,
                    },
                );
                Ok(())
            }
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                Err(anyhow!("Plugin installation failed: {}", stderr))
            }
            Err(e) => Err(anyhow!(
                "Failed to run 'adi plugin install': {}. \
                Plugin auto-install requires adi CLI to be available.",
                e
            )),
        }
    }
}

/// Global plugin loader instance
static PLUGIN_LOADER: std::sync::OnceLock<PluginLoader> = std::sync::OnceLock::new();

/// Get the global plugin loader
pub fn plugin_loader() -> &'static PluginLoader {
    PLUGIN_LOADER.get_or_init(PluginLoader::new)
}
