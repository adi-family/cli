//! Plugin Bridge Module
//!
//! Bridges the lib-plugin-abi-v3 trait system with hive-core by storing concrete
//! plugin instances (as trait objects) in categorized HashMaps.
//!
//! All plugins are **installable** via `adi plugin install <plugin-id>`.
//! Plugins are auto-installed on first use when referenced in hive.yaml.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;


use lib_plugin_abi_v3::env::EnvProvider;
use lib_plugin_abi_v3::health::HealthCheck;
use lib_plugin_abi_v3::obs::ObservabilitySink;
use lib_plugin_abi_v3::proxy::ProxyMiddleware;
use lib_plugin_abi_v3::runner::Runner;
use lib_plugin_abi_v3::{PluginCategory, PluginMetadata};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PluginLoadStatus {
    /// Plugin is bundled (compiled into binary via feature flags)
    Bundled,
    /// Plugin is installed via `adi plugin install`
    Installed,
    NotAvailable,
}

/// Plugin manager for hive-core
pub struct PluginManager {
    /// Runner plugins by name (e.g., "docker", "script")
    runners: Arc<RwLock<HashMap<String, Arc<dyn Runner>>>>,
    envs: Arc<RwLock<HashMap<String, Arc<dyn EnvProvider>>>>,
    healths: Arc<RwLock<HashMap<String, Arc<dyn HealthCheck>>>>,
    proxies: Arc<RwLock<HashMap<String, Arc<dyn ProxyMiddleware>>>>,
    obs: Arc<RwLock<HashMap<String, Arc<dyn ObservabilitySink>>>>,
    metadata: Arc<RwLock<HashMap<String, (PluginMetadata, PluginLoadStatus)>>>,
}

impl Default for PluginManager {
    fn default() -> Self {
        Self::new()
    }
}

