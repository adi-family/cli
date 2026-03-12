//! Podman Container Runner Plugin for Hive
//!
//! Executes services in Podman containers.
//!
//! ## Configuration
//!
//! ```yaml
//! runner:
//!   type: podman
//!   podman:
//!     image: postgres:15
//!     ports:
//!       - "{{runtime.port.main}}:5432"
//!     volumes:
//!       - "./data:/var/lib/postgresql/data"
//!     environment:
//!       POSTGRES_PASSWORD: "secret"
//! ```

use lib_plugin_abi_v3::{
    async_trait,
    hooks::HookExitStatus,
    runner::{ProcessHandle, Runner, RuntimeContext},
    Plugin, PluginCategory, PluginContext, PluginMetadata, PluginType, Result as PluginResult,
    SERVICE_RUNNER,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::process::Stdio;
use tokio::process::Command;
use tracing::{debug, info};

pub struct PodmanRunnerPlugin {
    socket: Option<String>,
}

impl Default for PodmanRunnerPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl PodmanRunnerPlugin {
    pub fn new() -> Self {
        Self { socket: None }
    }

    async fn run_podman(&self, args: &[&str]) -> PluginResult<String> {
        let mut cmd = Command::new("podman");

        if let Some(ref socket) = self.socket {
            cmd.arg("--url").arg(socket);
        }

        cmd.args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        debug!("Running: podman {}", args.join(" "));

        let output = cmd
            .output()
            .await
            .map_err(|e| lib_plugin_abi_v3::PluginError::Other(e.into()))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(lib_plugin_abi_v3::PluginError::Other(anyhow::anyhow!(
                "podman command failed: {}",
                stderr
            )));
        }

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    fn extract_config(config: &serde_json::Value) -> PluginResult<PodmanConfig> {
        let podman_value = config.get("podman").ok_or_else(|| {
            lib_plugin_abi_v3::PluginError::Other(anyhow::anyhow!(
                "Missing 'podman' configuration for podman runner"
            ))
        })?;

        serde_json::from_value(podman_value.clone()).map_err(|e| {
            lib_plugin_abi_v3::PluginError::Other(anyhow::anyhow!(
                "Failed to parse podman runner configuration: {}",
                e
            ))
        })
    }
}

#[async_trait]
impl Plugin for PodmanRunnerPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            id: "hive.runner.podman".to_string(),
            name: "Podman Runner".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            plugin_type: PluginType::Orchestration,
            author: Some("ADI Team".to_string()),
            description: Some("Podman container runner plugin".to_string()),
            category: Some(PluginCategory::Runner),
        }
    }

    async fn init(&mut self, ctx: &PluginContext) -> PluginResult<()> {
        if let Some(socket) = ctx.config.get("socket").and_then(|v| v.as_str()) {
            self.socket = Some(socket.to_string());
        }
        Ok(())
    }

    async fn shutdown(&self) -> PluginResult<()> {
        Ok(())
    }

    fn provides(&self) -> Vec<&'static str> {
        vec![SERVICE_RUNNER]
    }
}

#[async_trait]
impl Runner for PodmanRunnerPlugin {
    async fn start(
        &self,
        service_name: &str,
        config: &serde_json::Value,
        env: HashMap<String, String>,
        ctx: &RuntimeContext,
    ) -> PluginResult<ProcessHandle> {
        let podman_config = Self::extract_config(config)?;

        let container_name = format!("hive-{}", service_name);

        let _ = self.run_podman(&["rm", "-f", &container_name]).await;

        let mut args = vec!["run", "-d", "--name", &container_name];

        let interpolated_ports: Vec<String> = podman_config
            .ports
            .iter()
            .map(|p| ctx.interpolate(p))
            .collect::<PluginResult<Vec<_>>>()?;

        for port in &interpolated_ports {
            args.push("-p");
            args.push(port);
        }

        for volume in &podman_config.volumes {
            args.push("-v");
            args.push(volume);
        }

        let env_strings: Vec<String> = podman_config
            .environment
            .iter()
            .map(|(k, v)| {
                let interpolated = ctx.interpolate(v).unwrap_or_else(|_| v.clone());
                format!("{}={}", k, interpolated)
            })
            .collect();

        for env_str in &env_strings {
            args.push("-e");
            args.push(env_str);
        }

        let hive_env_strings: Vec<String> =
            env.iter().map(|(k, v)| format!("{}={}", k, v)).collect();

        for env_str in &hive_env_strings {
            args.push("-e");
            args.push(env_str);
        }

        if let Some(ref network) = podman_config.network_mode {
            args.push("--network");
            args.push(network);
        }

        if let Some(ref restart) = podman_config.restart {
            args.push("--restart");
            args.push(restart);
        }

        args.push(&podman_config.image);

        if let Some(ref command) = podman_config.command {
            for cmd_part in command {
                args.push(cmd_part);
            }
        }

        info!(
            "Creating podman container {} from image {}",
            container_name, podman_config.image
        );

        self.run_podman(&args).await?;

        info!("Container {} started", container_name);

        Ok(ProcessHandle {
            id: format!("podman-{}", container_name),
            runner_type: "podman".to_string(),
            pid: None,
            container_name: Some(container_name),
            metadata: [("image".to_string(), podman_config.image)]
                .into_iter()
                .collect(),
        })
    }

