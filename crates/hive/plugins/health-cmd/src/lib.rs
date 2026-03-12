//! Command Health Check Plugin for Hive
//!
//! Executes a shell command to check service health.
//!
//! ## Configuration
//!
//! ```yaml
//! healthcheck:
//!   type: cmd
//!   cmd:
//!     command: pg_isready -U adi
//!     working_dir: .
//!     timeout: 5s
//! ```

use async_trait::async_trait;
use lib_plugin_abi_v3::{
    health::{HealthCheck, HealthResult},
    runner::RuntimeContext,
    utils::parse_duration,
    Plugin, PluginCategory, PluginContext, PluginMetadata, PluginType,
    Result as PluginResult, SERVICE_HEALTH_CHECK,
};
use serde::{Deserialize, Serialize};
use std::process::Stdio;
use std::time::{Duration, Instant};
use tokio::process::Command;
use tokio::time::timeout;
use tracing::{debug, trace, warn};


pub struct CmdHealthPlugin {
    default_timeout: Duration,
}

impl Default for CmdHealthPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl CmdHealthPlugin {
    pub fn new() -> Self {
        Self {
            default_timeout: Duration::from_secs(5),
        }
    }
}

#[async_trait]
impl Plugin for CmdHealthPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            id: "hive.health.cmd".to_string(),
            name: "Command Health Check".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            plugin_type: PluginType::Orchestration,
            author: Some("ADI Team".to_string()),
            description: Some("Command-based health check".to_string()),
            category: Some(PluginCategory::Health),
        }
    }

    async fn init(&mut self, ctx: &PluginContext) -> PluginResult<()> {
        if let Some(timeout_str) = ctx.config.get("timeout").and_then(|v| v.as_str()) {
            if let Some(duration) = parse_duration(timeout_str) {
                self.default_timeout = duration;
            }
        }
        Ok(())
    }

    async fn shutdown(&self) -> PluginResult<()> {
        Ok(())
    }

    fn provides(&self) -> Vec<&'static str> {
        vec![SERVICE_HEALTH_CHECK]
    }
}

#[async_trait]
impl HealthCheck for CmdHealthPlugin {
    async fn check(&self, config: &serde_json::Value, ctx: &RuntimeContext) -> PluginResult<HealthResult> {
        let cmd_config: CmdHealthConfig = config
            .get("cmd")
            .ok_or_else(|| lib_plugin_abi_v3::PluginError::Config("Missing 'cmd' configuration".to_string()))?
            .clone()
            .try_into()
            .map_err(|e| lib_plugin_abi_v3::PluginError::Config(format!("Invalid cmd health config: {}", e)))?;

        let command = interpolate_command(&cmd_config.command, ctx)?;

        let check_timeout = cmd_config
            .timeout
            .as_ref()
            .and_then(|t| parse_duration(t))
            .unwrap_or(self.default_timeout);

        let working_dir = if let Some(ref dir) = cmd_config.working_dir {
            if std::path::Path::new(dir).is_absolute() {
                std::path::PathBuf::from(dir)
            } else {
                ctx.working_dir.join(dir)
            }
        } else {
            ctx.working_dir.clone()
        };



        debug!(
            command = %command,
            working_dir = %working_dir.display(),
            timeout_ms = check_timeout.as_millis() as u64,
            "Starting command health check"
        );

        let start = Instant::now();

        let mut cmd = if cfg!(target_os = "windows") {
            let mut c = Command::new("cmd");
            c.args(["/C", &command]);
            c
        } else {
            let mut c = Command::new("sh");
            c.args(["-c", &command]);
            c
        };

        cmd.current_dir(&working_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .env_clear()
            .envs(&ctx.env);

        let result = timeout(check_timeout, cmd.output()).await;

        let elapsed = start.elapsed();

        match result {
            Ok(Ok(output)) => {
                if output.status.success() {
                    trace!(
                        command = %command,
                        elapsed_ms = elapsed.as_millis() as u64,
                        "Command health check passed"
                    );
                    Ok(HealthResult::healthy()
                        .with_response_time(elapsed.as_millis() as u64)
                        .with_message(format!("Command succeeded in {}ms", elapsed.as_millis())))
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    let message = if !stderr.is_empty() {
                        stderr.to_string()
                    } else if !stdout.is_empty() {
                        stdout.to_string()
                    } else {
                        format!("Exit code: {:?}", output.status.code())
                    };
                    warn!(
                        command = %command,
                        exit_code = ?output.status.code(),
                        elapsed_ms = elapsed.as_millis() as u64,
                        "Command health check failed"
                    );
                    Ok(HealthResult::unhealthy(message)
                        .with_response_time(elapsed.as_millis() as u64))
                }
            }
            Ok(Err(e)) => {
                warn!(command = %command, error = %e, "Command health check execution failed");
                Ok(HealthResult::unhealthy(format!("Command failed: {}", e)))
            }
            Err(_) => {
                warn!(command = %command, timeout_s = check_timeout.as_secs(), "Command health check timed out");
                Ok(HealthResult::unhealthy(format!(
                    "Command timed out after {}s",
                    check_timeout.as_secs()
                )))
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CmdHealthConfig {
    pub command: String,
    pub working_dir: Option<String>,
    pub timeout: Option<String>,
}

impl TryFrom<serde_json::Value> for CmdHealthConfig {
    type Error = serde_json::Error;

    fn try_from(value: serde_json::Value) -> Result<Self, Self::Error> {
        serde_json::from_value(value)
    }
}

fn interpolate_command(command: &str, ctx: &RuntimeContext) -> PluginResult<String> {
    ctx.interpolate(command)
}

#[cfg(feature = "plugin")]
#[no_mangle]
pub fn plugin_create() -> Box<dyn Plugin> {
    Box::new(CmdHealthPlugin::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_cmd_success() {
        let plugin = CmdHealthPlugin::new();
        let config = serde_json::json!({
            "cmd": {
                "command": "echo hello"
            }
        });

        let ctx = RuntimeContext::new("test", std::env::current_dir().unwrap());

        let result = plugin.check(&config, &ctx).await.unwrap();
        assert!(result.healthy);
    }

    #[tokio::test]
    async fn test_cmd_failure() {
        let plugin = CmdHealthPlugin::new();
        let config = serde_json::json!({
            "cmd": {
                "command": "exit 1"
            }
        });

        let ctx = RuntimeContext::new("test", std::env::current_dir().unwrap());

        let result = plugin.check(&config, &ctx).await.unwrap();
        assert!(!result.healthy);
    }

}
