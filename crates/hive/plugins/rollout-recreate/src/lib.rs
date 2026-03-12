//! Recreate Rollout Strategy Plugin for Hive
//!
//! **STATUS: PLAN-ONLY**
//!
//! This plugin generates a deployment plan for simple stop-then-start rollouts.
//! The actual step execution is delegated to the Hive daemon. This plugin is a
//! **plan generator only** - it does not execute deployment steps itself.
//!
//! ## How It Works
//!
//! 1. Plugin generates a 3-step deployment plan (stop, start, wait-healthy)
//! 2. Hive daemon receives and executes each step
//! 3. Process management and health checking are handled by the daemon
//!
//! ## Configuration
//!
//! ```yaml
//! rollout:
//!   type: recreate
//!   recreate:
//!     ports:
//!       http: 8080
//!     timeout: 60s  # Optional: health check timeout
//! ```
//!
//! ## Limitations
//!
//! - `execute_step()` delegates to daemon (returns success immediately)
//! - `rollback()` delegates to daemon (no-op in plugin)
//! - Health check timeout is currently hardcoded to 60s in the plan

use anyhow::anyhow;
use lib_plugin_abi_v3::{
    async_trait,
    rollout::{RolloutContext, RolloutStep, RolloutStepResult, RolloutStepType, RolloutStrategy},
    Plugin, PluginCategory, PluginContext, PluginMetadata, PluginType,
    Result as PluginResult, SERVICE_ROLLOUT_STRATEGY,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::debug;

pub struct RecreateRolloutPlugin;

impl Default for RecreateRolloutPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl RecreateRolloutPlugin {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Plugin for RecreateRolloutPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            id: "hive.rollout.recreate".to_string(),
            name: "recreate".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            plugin_type: PluginType::Orchestration,
            author: Some("ADI Team".to_string()),
            description: Some("Stop old instance, start new (plan-only, execution by daemon)".to_string()),
            category: Some(PluginCategory::Rollout),
        }
    }

    async fn init(&mut self, _ctx: &PluginContext) -> PluginResult<()> {
        Ok(())
    }

    async fn shutdown(&self) -> PluginResult<()> {
        Ok(())
    }

    fn provides(&self) -> Vec<&'static str> {
        vec![SERVICE_ROLLOUT_STRATEGY]
    }
}

#[async_trait]
impl RolloutStrategy for RecreateRolloutPlugin {
    async fn plan(&self, config: &serde_json::Value) -> PluginResult<Vec<RolloutStep>> {
        let recreate_config: RecreateConfig = config
            .get("recreate")
            .ok_or_else(|| anyhow!("Missing 'recreate' configuration"))?
            .clone()
            .try_into()
            .map_err(|e| anyhow!("Invalid recreate config: {}", e))?;

        debug!(
            "Planning recreate rollout with ports: {:?}",
            recreate_config.ports
        );

        let mut steps = vec![];

        steps.push(RolloutStep {
            id: "stop-old".to_string(),
            step_type: RolloutStepType::Stop,
            description: "Stop existing instance".to_string(),
            config: serde_json::json!({}),
        });

        steps.push(RolloutStep {
            id: "start-new".to_string(),
            step_type: RolloutStepType::Start,
            description: "Start new instance".to_string(),
            config: serde_json::json!({
                "ports": recreate_config.ports
            }),
        });

        steps.push(RolloutStep {
            id: "wait-healthy".to_string(),
            step_type: RolloutStepType::WaitHealthy,
            description: "Wait for health check".to_string(),
            config: serde_json::json!({
                "timeout": "60s"
            }),
        });

        Ok(steps)
    }

    async fn execute_step(
        &self,
        step: &RolloutStep,
        context: &RolloutContext,
    ) -> PluginResult<RolloutStepResult> {
        debug!(
            "Recreate step '{}' delegated to daemon for service '{}'",
            step.id, context.service_name
        );
        // Plan-generator only: the Hive daemon executes the actual step.
        Ok(RolloutStepResult::success(context.clone()))
    }

    async fn rollback(&self, context: &RolloutContext) -> PluginResult<()> {
        debug!(
            "Recreate rollback for '{}' delegated to daemon",
            context.service_name
        );
        // Plan-generator only: the Hive daemon will attempt to restart the old instance.
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecreateConfig {
    pub ports: HashMap<String, u16>,
}

impl TryFrom<serde_json::Value> for RecreateConfig {
    type Error = serde_json::Error;

    fn try_from(value: serde_json::Value) -> Result<Self, Self::Error> {
        serde_json::from_value(value)
    }
}

#[cfg(feature = "plugin")]
#[no_mangle]
pub fn plugin_create() -> Box<dyn Plugin> {
    Box::new(RecreateRolloutPlugin::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metadata() {
        let plugin = RecreateRolloutPlugin::new();
        let meta = plugin.metadata();
        assert_eq!(meta.id, "hive.rollout.recreate");
        assert_eq!(meta.name, "recreate");
    }

    #[tokio::test]
    async fn test_plan() {
        let plugin = RecreateRolloutPlugin::new();
        let config = serde_json::json!({
            "recreate": {
                "ports": {
                    "http": 8080
                }
            }
        });

        let steps = plugin.plan(&config).await.unwrap();
        assert_eq!(steps.len(), 3);
        assert_eq!(steps[0].id, "stop-old");
        assert_eq!(steps[1].id, "start-new");
        assert_eq!(steps[2].id, "wait-healthy");
    }
}