    async fn stop(&self, handle: &ProcessHandle) -> PluginResult<()> {
        let container_name = handle.container_name.as_ref().ok_or_else(|| {
            lib_plugin_abi_v3::PluginError::Other(anyhow::anyhow!(
                "Missing container name in handle"
            ))
        })?;

        info!("Stopping podman container {}", container_name);

        self.run_podman(&["stop", "-t", "10", container_name])
            .await?;

        self.run_podman(&["rm", container_name]).await?;

        info!("Container {} stopped and removed", container_name);
        Ok(())
    }

    async fn is_running(&self, handle: &ProcessHandle) -> bool {
        let Some(container_name) = &handle.container_name else {
            return false;
        };

        match self
            .run_podman(&["inspect", "--format", "{{.State.Running}}", container_name])
            .await
        {
            Ok(output) => output.trim() == "true",
            Err(_) => false,
        }
    }

    async fn logs(&self, handle: &ProcessHandle, lines: Option<usize>) -> PluginResult<Vec<String>> {
        let container_name = handle.container_name.as_ref().ok_or_else(|| {
            lib_plugin_abi_v3::PluginError::Other(anyhow::anyhow!(
                "Missing container name in handle"
            ))
        })?;

        let mut args = vec!["logs"];

        let tail_str;
        if let Some(n) = lines {
            args.push("--tail");
            tail_str = n.to_string();
            args.push(&tail_str);
        }

        args.push(container_name);

        let output = self.run_podman(&args).await?;
        Ok(output.lines().map(String::from).collect())
    }

    fn supports_hooks(&self) -> bool {
        true
    }

    async fn run_hook(
        &self,
        config: &serde_json::Value,
        env: HashMap<String, String>,
        ctx: &RuntimeContext,
    ) -> PluginResult<HookExitStatus> {
        let podman_config = Self::extract_config(config)?;

        let hook_id = uuid::Uuid::new_v4().to_string();
        let container_name = format!("hive-hook-{}", &hook_id[..8]);

        let mut args = vec![
            "run".to_string(),
            "--rm".to_string(),
            "--name".to_string(),
            container_name.clone(),
        ];

        let interpolated_ports: Vec<String> = podman_config
            .ports
            .iter()
            .map(|p| ctx.interpolate(p))
            .collect::<PluginResult<Vec<_>>>()?;

        for port in &interpolated_ports {
            args.push("-p".to_string());
            args.push(port.clone());
        }

        for volume in &podman_config.volumes {
            args.push("-v".to_string());
            args.push(volume.clone());
        }

        for (k, v) in &podman_config.environment {
            let interpolated = ctx.interpolate(v).unwrap_or_else(|_| v.clone());
            args.push("-e".to_string());
            args.push(format!("{}={}", k, interpolated));
        }

        for (k, v) in &env {
            args.push("-e".to_string());
            args.push(format!("{}={}", k, v));
        }

        if let Some(ref network) = podman_config.network_mode {
            args.push("--network".to_string());
            args.push(network.clone());
        }

        args.push(podman_config.image.clone());

        if let Some(ref command) = podman_config.command {
            for cmd_part in command {
                args.push(cmd_part.clone());
            }
        }

        info!(
            "Running hook via podman: {} (image: {})",
            container_name, podman_config.image
        );

        let mut cmd = Command::new("podman");

        if let Some(ref socket) = self.socket {
            cmd.arg("--url").arg(socket);
        }

        let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        cmd.args(&args_refs)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        debug!("Running: podman {}", args.join(" "));

        let output = cmd
            .output()
            .await
            .map_err(|e| lib_plugin_abi_v3::PluginError::Other(e.into()))?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let code = output.status.code().unwrap_or(-1);

        info!(
            "Hook container {} finished with exit code {}",
            container_name, code
        );

        Ok(HookExitStatus {
            code,
            output: if stdout.is_empty() {
                None
            } else {
                Some(stdout)
            },
            stderr: if stderr.is_empty() {
                None
            } else {
                Some(stderr)
            },
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PodmanConfig {
    pub image: String,
    /// Format: `"HOST:CONTAINER"`
    #[serde(default)]
    pub ports: Vec<String>,
    /// Format: `"HOST:CONTAINER"` or `"HOST:CONTAINER:MODE"`
    #[serde(default)]
    pub volumes: Vec<String>,
    #[serde(default)]
    pub environment: HashMap<String, String>,
    pub command: Option<Vec<String>>,
    pub entrypoint: Option<Vec<String>>,
    pub network_mode: Option<String>,
    pub restart: Option<String>,
}

#[cfg(feature = "plugin")]
#[no_mangle]
pub fn plugin_create() -> Box<dyn Plugin> {
    Box::new(PodmanRunnerPlugin::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metadata() {
        let plugin = PodmanRunnerPlugin::new();
        let meta = plugin.metadata();
        assert_eq!(meta.id, "hive.runner.podman");
        assert_eq!(meta.name, "Podman Runner");
    }

    #[test]
    fn test_config_parse() {
        let config = serde_json::json!({
            "image": "postgres:15",
            "ports": ["5432:5432"],
            "environment": {
                "POSTGRES_PASSWORD": "secret"
            }
        });

        let podman_config: PodmanConfig = serde_json::from_value(config).unwrap();
        assert_eq!(podman_config.image, "postgres:15");
        assert_eq!(podman_config.ports.len(), 1);
    }
}
