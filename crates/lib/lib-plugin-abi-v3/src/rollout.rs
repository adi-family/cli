//! Rollout strategy plugin trait

use crate::{Plugin, Result};
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use std::time::Duration;

/// Rollout strategy plugin trait
///
/// Rollout strategies control how service updates are deployed
/// (recreate, blue-green, canary, rolling, etc.).
#[async_trait]
pub trait RolloutStrategy: Plugin {
    /// Plan rollout steps
    async fn plan(&self, config: &Value) -> Result<Vec<RolloutStep>>;

    /// Execute a single rollout step
    async fn execute_step(&self, step: &RolloutStep, ctx: &RolloutContext) -> Result<RolloutStepResult>;

    /// Rollback deployment
    async fn rollback(&self, ctx: &RolloutContext) -> Result<()>;
}

/// Rollout step
#[derive(Debug, Clone)]
pub enum RolloutStep {
    /// Stop a service instance
    Stop { instance: String },

    /// Start a service instance
    Start { instance: String },

    /// Wait for instance to be healthy
    WaitHealthy {
        instance: String,
        timeout: Duration,
    },

    /// Switch traffic from one instance to another
    SwitchTraffic { from: String, to: String },

    /// Wait for a duration
    Wait { duration: Duration },

    /// Run a custom command
    Command {
        command: String,
        args: Vec<String>,
    },
}

/// Rollout context
#[derive(Debug, Clone)]
pub struct RolloutContext {
    /// Service name
    pub service_name: String,

    /// Old version
    pub old_version: String,

    /// New version
    pub new_version: String,

    /// Running instances
    pub instances: HashMap<String, crate::runner::ProcessHandle>,

    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

/// Rollout step result
#[derive(Debug, Clone)]
pub struct RolloutStepResult {
    /// Whether the step succeeded
    pub success: bool,

    /// Optional message
    pub message: Option<String>,
}

impl RolloutStepResult {
    /// Create a successful result
    pub fn success() -> Self {
        Self {
            success: true,
            message: None,
        }
    }

    /// Create a failed result
    pub fn failed(message: impl Into<String>) -> Self {
        Self {
            success: false,
            message: Some(message.into()),
        }
    }
}
