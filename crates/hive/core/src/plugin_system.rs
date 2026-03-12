//! Hive Plugin System
//!
//! Provides infrastructure for plugin discovery, loading, and auto-installation.
//! Plugins are identified by their plugin ID (e.g., "hive.runner.docker").
//!
//! ## Plugin Types
//!
//! | Type | Prefix | Purpose |
//! |------|--------|---------|
//! | parse | hive.parse.* | Parse-time variable interpolation |
//! | runner | hive.runner.* | Execute services |
//! | env | hive.env.* | Provide environment variables |
//! | health | hive.health.* | Check service readiness |
//! | rollout | hive.rollout.* | Control deployment strategy |
//! | proxy.ssl | hive.proxy.ssl.* | TLS/SSL termination |
//! | proxy.auth | hive.proxy.auth.* | Proxy authentication |
//! | proxy | hive.proxy.* | Proxy middleware |
//! | obs | hive.obs.* | Observability |

use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PluginType {
    /// Parse-time interpolation plugins
    Parse,
    /// Service runner plugins
    Runner,
    /// Environment variable plugins
    Env,
    /// Health check plugins
    Health,
    /// Rollout strategy plugins
    Rollout,
    /// Proxy SSL plugins
    ProxySsl,
    /// Proxy authentication plugins
    ProxyAuth,
    /// Proxy middleware plugins
    Proxy,
    /// Observability plugins
    Obs,
}