impl PluginManager {
    pub fn new() -> Self {
        Self {
            runners: Arc::new(RwLock::new(HashMap::new())),
            envs: Arc::new(RwLock::new(HashMap::new())),
            healths: Arc::new(RwLock::new(HashMap::new())),
            proxies: Arc::new(RwLock::new(HashMap::new())),
            obs: Arc::new(RwLock::new(HashMap::new())),
            metadata: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn register_runner(&self, plugin: impl Runner + 'static) {
        let meta = plugin.metadata();
        let name = meta.name.clone();
        let id = meta.id.clone();

        let mut metadata = self.metadata.write().await;
        metadata.insert(id, (meta, PluginLoadStatus::Bundled));

        let mut runners = self.runners.write().await;
        runners.insert(name, Arc::new(plugin));
    }

    pub async fn register_env(&self, plugin: impl EnvProvider + 'static) {
        let meta = plugin.metadata();
        let name = meta.name.clone();
        let id = meta.id.clone();

        let mut metadata = self.metadata.write().await;
        metadata.insert(id, (meta, PluginLoadStatus::Bundled));

        let mut envs = self.envs.write().await;
        envs.insert(name, Arc::new(plugin));
    }

    pub async fn register_health(&self, plugin: impl HealthCheck + 'static) {
        let meta = plugin.metadata();
        let name = meta.name.clone();
        let id = meta.id.clone();

        let mut metadata = self.metadata.write().await;
        metadata.insert(id, (meta, PluginLoadStatus::Bundled));

        let mut healths = self.healths.write().await;
        healths.insert(name, Arc::new(plugin));
    }

    pub async fn register_proxy(&self, plugin: impl ProxyMiddleware + 'static) {
        let meta = plugin.metadata();
        let name = meta.name.clone();
        let id = meta.id.clone();

        let mut metadata = self.metadata.write().await;
        metadata.insert(id, (meta, PluginLoadStatus::Bundled));

        let mut proxies = self.proxies.write().await;
        proxies.insert(name, Arc::new(plugin));
    }

    pub async fn register_obs(&self, plugin: impl ObservabilitySink + 'static) {
        let meta = plugin.metadata();
        let name = meta.name.clone();
        let id = meta.id.clone();

        let mut metadata = self.metadata.write().await;
        metadata.insert(id, (meta, PluginLoadStatus::Bundled));

        let mut obs = self.obs.write().await;
        obs.insert(name, Arc::new(plugin));
    }

    /// Register a dynamically loaded runner plugin (from installed plugins on disk).
    ///
    /// Uses the plugin ID to derive the short runner name (e.g., "docker" from "hive.runner.docker").
    pub async fn register_dynamic_runner(&self, runner: Arc<dyn Runner>) {
        let meta = runner.metadata();
        let id = meta.id.clone();
        let name = id
            .strip_prefix("hive.runner.")
            .unwrap_or(&meta.name)
            .to_string();

        let mut metadata = self.metadata.write().await;
        metadata.insert(id, (meta, PluginLoadStatus::Installed));

        let mut runners = self.runners.write().await;
        runners.insert(name, runner);
    }

    pub async fn get_runner(&self, name: &str) -> Option<Arc<dyn Runner>> {
        let runners = self.runners.read().await;
        runners.get(name).cloned()
    }

    pub async fn get_env(&self, name: &str) -> Option<Arc<dyn EnvProvider>> {
        let envs = self.envs.read().await;
        envs.get(name).cloned()
    }

    pub async fn get_health(&self, name: &str) -> Option<Arc<dyn HealthCheck>> {
        let healths = self.healths.read().await;
        healths.get(name).cloned()
    }

    pub async fn get_proxy(&self, name: &str) -> Option<Arc<dyn ProxyMiddleware>> {
        let proxies = self.proxies.read().await;
        proxies.get(name).cloned()
    }

    pub async fn get_obs(&self, name: &str) -> Option<Arc<dyn ObservabilitySink>> {
        let obs = self.obs.read().await;
        obs.get(name).cloned()
    }

    pub async fn is_available(&self, plugin_id: &str) -> bool {
        let metadata = self.metadata.read().await;
        metadata.contains_key(plugin_id)
    }

    pub async fn list_plugins(&self) -> Vec<(PluginMetadata, PluginLoadStatus)> {
        let metadata = self.metadata.read().await;
        metadata.values().cloned().collect()
    }

    pub async fn list_by_category(&self, category: PluginCategory) -> Vec<PluginMetadata> {
        let metadata = self.metadata.read().await;
        metadata
            .values()
            .filter(|(m, _)| m.category == Some(category))
            .map(|(m, _)| m.clone())
            .collect()
    }

    pub async fn get_all_runners(&self) -> HashMap<String, Arc<dyn Runner>> {
        let runners = self.runners.read().await;
        runners.clone()
    }

    pub async fn runner_names(&self) -> Vec<String> {
        let runners = self.runners.read().await;
        runners.keys().cloned().collect()
    }

    pub async fn health_names(&self) -> Vec<String> {
        let healths = self.healths.read().await;
        healths.keys().cloned().collect()
    }

    pub async fn obs_names(&self) -> Vec<String> {
        let obs = self.obs.read().await;
        obs.keys().cloned().collect()
    }
}

/// Initialize plugin manager.
///
/// Registers only the script runner (always bundled).
/// All other plugins are installed via `adi plugin install`.
pub async fn init_plugins() -> PluginManager {
    let manager = PluginManager::new();
    manager
        .register_runner(hive_runner_script::ScriptRunnerPlugin::new())
        .await;
    manager
}


static PLUGIN_MANAGER: std::sync::OnceLock<PluginManager> = std::sync::OnceLock::new();

pub fn plugin_manager() -> &'static PluginManager {
    PLUGIN_MANAGER.get_or_init(PluginManager::new)
}

/// Call this once at startup
pub async fn init_global_plugins() -> &'static PluginManager {
    // Initialize if not already done
    if PLUGIN_MANAGER.get().is_none() {
        let manager = init_plugins().await;
        let _ = PLUGIN_MANAGER.set(manager);
    }
    PLUGIN_MANAGER.get().unwrap()
}

// Re-export v3 plugin ABI types for external use
pub mod abi {
    pub use lib_plugin_abi_v3::*;
}
