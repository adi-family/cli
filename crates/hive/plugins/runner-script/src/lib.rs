//! Script Runner Plugin for Hive
//!
//! Executes services as shell commands, with log capture and proper
//! SIGTERM → SIGKILL shutdown semantics.
//!
//! ## Configuration
//!
//! ```yaml
//! runner:
//!   type: script
//!   script:
//!     run: npm start
//!     working_dir: packages/api     # relative to project root
//!     shell: bash                   # optional, defaults to $SHELL
//! ```

use anyhow::{anyhow, Context as AnyhowContext, Result as AnyhowResult};
use dashmap::DashMap;
use lib_plugin_abi_v3::{
    async_trait,
    runner::{ProcessHandle, Runner, RuntimeContext},
    utils::resolve_shell,
    Plugin, PluginCategory, PluginContext, PluginError, PluginMetadata, PluginType,
    Result as PluginResult, SERVICE_RUNNER,
};
use serde::Deserialize;
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::{Mutex, RwLock};
use tracing::{debug, info, warn};

struct ScriptState {
    pid: u32,
    child: Arc<Mutex<Option<Child>>>,
    logs: Arc<RwLock<Vec<String>>>,
}

pub struct ScriptRunnerPlugin {
    states: Arc<DashMap<String, ScriptState>>,
}

impl Default for ScriptRunnerPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl ScriptRunnerPlugin {
    pub fn new() -> Self {
        Self {
            states: Arc::new(DashMap::new()),
        }
    }

    fn extract_config(config: &serde_json::Value) -> AnyhowResult<ScriptConfig> {
        let script = config
            .get("script")
            .ok_or_else(|| anyhow!("Missing 'script' configuration for script runner"))?;
        serde_json::from_value(script.clone()).context("Failed to parse script runner configuration")
    }

    async fn kill_pid(pid: u32, label: &str) {
        #[cfg(unix)]
        {
            unsafe { libc::kill(pid as i32, libc::SIGTERM) };

            let timeout = std::time::Duration::from_secs(10);
            let start = std::time::Instant::now();
            while start.elapsed() < timeout {
                if unsafe { libc::kill(pid as i32, 0) } != 0 {
                    return;
                }
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }

            warn!("Process {} ({}) did not exit gracefully, sending SIGKILL", label, pid);
            unsafe { libc::kill(pid as i32, libc::SIGKILL) };
        }
    }
}

#[async_trait]
impl Plugin for ScriptRunnerPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            id: "hive.runner.script".to_string(),
            name: "script".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            plugin_type: PluginType::Orchestration,
            author: None,
            description: Some("Execute shell scripts and commands".to_string()),
            category: Some(PluginCategory::Runner),
        }
    }

    async fn init(&mut self, _ctx: &PluginContext) -> PluginResult<()> {
        Ok(())
    }

    async fn shutdown(&self) -> PluginResult<()> {
        let keys: Vec<String> = self.states.iter().map(|e| e.key().clone()).collect();
        for key in keys {
            if let Some((_, state)) = self.states.remove(&key) {
                if let Some(mut child) = state.child.lock().await.take() {
                    let _ = child.kill().await;
                }
            }
        }
        Ok(())
    }

    fn provides(&self) -> Vec<&'static str> {
        vec![SERVICE_RUNNER]
    }
}

#[async_trait]
impl Runner for ScriptRunnerPlugin {
    async fn start(
        &self,
        service_name: &str,
        config: &serde_json::Value,
        env: HashMap<String, String>,
        ctx: &RuntimeContext,
    ) -> PluginResult<ProcessHandle> {
        let script_config = Self::extract_config(config)?;

        // ctx.working_dir is project_root (set by ServiceManager)
        let project_root = &ctx.working_dir;

        let working_dir = script_config
            .working_dir
            .as_deref()
            .map(|d| project_root.join(d))
            .unwrap_or_else(|| project_root.clone());

        let shell_name = script_config
            .shell
            .as_deref()
            .or(ctx.shell.as_deref());
        let shell = resolve_shell(shell_name, false).program;

        // exec replaces the shell so the PID matches the actual process
        let exec_cmd = format!("exec {}", script_config.run);

        let log_dir = project_root.join(".adi/hive/logs");
        std::fs::create_dir_all(&log_dir)
            .with_context(|| format!("Failed to create log directory: {:?}", log_dir))
            .map_err(|e| PluginError::Other(e))?;
        let log_file_path = log_dir.join(format!("{}.log", service_name));

        let log_file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_file_path)
            .with_context(|| format!("Failed to open log file: {:?}", log_file_path))
            .map_err(|e| PluginError::Other(e))?;

        let mut child = Command::new(&shell)
            .args(["-l", "-c"])
            .arg(&exec_cmd)
            .current_dir(&working_dir)
            .envs(env.iter())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .process_group(0)
            .spawn()
            .with_context(|| format!("Failed to spawn process for {}", service_name))
            .map_err(|e| PluginError::Other(e))?;

        let pid = child.id().ok_or_else(|| {
            PluginError::Other(anyhow!("Spawned process has no PID"))
        })?;

        let logs: Arc<RwLock<Vec<String>>> = Arc::new(RwLock::new(Vec::new()));

