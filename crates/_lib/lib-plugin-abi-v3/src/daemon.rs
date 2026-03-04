//! Daemon service traits for plugins
//!
//! Plugins can implement long-running background services and
//! register commands that require daemon execution.

use crate::{Plugin, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Daemon service trait for long-running background services
///
/// Plugins implementing this trait can run as background daemons,
/// managed by `adi daemon`.
///
/// # Example
///
/// ```rust,ignore
/// #[async_trait]
/// impl DaemonService for TasksPlugin {
///     async fn start(&self, ctx: DaemonContext) -> Result<()> {
///         loop {
///             self.process_queue().await?;
///             tokio::time::sleep(Duration::from_secs(1)).await;
///         }
///     }
///
///     async fn stop(&self) -> Result<()> {
///         self.cleanup().await
///     }
///
///     async fn status(&self) -> ServiceStatus {
///         ServiceStatus::Running
///     }
/// }
/// ```
#[async_trait]
pub trait DaemonService: Plugin {
    /// Start the daemon service
    ///
    /// This method should run the main service loop.
    /// It will be called in a background task.
    async fn start(&self, ctx: DaemonContext) -> Result<()>;

    /// Stop the daemon service gracefully
    ///
    /// Called when the daemon is being shut down.
    /// Should clean up resources and exit promptly.
    async fn stop(&self) -> Result<()> {
        Ok(())
    }

    /// Get current service status
    async fn status(&self) -> ServiceStatus {
        ServiceStatus::Unknown
    }

    /// Reload configuration without restart
    async fn reload(&self) -> Result<()> {
        Ok(())
    }
}

/// Daemon execution context
#[derive(Debug, Clone)]
pub struct DaemonContext {
    /// Plugin identifier
    pub plugin_id: String,

    /// Daemon data directory
    pub data_dir: PathBuf,

    /// Daemon config directory
    pub config_dir: PathBuf,

    /// Daemon log directory
    pub log_dir: PathBuf,

    /// PID file path
    pub pid_file: PathBuf,

    /// Socket path for IPC
    pub socket_path: PathBuf,
}

impl DaemonContext {
    /// Create a new daemon context
    pub fn new(
        plugin_id: impl Into<String>,
        data_dir: PathBuf,
        config_dir: PathBuf,
    ) -> Self {
        let plugin_id = plugin_id.into();
        let log_dir = data_dir.join("logs");
        let pid_file = data_dir.join(format!("{}.pid", plugin_id.replace('.', "-")));
        let socket_path = data_dir.join(format!("{}.sock", plugin_id.replace('.', "-")));

        Self {
            plugin_id,
            data_dir,
            config_dir,
            log_dir,
            pid_file,
            socket_path,
        }
    }
}

/// Daemon service status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ServiceStatus {
    /// Service is starting up
    Starting,

    /// Service is running normally
    Running,

    /// Service is stopping
    Stopping,

    /// Service has stopped
    Stopped,

    /// Service encountered an error
    Error,

    /// Service status is unknown
    Unknown,
}

impl ServiceStatus {
    /// Check if service is healthy (starting or running)
    pub fn is_healthy(&self) -> bool {
        matches!(self, ServiceStatus::Starting | ServiceStatus::Running)
    }

    /// Check if service is running
    pub fn is_running(&self) -> bool {
        matches!(self, ServiceStatus::Running)
    }
}

impl std::fmt::Display for ServiceStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ServiceStatus::Starting => write!(f, "starting"),
            ServiceStatus::Running => write!(f, "running"),
            ServiceStatus::Stopping => write!(f, "stopping"),
            ServiceStatus::Stopped => write!(f, "stopped"),
            ServiceStatus::Error => write!(f, "error"),
            ServiceStatus::Unknown => write!(f, "unknown"),
        }
    }
}

/// Daemon command for privileged execution
///
/// Commands are registered at compile time and shown during plugin installation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonCommand {
    /// Command string to execute
    pub command: String,

    /// Whether this command requires sudo/root
    pub requires_sudo: bool,

    /// Description of what this command does
    pub description: Option<String>,
}

impl DaemonCommand {
    /// Create a regular (user-level) daemon command
    pub fn regular(command: impl Into<String>) -> Self {
        Self {
            command: command.into(),
            requires_sudo: false,
            description: None,
        }
    }

    /// Create a sudo (root-level) daemon command
    pub fn sudo(command: impl Into<String>) -> Self {
        Self {
            command: command.into(),
            requires_sudo: true,
            description: None,
        }
    }

    /// Add description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }
}

/// Result of daemon command execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonCommandResult {
    /// Exit code
    pub exit_code: i32,

    /// Standard output
    pub stdout: String,

    /// Standard error
    pub stderr: String,
}

impl DaemonCommandResult {
    /// Check if command succeeded
    pub fn success(&self) -> bool {
        self.exit_code == 0
    }
}

/// Daemon client for executing commands
///
/// Provided to plugins via PluginContext.
#[async_trait]
pub trait DaemonClient: Send + Sync {
    /// Execute a daemon command
    async fn exec(&self, cmd: DaemonCommand) -> Result<DaemonCommandResult>;

    /// Check if daemon is available
    async fn is_available(&self) -> bool;
}

/// Global commands trait for CLI root commands
///
/// Plugins can register commands directly on the `adi` CLI root
/// (e.g., `adi up` instead of `adi hive up`).
#[async_trait]
pub trait GlobalCommands: Plugin {
    /// List all global CLI commands provided by this plugin
    async fn list_global_commands(&self) -> Vec<crate::cli::CliCommand>;

    /// Execute a global CLI command
    async fn run_global_command(&self, ctx: &crate::cli::CliContext) -> Result<crate::cli::CliResult>;
}
