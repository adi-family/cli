//! Blue-Green Rollout Strategy Plugin for Hive
//!
//! **STATUS: EXPERIMENTAL / PLAN-ONLY**
//!
//! This plugin generates a deployment plan for zero-downtime blue-green deployments.
//! The actual step execution is delegated to the Hive daemon. This plugin is a
//! **plan generator only** - it does not execute deployment steps itself.
//!
//! ## How It Works
//!
//! 1. Plugin generates a 5-step deployment plan
//! 2. Hive daemon receives and executes each step
//! 3. Traffic switching and rollback are handled by the daemon
//!
//! ## Configuration
//!
//! ```yaml
//! rollout:
//!   type: blue-green
//!   blue-green:
//!     ports:
//!       http:
//!         blue: 8080
//!         green: 8081
//!     healthy_duration: 10s
//!     timeout: 60s
//!     on_failure: keep-old
//! ```
//!
//! ## Limitations
//!
//! - `execute_step()` delegates to daemon (returns success immediately)
//! - `rollback()` delegates to daemon (no-op in plugin)
//! - State tracking (which color is active) is managed by daemon, not plugin

use lib_plugin_abi_v3::{
    async_trait,
    rollout::{RolloutContext, RolloutStep, RolloutStepResult, RolloutStepType, RolloutStrategy},
    Plugin, PluginCategory, PluginContext, PluginMetadata, PluginType,
    Result as PluginResult, SERVICE_ROLLOUT_STRATEGY,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::debug;

pub struct BlueGreenRolloutPlugin;

impl Default for BlueGreenRolloutPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl BlueGreenRolloutPlugin {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Plugin for BlueGreenRolloutPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            id: "hive.rollout.blue-green".to_string(),
            name: "Blue-Green Rollout".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            plugin_type: PluginType::Orchestration,
            author: Some("ADI Team".to_string()),
            description: Some("Zero-downtime blue-green deployment strategy (plan-only, execution by daemon)".to_string()),
            category: Some(PluginCategory::Rollout),
        }
    }

    async fn init(&mut self, _ctx: &PluginContext) -> PluginResult<()> {
        debug!("Blue-green rollout plugin initialized");
        Ok(())
    }

    async fn shutdown(&self) -> PluginResult<()> {
        debug!("Blue-green rollout plugin shutting down");
        Ok(())
    }

    fn provides(&self) -> Vec<&'static str> {
        vec![SERVICE_ROLLOUT_STRATEGY]
    }
}

#[async_trait]
impl RolloutStrategy for BlueGreenRolloutPlugin {
    async fn plan(&self, config: &serde_json::Value) -> PluginResult<Vec<RolloutStep>> {
        let bg_config: BlueGreenConfig = config
            .get("blue-green")
            .ok_or_else(|| lib_plugin_abi_v3::PluginError::Config("Missing 'blue-green' configuration".to_string()))?
            .clone()
            .try_into()
            .map_err(|e| lib_plugin_abi_v3::PluginError::Config(format!("Invalid blue-green config: {}", e)))?;

        debug!(
            "Planning blue-green rollout with ports: {:?}",
            bg_config.ports
        );

        let healthy_duration = bg_config.healthy_duration.as_deref().unwrap_or("10s");
        let timeout = bg_config.timeout.as_deref().unwrap_or("60s");

        let mut steps = vec![];

        steps.push(RolloutStep {
            id: "start-new".to_string(),
            step_type: RolloutStepType::Start,
            description: "Start new instance on inactive port".to_string(),
            config: serde_json::json!({
                "ports": bg_config.ports,
                "use_inactive": true
            }),
        });

        steps.push(RolloutStep {
            id: "wait-healthy".to_string(),
            step_type: RolloutStepType::WaitHealthy,
            description: "Wait for health check".to_string(),
            config: serde_json::json!({
                "timeout": timeout
            }),
        });

        steps.push(RolloutStep {
            id: "wait-stable".to_string(),
            step_type: RolloutStepType::Wait,
            description: format!("Wait {} to confirm stability", healthy_duration),
            config: serde_json::json!({
                "duration": healthy_duration
            }),
        });

        steps.push(RolloutStep {
            id: "switch-traffic".to_string(),
            step_type: RolloutStepType::SwitchTraffic,
            description: "Switch traffic to new instance".to_string(),
            config: serde_json::json!({}),
        });

        steps.push(RolloutStep {
            id: "stop-old".to_string(),
            step_type: RolloutStepType::Stop,
            description: "Stop old instance".to_string(),
            config: serde_json::json!({
                "target": "old"
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
            "Blue-green step '{}' delegated to daemon for service '{}'",
            step.id, context.service_name
        );
        // Plan-generator only: the Hive daemon executes the actual step.
        Ok(RolloutStepResult::success(context.clone()))
    }

    async fn rollback(&self, context: &RolloutContext) -> PluginResult<()> {
        debug!(
            "Blue-green rollback for '{}' delegated to daemon",
            context.service_name
        );
        // Plan-generator only: the Hive daemon performs the rollback
        // (switch traffic back to old instance, stop new instance).
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlueGreenConfig {
    pub ports: HashMap<String, BlueGreenPort>,
    pub healthy_duration: Option<String>,
    pub timeout: Option<String>,
    /// Action on failure: `"keep-old"` or `"abort"`
    pub on_failure: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlueGreenPort {
    pub blue: u16,
    pub green: u16,
}

impl TryFrom<serde_json::Value> for BlueGreenConfig {
    type Error = serde_json::Error;

    fn try_from(value: serde_json::Value) -> Result<Self, Self::Error> {
        serde_json::from_value(value)
    }
}

#[cfg(feature = "plugin")]
#[no_mangle]
pub fn plugin_create() -> Box<dyn Plugin> {
    Box::new(BlueGreenRolloutPlugin::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metadata() {
        let plugin = BlueGreenRolloutPlugin::new();
        let meta = plugin.metadata();
        assert_eq!(meta.id, "hive.rollout.blue-green");
        assert_eq!(meta.name, "Blue-Green Rollout");
    }

    #[tokio::test]
    async fn test_plan() {
        let plugin = BlueGreenRolloutPlugin::new();
        let config = serde_json::json!({
            "blue-green": {
                "ports": {
                    "http": {
                        "blue": 8080,
                        "green": 8081
                    }
                },
                "healthy_duration": "10s"
            }
        });

        let steps = plugin.plan(&config).await.unwrap();
        assert_eq!(steps.len(), 5);
        assert_eq!(steps[0].id, "start-new");
        assert_eq!(steps[1].id, "wait-healthy");
        assert_eq!(steps[2].id, "wait-stable");
        assert_eq!(steps[3].id, "switch-traffic");
        assert_eq!(steps[4].id, "stop-old");
    }

    #[test]
    fn test_config_parse() {
        let config = serde_json::json!({
            "ports": {
                "http": {
                    "blue": 8080,
                    "green": 8081
                }
            },
            "healthy_duration": "10s",
            "on_failure": "keep-old"
        });

        let bg_config: BlueGreenConfig = serde_json::from_value(config).unwrap();
        assert_eq!(bg_config.ports.get("http").unwrap().blue, 8080);
        assert_eq!(bg_config.ports.get("http").unwrap().green, 8081);
    }
}
