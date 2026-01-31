//! Health Check Plugin Trait
//!
//! Health plugins check if a service is ready to accept traffic.
//! Examples: http (HTTP endpoint), tcp (port check), cmd (command), grpc, etc.

use crate::types::RuntimeContext;
use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Trait for health check plugins
#[async_trait]
pub trait HealthPlugin: Send + Sync {
    /// Get plugin metadata
    fn metadata(&self) -> crate::PluginMetadata;

    /// Initialize the plugin with default configuration
    async fn init(&mut self, defaults: &serde_json::Value) -> Result<()> {
        let _ = defaults;
        Ok(())
    }

    /// Perform a health check
    async fn check(&self, config: &serde_json::Value, ctx: &RuntimeContext) -> Result<HealthResult>;

    /// Shutdown the plugin
    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }
}

/// Result of a health check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResult {
    /// Whether the service is healthy
    pub healthy: bool,
    /// Optional message describing the result
    pub message: Option<String>,
    /// Response time in milliseconds
    pub response_time_ms: Option<u64>,
    /// Additional details
    pub details: Option<serde_json::Value>,
}

impl HealthResult {
    /// Create a healthy result
    pub fn healthy() -> Self {
        Self {
            healthy: true,
            message: None,
            response_time_ms: None,
            details: None,
        }
    }

    /// Create an unhealthy result
    pub fn unhealthy(message: impl Into<String>) -> Self {
        Self {
            healthy: false,
            message: Some(message.into()),
            response_time_ms: None,
            details: None,
        }
    }

    /// Add response time
    pub fn with_response_time(mut self, ms: u64) -> Self {
        self.response_time_ms = Some(ms);
        self
    }

    /// Add a message
    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }
}
