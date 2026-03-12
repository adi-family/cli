//! Core Plugin System
//!
//! Core plugins extend the Hive daemon itself with additional functionality.
//! Unlike runtime plugins (runners, health checks, etc.), core plugins can:
//! - React to daemon lifecycle events
//! - Manage services programmatically
//! - Extend control interfaces (WebSocket, HTTP, etc.)
//! - Implement custom orchestration logic
//!
//! Core plugins receive a DaemonClient to interact with the daemon.

use anyhow::Result;
use async_trait::async_trait;
use lib_hive_daemon_client::DaemonClient;
use std::sync::Arc;
use tracing::info;

#[async_trait]
pub trait CorePlugin: Send + Sync {
    fn name(&self) -> &str;

    fn version(&self) -> &str {
        env!("CARGO_PKG_VERSION")
    }

    /// Called once at daemon startup.
    async fn init(&mut self, client: Arc<DaemonClient>) -> Result<()>;

    async fn shutdown(&mut self) -> Result<()> {
        Ok(())
    }

    async fn on_event(&self, event: DaemonEvent) -> Result<()> {
        let _ = event;
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub enum DaemonEvent {
    DaemonStarted,
    DaemonShutdown,
    SourceAdded { name: String },
    SourceRemoved { name: String },
    ServiceStarted { fqn: String },
    ServiceStopped { fqn: String },
    ServiceCrashed { fqn: String, exit_code: i32 },
    ServiceHealthChanged { fqn: String, healthy: bool },
}

pub struct CorePluginRegistry {
    plugins: Vec<Box<dyn CorePlugin>>,
    daemon_client: Option<Arc<DaemonClient>>,
}

impl CorePluginRegistry {
    pub fn new() -> Self {
        Self {
            plugins: Vec::new(),
            daemon_client: None,
        }
    }

    pub fn register<P: CorePlugin + 'static>(&mut self, plugin: P) {
        info!("Registering core plugin: {}", plugin.name());
        self.plugins.push(Box::new(plugin));
    }

    pub async fn init_all(&mut self, daemon_client: Arc<DaemonClient>) -> Result<()> {
        info!("Initializing {} core plugins", self.plugins.len());

        self.daemon_client = Some(daemon_client.clone());

        for plugin in &mut self.plugins {
            info!("Initializing plugin: {}", plugin.name());
            plugin.init(daemon_client.clone()).await?;
        }

        Ok(())
    }

    pub async fn shutdown_all(&mut self) -> Result<()> {
        info!("Shutting down {} core plugins", self.plugins.len());

        for plugin in &mut self.plugins {
            info!("Shutting down plugin: {}", plugin.name());
            if let Err(e) = plugin.shutdown().await {
                tracing::error!("Error shutting down plugin {}: {}", plugin.name(), e);
            }
        }

        Ok(())
    }

    pub async fn broadcast_event(&self, event: DaemonEvent) {
        for plugin in &self.plugins {
            if let Err(e) = plugin.on_event(event.clone()).await {
                tracing::error!(
                    "Error handling event {:?} in plugin {}: {}",
                    event,
                    plugin.name(),
                    e
                );
            }
        }
    }

    pub fn count(&self) -> usize {
        self.plugins.len()
    }
}

impl Default for CorePluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};

    struct TestPlugin {
        initialized: Arc<AtomicBool>,
    }

    #[async_trait]
    impl CorePlugin for TestPlugin {
        fn name(&self) -> &str {
            "test-plugin"
        }

        async fn init(&mut self, _client: Arc<DaemonClient>) -> Result<()> {
            self.initialized.store(true, Ordering::SeqCst);
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_plugin_registration() {
        let mut registry = CorePluginRegistry::new();
        let initialized = Arc::new(AtomicBool::new(false));

        let plugin = TestPlugin {
            initialized: initialized.clone(),
        };

        registry.register(plugin);
        assert_eq!(registry.count(), 1);
    }
}
