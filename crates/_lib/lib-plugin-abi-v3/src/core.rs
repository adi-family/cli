//! Core plugin trait and types

use crate::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;

/// Base trait that all plugins must implement
#[async_trait]
pub trait Plugin: Send + Sync {
    /// Returns plugin metadata
    fn metadata(&self) -> PluginMetadata;

    /// Initialize plugin with context
    ///
    /// Called once when the plugin is loaded. Use this to set up resources,
    /// load configuration, and prepare the plugin for use.
    async fn init(&mut self, ctx: &PluginContext) -> Result<()>;

    /// Shutdown plugin gracefully
    ///
    /// Called before the plugin is unloaded. Use this to clean up resources,
    /// close connections, and ensure graceful shutdown.
    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    /// Optional: Handle custom events/messages
    ///
    /// Plugins can receive events from the host for things like config changes,
    /// hot-reload triggers, or custom application events.
    async fn handle_event(&self, _event: &PluginEvent) -> Result<()> {
        Ok(())
    }

    /// List services provided by this plugin
    ///
    /// Returns a list of service type identifiers that this plugin provides.
    /// Used for capability discovery.
    fn provides(&self) -> Vec<&'static str> {
        vec![]
    }
}

/// Plugin metadata
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PluginMetadata {
    /// Unique plugin identifier (e.g., "adi.tasks", "hive.runner.docker")
    #[serde(default)]
    pub id: String,

    /// Human-readable plugin name (e.g., "ADI Tasks", "Docker Runner")
    #[serde(default)]
    pub name: String,

    /// Semantic version (e.g., "0.8.8")
    #[serde(default)]
    pub version: String,

    /// Plugin type
    #[serde(default)]
    pub plugin_type: PluginType,

    /// Optional author
    #[serde(default)]
    pub author: Option<String>,

    /// Optional description
    #[serde(default)]
    pub description: Option<String>,

    /// Optional orchestration category (for Hive plugins)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category: Option<PluginCategory>,
}

impl PluginMetadata {
    /// Create a new PluginMetadata with required fields
    pub fn new(id: impl Into<String>, name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            version: version.into(),
            plugin_type: PluginType::Extension,
            author: None,
            description: None,
            category: None,
        }
    }

    /// Set the plugin type
    pub fn with_type(mut self, plugin_type: PluginType) -> Self {
        self.plugin_type = plugin_type;
        self
    }

    /// Set the author
    pub fn with_author(mut self, author: impl Into<String>) -> Self {
        self.author = Some(author.into());
        self
    }

    /// Set the description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set the category (for orchestration plugins)
    pub fn with_category(mut self, category: PluginCategory) -> Self {
        self.category = Some(category);
        self
    }
}

/// Plugin type classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum PluginType {
    /// Core functionality plugin
    Core,

    /// Optional extension
    #[default]
    Extension,

    /// UI theme
    Theme,

    /// Custom font
    Font,

    /// Orchestration plugin (Hive)
    Orchestration,
}

/// Plugin categories for orchestration plugins
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PluginCategory {
    /// Service runners (Docker, script, compose, podman)
    Runner,
    /// Environment variable providers (dotenv, vault, 1password, AWS secrets)
    Env,
    /// Health check providers (HTTP, TCP, gRPC, databases)
    Health,
    /// HTTP proxy middleware (CORS, rate limit, auth, IP filter)
    Proxy,
    /// Observability (stdout, file, Loki, Prometheus)
    Obs,
    /// Deployment strategies (recreate, blue-green)
    Rollout,
}

impl PluginCategory {
    /// Get the plugin ID prefix for this category
    pub fn prefix(&self) -> &'static str {
        match self {
            PluginCategory::Runner => "hive.runner.",
            PluginCategory::Env => "hive.env.",
            PluginCategory::Health => "hive.health.",
            PluginCategory::Proxy => "hive.proxy.",
            PluginCategory::Obs => "hive.obs.",
            PluginCategory::Rollout => "hive.rollout.",
        }
    }

    /// Parse category from plugin ID
    pub fn from_plugin_id(plugin_id: &str) -> Option<(Self, &str)> {
        if let Some(name) = plugin_id.strip_prefix("hive.runner.") {
            Some((PluginCategory::Runner, name))
        } else if let Some(name) = plugin_id.strip_prefix("hive.env.") {
            Some((PluginCategory::Env, name))
        } else if let Some(name) = plugin_id.strip_prefix("hive.health.") {
            Some((PluginCategory::Health, name))
        } else if let Some(name) = plugin_id.strip_prefix("hive.proxy.") {
            Some((PluginCategory::Proxy, name))
        } else if let Some(name) = plugin_id.strip_prefix("hive.obs.") {
            Some((PluginCategory::Obs, name))
        } else if let Some(name) = plugin_id.strip_prefix("hive.rollout.") {
            Some((PluginCategory::Rollout, name))
        } else {
            None
        }
    }
}

/// Resolve a short name to a full plugin ID
pub fn resolve_plugin_id(category: PluginCategory, name: &str) -> String {
    if name.starts_with("hive.") {
        name.to_string()
    } else {
        format!("{}{}", category.prefix(), name)
    }
}

/// Plugin initialization context
///
/// Provides the plugin with necessary information and resources for initialization.
pub struct PluginContext {
    /// Plugin identifier
    pub plugin_id: String,

    /// Plugin data directory (~/.local/share/adi/<plugin-id>/)
    pub data_dir: PathBuf,

    /// Plugin config directory (~/.config/adi/<plugin-id>/)
    pub config_dir: PathBuf,

    /// Plugin configuration from config.toml
    pub config: Value,
}

impl PluginContext {
    /// Create a new plugin context
    pub fn new(
        plugin_id: impl Into<String>,
        data_dir: PathBuf,
        config_dir: PathBuf,
        config: Value,
    ) -> Self {
        Self {
            plugin_id: plugin_id.into(),
            data_dir,
            config_dir,
            config,
        }
    }
}

/// Plugin events
///
/// Events that plugins can receive from the host.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PluginEvent {
    /// Configuration changed
    ConfigChanged(Value),

    /// Host is shutting down
    HostShutdown,

    /// Custom event
    Custom {
        event_type: String,
        data: Value,
    },
}
