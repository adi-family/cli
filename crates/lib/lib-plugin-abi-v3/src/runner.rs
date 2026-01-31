//! Runner plugin trait for service execution

use crate::{Plugin, Result};
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

/// Runner plugin trait
///
/// Runners are responsible for executing and managing service processes
/// (Docker containers, scripts, systemd services, etc.).
#[async_trait]
pub trait Runner: Plugin {
    /// Start a service
    async fn start(
        &self,
        service_name: &str,
        config: &Value,
        env: HashMap<String, String>,
        ctx: &RuntimeContext,
    ) -> Result<ProcessHandle>;

    /// Stop a running service
    async fn stop(&self, handle: &ProcessHandle) -> Result<()>;

    /// Check if service is running
    async fn is_running(&self, handle: &ProcessHandle) -> bool;

    /// Get service logs
    async fn logs(&self, handle: &ProcessHandle, lines: Option<usize>) -> Result<Vec<String>>;

    /// Check if this runner supports lifecycle hooks
    fn supports_hooks(&self) -> bool {
        false
    }

    /// Run a lifecycle hook (if supported)
    async fn run_hook(
        &self,
        config: &Value,
        env: HashMap<String, String>,
        ctx: &RuntimeContext,
    ) -> Result<HookExitStatus> {
        Err(crate::PluginError::Other(anyhow::anyhow!("Hooks not supported")))
    }
}

/// Process handle
#[derive(Debug, Clone)]
pub struct ProcessHandle {
    /// Process/container identifier
    pub id: String,

    /// Runner type (e.g., "docker", "script")
    pub runner_type: String,

    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

/// Hook exit status
#[derive(Debug, Clone)]
pub struct HookExitStatus {
    /// Whether the hook succeeded
    pub success: bool,

    /// Exit code
    pub exit_code: i32,

    /// Hook output
    pub output: String,
}

/// Runtime context for service execution
#[derive(Debug, Clone)]
pub struct RuntimeContext {
    /// Service name
    pub service_name: String,

    /// Allocated ports (port name -> port number)
    pub ports: HashMap<String, u16>,

    /// Environment variables
    pub env: HashMap<String, String>,

    /// Working directory
    pub working_dir: PathBuf,
}

/// Lifecycle hook type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HookType {
    PreUp,
    PostUp,
    PreDown,
    PostDown,
}

/// Hook failure behavior
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OnFailure {
    Abort,
    Warn,
}