impl PluginType {
    pub fn prefix(&self) -> &'static str {
        match self {
            PluginType::Parse => "hive.parse.",
            PluginType::Runner => "hive.runner.",
            PluginType::Env => "hive.env.",
            PluginType::Health => "hive.health.",
            PluginType::Rollout => "hive.rollout.",
            PluginType::ProxySsl => "hive.proxy.ssl.",
            PluginType::ProxyAuth => "hive.proxy.auth.",
            PluginType::Proxy => "hive.proxy.",
            PluginType::Obs => "hive.obs.",
        }
    }

    pub fn from_type_string(plugin_type: PluginType, type_str: &str) -> String {
        format!("{}{}", plugin_type.prefix(), type_str)
    }

    pub fn from_plugin_id(plugin_id: &str) -> Option<(PluginType, &str)> {
        // Order matters - more specific prefixes first
        if let Some(name) = plugin_id.strip_prefix("hive.proxy.ssl.") {
            return Some((PluginType::ProxySsl, name));
        }
        if let Some(name) = plugin_id.strip_prefix("hive.proxy.auth.") {
            return Some((PluginType::ProxyAuth, name));
        }
        if let Some(name) = plugin_id.strip_prefix("hive.proxy.") {
            return Some((PluginType::Proxy, name));
        }
        if let Some(name) = plugin_id.strip_prefix("hive.parse.") {
            return Some((PluginType::Parse, name));
        }
        if let Some(name) = plugin_id.strip_prefix("hive.runner.") {
            return Some((PluginType::Runner, name));
        }
        if let Some(name) = plugin_id.strip_prefix("hive.env.") {
            return Some((PluginType::Env, name));
        }
        if let Some(name) = plugin_id.strip_prefix("hive.health.") {
            return Some((PluginType::Health, name));
        }
        if let Some(name) = plugin_id.strip_prefix("hive.rollout.") {
            return Some((PluginType::Rollout, name));
        }
        if let Some(name) = plugin_id.strip_prefix("hive.obs.") {
            return Some((PluginType::Obs, name));
        }
        None
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PluginStatus {
    /// Plugin is bundled (compiled into binary via feature flags)
    Bundled,
    /// Plugin is installed via `adi plugin install`
    Installed,
    NotInstalled,
    Failed(String),
}

#[derive(Debug, Clone)]
pub struct PluginInfo {
    /// Full plugin ID (e.g., "hive.runner.docker")
    pub id: String,
    pub plugin_type: PluginType,
    /// Short name (e.g., "docker")
    pub name: String,
    pub status: PluginStatus,
    pub description: Option<String>,
}

/// Plugin registry manages available plugins
pub struct PluginRegistry {
    plugins: Arc<RwLock<HashMap<String, PluginInfo>>>,
    auto_install_enabled: bool,
}

impl PluginRegistry {
    pub fn new(auto_install_enabled: bool) -> Self {
        let registry = Self {
            plugins: Arc::new(RwLock::new(HashMap::new())),
            auto_install_enabled,
        };

        // Register built-in plugins synchronously (since we're in new())
        let builtins = Self::builtin_plugins();
        let plugins = Arc::clone(&registry.plugins);
        
        // Use blocking lock since we're not async yet
        if let Ok(mut guard) = plugins.try_write() {
            for info in builtins {
                guard.insert(info.id.clone(), info);
            }
        }

        registry
    }

    /// Get list of core plugins (implemented directly in hive-core, not via separate crates)
    ///
    /// These are plugins whose logic is built into hive-core itself,
    /// as opposed to installable plugins that are separate crates.
    fn builtin_plugins() -> Vec<PluginInfo> {
        vec![
            // Core parse plugins (implemented in hive-core)
            PluginInfo {
                id: "hive.parse.env".to_string(),
                plugin_type: PluginType::Parse,
                name: "env".to_string(),
                status: PluginStatus::Bundled,
                description: Some("Environment variable interpolation".to_string()),
            },
            PluginInfo {
                id: "hive.parse.service".to_string(),
                plugin_type: PluginType::Parse,
                name: "service".to_string(),
                status: PluginStatus::Bundled,
                description: Some("Service name interpolation".to_string()),
            },
            // Core env plugins (implemented in hive-core)
            PluginInfo {
                id: "hive.env.static".to_string(),
                plugin_type: PluginType::Env,
                name: "static".to_string(),
                status: PluginStatus::Bundled,
                description: Some("Static key-value environment variables".to_string()),
            },
            // Core health plugins (implemented in hive-core)
            PluginInfo {
                id: "hive.health.cmd".to_string(),
                plugin_type: PluginType::Health,
                name: "cmd".to_string(),
                status: PluginStatus::Bundled,
                description: Some("Command execution health check".to_string()),
            },
            // Core rollout plugins (implemented in hive-core)
            PluginInfo {
                id: "hive.rollout.recreate".to_string(),
                plugin_type: PluginType::Rollout,
                name: "recreate".to_string(),
                status: PluginStatus::Bundled,
                description: Some("Stop-then-start deployment".to_string()),
            },
        ]
    }

    pub async fn is_available(&self, plugin_id: &str) -> bool {
        let plugins = self.plugins.read().await;
        plugins
            .get(plugin_id)
            .map(|p| matches!(p.status, PluginStatus::Bundled | PluginStatus::Installed))
            .unwrap_or(false)
    }

    pub async fn is_bundled(&self, plugin_id: &str) -> bool {
        let plugins = self.plugins.read().await;
        plugins
            .get(plugin_id)
            .map(|p| p.status == PluginStatus::Bundled)
            .unwrap_or(false)
    }

    /// Alias for backwards compatibility
    #[deprecated(since = "0.2.0", note = "Use is_bundled() instead")]
    pub async fn is_builtin(&self, plugin_id: &str) -> bool {
        self.is_bundled(plugin_id).await
    }

    pub async fn get(&self, plugin_id: &str) -> Option<PluginInfo> {
        let plugins = self.plugins.read().await;
        plugins.get(plugin_id).cloned()
    }

    pub async fn list_by_type(&self, plugin_type: PluginType) -> Vec<PluginInfo> {
        let plugins = self.plugins.read().await;
        plugins
            .values()
            .filter(|p| p.plugin_type == plugin_type)
            .cloned()
            .collect()
    }

    /// Ensure a plugin is available, auto-installing if necessary
    pub async fn ensure_available(&self, plugin_id: &str) -> Result<()> {
        if self.is_available(plugin_id).await {
            return Ok(());
        }

        if !self.auto_install_enabled {
            return Err(anyhow!(
                "Plugin '{}' is not installed and auto-install is disabled. \
                Install it manually with: adi plugin install {}",
                plugin_id,
                plugin_id
            ));
        }

        info!("Auto-installing plugin: {}", plugin_id);
        self.install(plugin_id).await
    }

    pub async fn install(&self, plugin_id: &str) -> Result<()> {
        // Parse plugin type
        let (plugin_type, name) = PluginType::from_plugin_id(plugin_id)
            .ok_or_else(|| anyhow!("Invalid plugin ID format: {}", plugin_id))?;

        info!("Installing plugin: {} (type: {:?})", plugin_id, plugin_type);

        // Try to install via adi CLI
        let output = tokio::process::Command::new("adi")
            .args(["plugin", "install", plugin_id])
            .output()
            .await;

        match output {
            Ok(output) if output.status.success() => {
                // Register the plugin
                let mut plugins = self.plugins.write().await;
                plugins.insert(
                    plugin_id.to_string(),
                    PluginInfo {
                        id: plugin_id.to_string(),
                        plugin_type,
                        name: name.to_string(),
                        status: PluginStatus::Installed,
                        description: None,
                    },
                );
                info!("Plugin installed successfully: {}", plugin_id);
                Ok(())
            }
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                let error_msg = format!("Plugin installation failed: {}", stderr);
                
                // Register as failed
                let mut plugins = self.plugins.write().await;
                plugins.insert(
                    plugin_id.to_string(),
                    PluginInfo {
                        id: plugin_id.to_string(),
                        plugin_type,
                        name: name.to_string(),
                        status: PluginStatus::Failed(error_msg.clone()),
                        description: None,
                    },
                );
                
                Err(anyhow!("{}", error_msg))
            }
            Err(e) => {
                warn!(
                    "Failed to run 'adi plugin install': {}. \
                    Plugin auto-install requires adi CLI to be available.",
                    e
                );
                Err(anyhow!(
                    "Plugin '{}' is not available. Install it with: adi plugin install {}",
                    plugin_id,
                    plugin_id
                ))
            }
        }
    }

    pub async fn register_installed(&self, plugin_id: &str, description: Option<String>) {
        let (plugin_type, name) = match PluginType::from_plugin_id(plugin_id) {
            Some(t) => t,
            None => {
                warn!("Cannot register plugin with invalid ID: {}", plugin_id);
                return;
            }
        };

        let mut plugins = self.plugins.write().await;
        plugins.insert(
            plugin_id.to_string(),
            PluginInfo {
                id: plugin_id.to_string(),
                plugin_type,
                name: name.to_string(),
                status: PluginStatus::Installed,
                description,
            },
        );
    }
}

