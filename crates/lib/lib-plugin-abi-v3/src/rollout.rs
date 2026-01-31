//! Rollout strategy plugin trait

use crate::{runner::ProcessHandle, Plugin, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

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
    pub config: Value,
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
    pub config: Value,
}

/// Rollout step result
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
