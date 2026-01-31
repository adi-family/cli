//! Runner Plugin Trait
//!
//! Runners are responsible for starting and managing service processes.
//! Examples: script (shell commands), docker (containers), systemd, etc.
//!
//! Runners can also execute **one-shot hook tasks** via [`RunnerPlugin::run_hook`].
//! Unlike `start()` which creates long-running processes, `run_hook()` executes
//! a task to completion and returns its exit status.

use crate::types::RuntimeContext;
use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Trait for runner plugins
#[async_trait]
pub trait RunnerPlugin: Send + Sync {
    /// Get plugin metadata
    fn metadata(&self) -> crate::PluginMetadata;

    /// Initialize the plugin with default configuration
    async fn init(&mut self, defaults: &serde_json::Value) -> Result<()>;

    /// Start a service
    async fn start(
        &self,
        service_name: &str,
        config: &serde_json::Value,
        env: HashMap<String, String>,
        ctx: &RuntimeContext,
    ) -> Result<ProcessHandle>;

    /// Stop a running service
    async fn stop(&self, handle: &ProcessHandle) -> Result<()>;

    /// Check if a process is still running
    async fn is_running(&self, handle: &ProcessHandle) -> bool;

    /// Get process logs
    async fn logs(&self, handle: &ProcessHandle, lines: Option<usize>) -> Result<Vec<String>>;

    /// Shutdown the plugin
    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    /// Whether this runner supports one-shot hook execution.
    ///
    /// Plugins that return `false` cannot be used in lifecycle hooks.
    /// Hive validates this at config parse time, not at runtime.
    fn supports_hooks(&self) -> bool {
        false
    }

    /// Run a one-shot task for lifecycle hooks.
    ///
    /// Unlike `start()` which creates a long-running process, this method
    /// executes a task to completion and returns its exit status.
    ///
    /// - For script runners: execute command, wait for exit
    /// - For docker runners: `docker run --rm`, wait for container to finish
    /// - For compose runners: `docker compose run --rm`, wait for completion
    ///
    /// The default implementation returns an error indicating the runner
    /// does not support hooks.
    async fn run_hook(
        &self,
        _config: &serde_json::Value,
        _env: HashMap<String, String>,
        _ctx: &RuntimeContext,
    ) -> Result<HookExitStatus> {
        Err(anyhow::anyhow!(
            "Runner '{}' does not support hook execution",
            self.metadata().name
        ))
    }
}

/// Exit status from a one-shot hook execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookExitStatus {
    /// Exit code (0 = success)
    pub code: i32,
    /// Optional output captured from the process
    pub output: Option<String>,
    /// Optional error output captured from the process
    pub stderr: Option<String>,
}

impl HookExitStatus {
    /// Create a successful exit status
    pub fn success() -> Self {
        Self {
            code: 0,
            output: None,
            stderr: None,
        }
    }

    /// Create a failed exit status
    pub fn failed(code: i32) -> Self {
        Self {
            code,
            output: None,
            stderr: None,
        }
    }

    /// Add stdout output
    pub fn with_output(mut self, output: String) -> Self {
        self.output = Some(output);
        self
    }

    /// Add stderr output
    pub fn with_stderr(mut self, stderr: String) -> Self {
        self.stderr = Some(stderr);
        self
    }

    /// Whether the hook succeeded (exit code 0)
    pub fn is_success(&self) -> bool {
        self.code == 0
    }
}

/// Handle for a running process
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessHandle {
    /// Unique identifier for this process
    pub id: String,
    /// Process type (script, docker, etc.)
    pub process_type: String,
    /// Process ID (OS PID for scripts, container ID for docker)
    pub pid: Option<u32>,
    /// Container name (for docker)
    pub container_name: Option<String>,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

impl ProcessHandle {
    /// Create a new process handle for a script
    pub fn script(pid: u32) -> Self {
        Self {
            id: format!("script-{}", pid),
            process_type: "script".to_string(),
            pid: Some(pid),
            container_name: None,
            metadata: HashMap::new(),
        }
    }

    /// Create a new process handle for a docker container
    pub fn docker(container_name: String) -> Self {
        Self {
            id: format!("docker-{}", container_name),
            process_type: "docker".to_string(),
            pid: None,
            container_name: Some(container_name),
            metadata: HashMap::new(),
        }
    }

    /// Add metadata to the handle
    pub fn with_metadata(mut self, key: &str, value: &str) -> Self {
        self.metadata.insert(key.to_string(), value.to_string());
        self
    }
}

/// Process status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProcessStatus {
    /// Process is running
    Running,
    /// Process has stopped with exit code
    Stopped(i32),
    /// Process was killed by signal
    Killed(i32),
    /// Process status is unknown
    Unknown,
}