static PLUGIN_REGISTRY: std::sync::OnceLock<PluginRegistry> = std::sync::OnceLock::new();

pub fn plugin_registry() -> &'static PluginRegistry {
    PLUGIN_REGISTRY.get_or_init(|| {
        let auto_install = lib_env_parse::is_truthy(
            &lib_env_parse::env_require("HIVE_AUTO_INSTALL")
                .expect("required environment variable `HIVE_AUTO_INSTALL` is not set"),
        );
        PluginRegistry::new(auto_install)
    })
}

/// Resolve a type string to a full plugin ID
pub fn resolve_plugin_id(plugin_type: PluginType, type_str: &str) -> String {
    // If it already looks like a full ID, return it
    if type_str.starts_with("hive.") {
        return type_str.to_string();
    }
    
    format!("{}{}", plugin_type.prefix(), type_str)
}

/// Check if a plugin type string refers to a core plugin (implemented in hive-core itself)
///
/// Core plugins don't require installation - they're part of hive-core.
/// Other plugins (docker, http health, obs, etc.) are installable via `adi plugin install`.
pub fn is_core_plugin(plugin_type: PluginType, type_str: &str) -> bool {
    match plugin_type {
        PluginType::Parse => matches!(type_str, "env" | "service"),
        PluginType::Runner => false,
        PluginType::Env => matches!(type_str, "static"),
        PluginType::Health => matches!(type_str, "cmd"),
        PluginType::Rollout => matches!(type_str, "recreate"),
        _ => false,
    }
}

/// Alias for backwards compatibility
#[deprecated(since = "0.2.0", note = "Use is_core_plugin() instead")]
pub fn is_builtin_plugin(plugin_type: PluginType, type_str: &str) -> bool {
    is_core_plugin(plugin_type, type_str)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_type_prefix() {
        assert_eq!(PluginType::Runner.prefix(), "hive.runner.");
        assert_eq!(PluginType::Env.prefix(), "hive.env.");
        assert_eq!(PluginType::ProxySsl.prefix(), "hive.proxy.ssl.");
    }

    #[test]
    fn test_plugin_type_from_id() {
        let (t, n) = PluginType::from_plugin_id("hive.runner.docker").unwrap();
        assert_eq!(t, PluginType::Runner);
        assert_eq!(n, "docker");

        let (t, n) = PluginType::from_plugin_id("hive.proxy.ssl.letsencrypt").unwrap();
        assert_eq!(t, PluginType::ProxySsl);
        assert_eq!(n, "letsencrypt");
    }

    #[test]
    fn test_resolve_plugin_id() {
        assert_eq!(
            resolve_plugin_id(PluginType::Runner, "docker"),
            "hive.runner.docker"
        );
        assert_eq!(
            resolve_plugin_id(PluginType::Runner, "hive.runner.docker"),
            "hive.runner.docker"
        );
    }

    #[test]
    fn test_is_core_plugin() {
        // Core plugins (implemented in hive-core)
        assert!(is_core_plugin(PluginType::Health, "cmd"));
        assert!(is_core_plugin(PluginType::Parse, "env"));

        // Script and docker runners are not core — they go through the plugin system
        assert!(!is_core_plugin(PluginType::Runner, "script"));
        assert!(!is_core_plugin(PluginType::Runner, "docker"));
        assert!(!is_core_plugin(PluginType::Health, "http"));
        assert!(!is_core_plugin(PluginType::Health, "tcp"));
        assert!(!is_core_plugin(PluginType::Obs, "stdout"));
    }

    #[tokio::test]
    async fn test_registry_core_plugins() {
        let registry = PluginRegistry::new(true);

        // Script runner is registered via ServiceManager, not the PluginRegistry
        assert!(!registry.is_available("hive.runner.script").await);

        // Docker is a bundled plugin registered at startup, not a core plugin
        assert!(!registry.is_available("hive.runner.docker").await);

        // Installable plugins are not available by default
        assert!(!registry.is_available("hive.health.http").await);
    }
}
