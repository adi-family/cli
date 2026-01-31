//! Health check plugin trait

use crate::{Plugin, Result};
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;

/// Health check plugin trait
///
/// Health checks verify that services are ready and functioning correctly.
#[async_trait]
pub trait HealthCheck: Plugin {
    /// Perform a health check
    async fn check(&self, config: &Value, ctx: &RuntimeContext) -> Result<HealthResult>;
}

/// Health check result
#[derive(Debug, Clone)]
pub struct HealthResult {
    /// Whether the service is healthy
    pub healthy: bool,

    /// Optional message
    pub message: Option<String>,

    /// Response time in milliseconds
    pub response_time_ms: u64,

    /// Additional details
    pub details: HashMap<String, String>,
}

impl HealthResult {
    /// Create a healthy result
    pub fn healthy() -> Self {
        Self {
            healthy: true,
            message: None,
            response_time_ms: 0,
            details: HashMap::new(),
        }
    }

    /// Create an unhealthy result
    pub fn unhealthy(message: impl Into<String>) -> Self {
        Self {
            healthy: false,
            message: Some(message.into()),
            response_time_ms: 0,
            details: HashMap::new(),
        }
    }

    /// Add a detail
    pub fn with_detail(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.details.insert(key.into(), value.into());
        self
    }

    /// Set response time
    pub fn with_response_time(mut self, ms: u64) -> Self {
        self.response_time_ms = ms;
        self
    }
}

/// Runtime context (re-exported from runner module)
pub use crate::runner::RuntimeContext;
