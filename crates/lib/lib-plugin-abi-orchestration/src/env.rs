//! Environment Plugin Trait
//!
//! Environment plugins provide environment variables to services.
//! Examples: static (key-value), dotenv (from files), vault (secrets), etc.

use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashMap;

/// Trait for environment plugins
#[async_trait]
pub trait EnvPlugin: Send + Sync {
    /// Get plugin metadata
    fn metadata(&self) -> crate::PluginMetadata;

    /// Load environment variables from the plugin
    async fn load(&self, config: &serde_json::Value) -> Result<HashMap<String, String>>;

    /// Refresh environment variables (for plugins that support hot-reload)
    async fn refresh(&self, _config: &serde_json::Value) -> Result<HashMap<String, String>> {
        self.load(_config).await
    }
}