        if let Some(stdout) = child.stdout.take() {
            let logs_clone = logs.clone();
            let log_file_clone = log_file.try_clone()
                .map_err(|e| PluginError::Other(anyhow!("Failed to clone log file: {}", e)))?;
            tokio::spawn(capture_stream(stdout, logs_clone, log_file_clone));
        }
        if let Some(stderr) = child.stderr.take() {
            let logs_clone = logs.clone();
            let log_file_clone = log_file.try_clone()
                .map_err(|e| PluginError::Other(anyhow!("Failed to clone log file: {}", e)))?;
            tokio::spawn(capture_stream(stderr, logs_clone, log_file_clone));
        }

        info!(
            "Script process started for {} (PID: {}, log: {:?})",
            service_name, pid, log_file_path
        );

        self.states.insert(
            service_name.to_string(),
            ScriptState {
                pid,
                child: Arc::new(Mutex::new(Some(child))),
                logs,
            },
        );

        Ok(ProcessHandle::script(pid).with_metadata("service_name", service_name))
    }

    async fn stop(&self, handle: &ProcessHandle) -> PluginResult<()> {
        let service_name = handle
            .metadata
            .get("service_name")
            .map(|s| s.as_str())
            .unwrap_or(&handle.id);

        if let Some((_, state)) = self.states.remove(service_name) {
            // Managed stop: SIGTERM then wait on child, SIGKILL on timeout
            #[cfg(unix)]
            unsafe {
                libc::kill(state.pid as i32, libc::SIGTERM);
            }

            let timeout = tokio::time::Duration::from_secs(10);
            if let Some(mut child) = state.child.lock().await.take() {
                match tokio::time::timeout(timeout, child.wait()).await {
                    Ok(_) => debug!("Script process {} exited gracefully", service_name),
                    Err(_) => {
                        warn!("Script process {} did not exit in time, sending SIGKILL", service_name);
                        child.kill().await.ok();
                    }
                }
            }

            return Ok(());
        }

        // Unmanaged stop (daemon restart): fall back to PID-based kill
        if let Some(pid) = handle.pid {
            info!("Stopping script process {} by PID {}", service_name, pid);
            Self::kill_pid(pid, service_name).await;
        }

        Ok(())
    }

    async fn is_running(&self, handle: &ProcessHandle) -> bool {
        match handle.pid {
            Some(pid) => {
                #[cfg(unix)]
                {
                    unsafe { libc::kill(pid as i32, 0) == 0 }
                }
                #[cfg(not(unix))]
                {
                    false
                }
            }
            None => false,
        }
    }

    async fn logs(&self, handle: &ProcessHandle, lines: Option<usize>) -> PluginResult<Vec<String>> {
        let service_name = handle
            .metadata
            .get("service_name")
            .map(|s| s.as_str())
            .unwrap_or(&handle.id);

        // In-memory logs (managed case)
        if let Some(state) = self.states.get(service_name) {
            let log_buf = state.logs.read().await;
            return Ok(match lines {
                Some(n) => {
                    let start = log_buf.len().saturating_sub(n);
                    log_buf[start..].to_vec()
                }
                None => log_buf.clone(),
            });
        }

        Ok(Vec::new())
    }
}

/// Reads lines from `reader`, appending to the in-memory log buffer and writing to `log_file`.
async fn capture_stream<R: tokio::io::AsyncRead + Unpin>(
    reader: R,
    logs: Arc<RwLock<Vec<String>>>,
    mut log_file: std::fs::File,
) {
    use std::io::Write;

    const MAX_LOG_LINES: usize = 10_000;
    let mut reader = BufReader::new(reader);
    let mut line = String::new();

    loop {
        line.clear();
        match reader.read_line(&mut line).await {
            Ok(0) => break,
            Ok(_) => {
                let trimmed = line.trim_end();
                if trimmed.is_empty() {
                    continue;
                }
                let _ = writeln!(log_file, "{}", trimmed);
                let mut buf = logs.write().await;
                buf.push(trimmed.to_string());
                if buf.len() > MAX_LOG_LINES {
                    let excess = buf.len() - MAX_LOG_LINES;
                    buf.drain(..excess);
                }
            }
            Err(e) => {
                debug!("Error reading stream: {}", e);
                break;
            }
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct ScriptConfig {
    pub run: String,
    /// Relative to project root.
    pub working_dir: Option<String>,
    /// e.g., "bash", "zsh". Defaults to $SHELL or "sh".
    pub shell: Option<String>,
}

#[cfg(feature = "plugin")]
#[no_mangle]
pub fn plugin_create() -> Box<dyn Plugin> {
    Box::new(ScriptRunnerPlugin::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metadata() {
        let plugin = ScriptRunnerPlugin::new();
        let meta = plugin.metadata();
        assert_eq!(meta.id, "hive.runner.script");
        assert_eq!(meta.name, "script");
    }

    #[test]
    fn test_extract_config() {
        let config = serde_json::json!({
            "script": {
                "run": "npm start",
                "working_dir": "packages/api",
                "shell": "bash"
            }
        });

        let cfg = ScriptRunnerPlugin::extract_config(&config).unwrap();
        assert_eq!(cfg.run, "npm start");
        assert_eq!(cfg.working_dir.as_deref(), Some("packages/api"));
        assert_eq!(cfg.shell.as_deref(), Some("bash"));
    }

    #[test]
    fn test_extract_config_minimal() {
        let config = serde_json::json!({
            "script": { "run": "cargo run" }
        });

        let cfg = ScriptRunnerPlugin::extract_config(&config).unwrap();
        assert_eq!(cfg.run, "cargo run");
        assert!(cfg.working_dir.is_none());
        assert!(cfg.shell.is_none());
    }
}
