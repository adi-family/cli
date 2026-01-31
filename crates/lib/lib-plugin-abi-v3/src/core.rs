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
}

/// Plugin metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMetadata {
    /// Unique plugin identifier (e.g., "adi.tasks")
    pub id: String,

    /// Human-readable plugin name (e.g., "ADI Tasks")
    pub name: String,

    /// Semantic version (e.g., "0.8.8")
    pub version: String,

    /// Plugin type
    pub plugin_type: PluginType,

    /// Optional author
    pub author: Option<String>,

    /// Optional description
    pub description: Option<String>,
}

/// Plugin type classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PluginType {
    /// Core functionality plugin
    Core,

    /// Optional extension
    Extension,

    /// UI theme
    Theme,

    /// Custom font
    Font,
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
