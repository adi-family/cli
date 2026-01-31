//! Runner plugin trait for service execution

use crate::{hooks::HookExitStatus, Plugin, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;

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
        _config: &Value,
        _env: HashMap<String, String>,
        _ctx: &RuntimeContext,
    ) -> Result<HookExitStatus> {
        Err(crate::PluginError::Other(anyhow::anyhow!(
            "Hooks not supported"
        )))
    }
}

/// Process handle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessHandle {
    /// Process/container identifier
    pub id: String,

    /// Runner type (e.g., "docker", "script")
    pub runner_type: String,

    /// Process ID (OS PID for scripts)
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
            runner_type: "script".to_string(),
            pid: Some(pid),
            container_name: None,
            metadata: HashMap::new(),
        }
    }

    /// Create a new process handle for a docker container
    pub fn docker(container_name: String) -> Self {
        Self {
            id: format!("docker-{}", container_name),
            runner_type: "docker".to_string(),
            pid: None,
            container_name: Some(container_name.clone()),
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProcessStatus {
    Running,
    Stopped,
    Failed,
    Unknown,
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

impl RuntimeContext {
    /// Create a new runtime context
    pub fn new(service_name: impl Into<String>, working_dir: PathBuf) -> Self {
        Self {
            service_name: service_name.into(),
            ports: HashMap::new(),
            env: HashMap::new(),
            working_dir,
        }
    }

    /// Add a port mapping
    pub fn with_port(mut self, name: impl Into<String>, port: u16) -> Self {
        self.ports.insert(name.into(), port);
        self
    }

    /// Add an environment variable
    pub fn with_env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.insert(key.into(), value.into());
        self
    }

    /// Interpolate template variables in a string
    ///
    /// Supports: `${PORT:<name>}`, `${ENV:<name>}`, `${SERVICE_NAME}`
    pub fn interpolate(&self, template: &str) -> crate::Result<String> {
        let mut result = template.to_string();

        // Replace ${SERVICE_NAME}
        result = result.replace("${SERVICE_NAME}", &self.service_name);

        // Replace ${PORT:<name>}
        let port_regex = regex::Regex::new(r"\$\{PORT:(\w+)\}").unwrap();
        for cap in port_regex.captures_iter(template) {
            let port_name = &cap[1];
            if let Some(port) = self.ports.get(port_name) {
                result = result.replace(&cap[0], &port.to_string());
            }
        }

        // Replace ${ENV:<name>}
        let env_regex = regex::Regex::new(r"\$\{ENV:(\w+)\}").unwrap();
        for cap in env_regex.captures_iter(template) {
            let env_name = &cap[1];
            if let Some(value) = self.env.get(env_name) {
                result = result.replace(&cap[0], value);
            }
        }

        Ok(result)
    }
}
