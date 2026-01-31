//! Rollout Strategy Plugin Trait
//!
//! Rollout plugins control how services are deployed/updated.
//! Examples: recreate (stop-start), blue-green, canary, rolling, etc.

use crate::runner::ProcessHandle;
use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Trait for rollout strategy plugins
#[async_trait]
pub trait RolloutPlugin: Send + Sync {
    /// Get plugin metadata
    fn metadata(&self) -> crate::PluginMetadata;

    /// Initialize the plugin with configuration
    async fn init(&mut self, config: &serde_json::Value) -> Result<()> {
        let _ = config;
        Ok(())
    }

    /// Plan the rollout steps
    async fn plan(&self, config: &serde_json::Value) -> Result<Vec<RolloutStep>>;

    /// Execute a single rollout step
    async fn execute_step(
        &self,
        step: &RolloutStep,
        context: &RolloutContext,
    ) -> Result<RolloutStepResult>;

    /// Rollback the deployment
    async fn rollback(&self, context: &RolloutContext) -> Result<()>;

    /// Shutdown the plugin
    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }
}

/// Rollout strategy type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RolloutStrategy {
    /// Stop old, start new (default)
    Recreate,
    /// Start new, switch traffic, stop old
    BlueGreen,
    /// Gradual traffic shift
    Canary,
    /// Rolling update (for replicated services)
    Rolling,
}

/// A single step in the rollout process
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RolloutStep {
    /// Step ID
    pub id: String,
    /// Step type
    pub step_type: RolloutStepType,
    /// Step description
    pub description: String,
    /// Step configuration
    pub config: serde_json::Value,
}

/// Types of rollout steps
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RolloutStepType {
    /// Stop existing process
    Stop,
    /// Start new process
    Start,
    /// Wait for health check
    WaitHealthy,
    /// Switch traffic
    SwitchTraffic,
    /// Wait for a duration
    Wait,
    /// Run a custom command
    Command,
}

/// Context for rollout execution
#[derive(Debug, Clone)]
pub struct RolloutContext {
    /// Service name
    pub service_name: String,
    /// Current process handle (if any)
    pub current_handle: Option<ProcessHandle>,
    /// New process handle (if started)
    pub new_handle: Option<ProcessHandle>,
    /// Rollout configuration
    pub config: serde_json::Value,
}

/// Result of a rollout step
#[derive(Debug, Clone)]
pub struct RolloutStepResult {
    /// Whether the step succeeded
    pub success: bool,
    /// Updated context (e.g., with new process handle)
    pub context: RolloutContext,
    /// Optional message
    pub message: Option<String>,
}

impl RolloutStepResult {
    /// Create a successful result
    pub fn success(context: RolloutContext) -> Self {
        Self {
            success: true,
            context,
            message: None,
        }
    }

    /// Create a failed result
    pub fn failure(context: RolloutContext, message: impl Into<String>) -> Self {
        Self {
            success: false,
            context,
            message: Some(message.into()),
        }
    }
}
