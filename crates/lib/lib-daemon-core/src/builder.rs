//! Unified Daemon Builder API
//!
//! Provides a high-level, fluent API for configuring and managing daemons
//! that works consistently across platforms.

use crate::error::{DaemonError, Result};
use crate::ipc_transport::{IpcClient, IpcEndpoint, IpcServer};
use crate::pid::PidFile;
use crate::platform::{is_process_running, kill_process, spawn_background, wait_for_exit, SpawnConfig};
use crate::service::{get_service_manager, RestartPolicy, ServiceConfig, ServiceStatus};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tracing::{info, warn};

/// Builder for creating and managing daemon processes
///
/// Provides a unified API for:
/// - Background process spawning
/// - System service installation (systemd/launchd)
/// - IPC communication
/// - Lifecycle management
///
/// # Example
///
/// ```no_run
/// use lib_daemon_core::DaemonBuilder;
///
/// # async fn example() -> lib_daemon_core::Result<()> {
/// let daemon = DaemonBuilder::new("my-daemon")
///     .executable("/usr/bin/my-daemon")
///     .description("My background service")
///     .working_dir("/var/lib/my-daemon")
///     .log_file("/var/log/my-daemon.log")
///     .env("RUST_LOG", "info")
///     .autostart(true)
///     .build()?;
///
/// // Start as background process
/// daemon.spawn().await?;
///
/// // Or install as system service
/// daemon.install().await?;
/// daemon.start().await?;
///
/// // Check status
/// let status = daemon.status().await?;
/// println!("Status: {:?}", status);
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct DaemonBuilder {
    name: String,
    executable: Option<PathBuf>,
    description: String,
    args: Vec<String>,
    working_dir: Option<PathBuf>,
    env: HashMap<String, String>,
    base_dir: Option<PathBuf>,
    pid_file: Option<PathBuf>,
    socket_path: Option<PathBuf>,
    log_file: Option<PathBuf>,
    error_log_file: Option<PathBuf>,
    restart_policy: RestartPolicy,
    autostart: bool,
    use_service_manager: bool,
}

impl DaemonBuilder {
    /// Create a new daemon builder with the given name
    ///
    /// The name is used as the service identifier and for default file paths.
    pub fn new<S: Into<String>>(name: S) -> Self {
        Self {
            name: name.into(),
            executable: None,
            description: String::new(),
            args: Vec::new(),
            working_dir: None,
            env: HashMap::new(),
            base_dir: None,
            pid_file: None,
            socket_path: None,
            log_file: None,
            error_log_file: None,
            restart_policy: RestartPolicy::OnFailure,
            autostart: false,
            use_service_manager: true,
        }
    }

    /// Set the path to the executable
    pub fn executable<P: Into<PathBuf>>(mut self, path: P) -> Self {
        self.executable = Some(path.into());
        self
    }

    /// Set the service description
    pub fn description<S: Into<String>>(mut self, desc: S) -> Self {
        self.description = desc.into();
        self
    }

    /// Add command-line arguments
    pub fn args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.args.extend(args.into_iter().map(|s| s.into()));
        self
    }

    /// Add a single argument
    pub fn arg<S: Into<String>>(mut self, arg: S) -> Self {
        self.args.push(arg.into());
        self
    }

    /// Set the working directory
    pub fn working_dir<P: Into<PathBuf>>(mut self, dir: P) -> Self {
        self.working_dir = Some(dir.into());
        self
    }

    /// Set the base directory for daemon files (PID, socket, logs)
    ///
    /// If not set, uses `~/.local/share/<name>/` on Unix or `%LOCALAPPDATA%/<name>/` on Windows
    pub fn base_dir<P: Into<PathBuf>>(mut self, dir: P) -> Self {
        self.base_dir = Some(dir.into());
        self
    }

    /// Add an environment variable
    pub fn env<K: Into<String>, V: Into<String>>(mut self, key: K, value: V) -> Self {
        self.env.insert(key.into(), value.into());
        self
    }

    /// Add multiple environment variables
    pub fn envs<I, K, V>(mut self, vars: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<String>,
    {
        for (k, v) in vars {
            self.env.insert(k.into(), v.into());
        }
        self
    }

    /// Set explicit PID file path
    pub fn pid_file<P: Into<PathBuf>>(mut self, path: P) -> Self {
        self.pid_file = Some(path.into());
        self
    }

    /// Set explicit socket path for IPC
    pub fn socket_path<P: Into<PathBuf>>(mut self, path: P) -> Self {
        self.socket_path = Some(path.into());
        self
    }

    /// Set log file path (combined stdout/stderr)
    pub fn log_file<P: Into<PathBuf>>(mut self, path: P) -> Self {
        self.log_file = Some(path.into());
        self
    }

    /// Set separate stdout log file
    pub fn stdout_log<P: Into<PathBuf>>(mut self, path: P) -> Self {
        self.log_file = Some(path.into());
        self
    }

    /// Set separate stderr log file
    pub fn stderr_log<P: Into<PathBuf>>(mut self, path: P) -> Self {
        self.error_log_file = Some(path.into());
        self
    }

    /// Set restart policy
    pub fn restart_policy(mut self, policy: RestartPolicy) -> Self {
        self.restart_policy = policy;
        self
    }

    /// Enable autostart on boot/login
    pub fn autostart(mut self, enabled: bool) -> Self {
        self.autostart = enabled;
        self
    }

    /// Disable system service manager integration (use direct spawn only)
    pub fn no_service_manager(mut self) -> Self {
        self.use_service_manager = false;
        self
    }

    /// Build the daemon handle
    pub fn build(self) -> Result<Daemon> {
        let executable = self.executable.ok_or_else(|| {
            DaemonError::Other(anyhow::anyhow!("Executable path is required"))
        })?;

        // Determine base directory
        let base_dir = self.base_dir.unwrap_or_else(|| {
            dirs::data_local_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(&self.name)
        });

        // Determine file paths
        let pid_file = self.pid_file.unwrap_or_else(|| base_dir.join("daemon.pid"));
        let socket_path = self.socket_path.unwrap_or_else(|| base_dir.join("daemon.sock"));
        let log_file = self.log_file.unwrap_or_else(|| base_dir.join("daemon.log"));
        let error_log_file = self.error_log_file.unwrap_or_else(|| log_file.clone());

        Ok(Daemon {
            name: self.name,
            executable,
            description: self.description,
            args: self.args,
            working_dir: self.working_dir,
            env: self.env,
            base_dir,
            pid_file,
            socket_path,
            log_file,
            error_log_file,
            restart_policy: self.restart_policy,
            autostart: self.autostart,
            use_service_manager: self.use_service_manager,
        })
    }
}

/// Handle for a configured daemon
///
/// Provides methods for controlling the daemon lifecycle.
#[derive(Debug, Clone)]
pub struct Daemon {
    name: String,
    executable: PathBuf,
    description: String,
    args: Vec<String>,
    working_dir: Option<PathBuf>,
    env: HashMap<String, String>,
    base_dir: PathBuf,
    pid_file: PathBuf,
    socket_path: PathBuf,
    log_file: PathBuf,
    error_log_file: PathBuf,
    restart_policy: RestartPolicy,
    autostart: bool,
    use_service_manager: bool,
}

impl Daemon {
    /// Get the daemon name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the base directory
    pub fn base_dir(&self) -> &Path {
        &self.base_dir
    }

    /// Get the PID file path
    pub fn pid_file(&self) -> &Path {
        &self.pid_file
    }

    /// Get the socket path
    pub fn socket_path(&self) -> &Path {
        &self.socket_path
    }

    /// Get the log file path
    pub fn log_file(&self) -> &Path {
        &self.log_file
    }

    /// Ensure base directory exists
    fn ensure_base_dir(&self) -> Result<()> {
        std::fs::create_dir_all(&self.base_dir)?;
        Ok(())
    }

    /// Spawn the daemon as a background process (without service manager)
    ///
    /// This is a direct spawn that detaches from the current process.
    /// Use `install()` + `start()` for proper service manager integration.
    pub async fn spawn(&self) -> Result<u32> {
        self.ensure_base_dir()?;

        if let Some(pid) = self.get_pid()? {
            if is_process_running(pid) {
                return Err(DaemonError::AlreadyRunning(pid));
            }
        }

        let config = SpawnConfig::new(self.executable.display().to_string())
            .args(self.args.clone())
            .stdout(self.log_file.display().to_string())
            .stderr(self.error_log_file.display().to_string())
            .pid_file(self.pid_file.display().to_string());

        let config = if let Some(ref dir) = self.working_dir {
            config.working_dir(dir.display().to_string())
        } else {
            config
        };

        let mut config = config;
        for (key, value) in &self.env {
            config = config.env(key.clone(), value.clone());
        }

        let pid = spawn_background(&config)?;
        info!("Daemon '{}' spawned with PID {}", self.name, pid);
        Ok(pid)
    }

    /// Install the daemon as a system service
    ///
    /// On Linux, creates a systemd user service.
    /// On macOS, creates a launchd user agent.
    pub async fn install(&self) -> Result<()> {
        self.ensure_base_dir()?;

        let config = self.to_service_config();
        let manager = get_service_manager();
        manager.install(&config).await?;
        
        info!("Daemon '{}' installed as system service", self.name);
        Ok(())
    }

    /// Uninstall the daemon from the system service manager
    pub async fn uninstall(&self) -> Result<()> {
        let manager = get_service_manager();
        manager.uninstall(&self.name).await?;
        
        info!("Daemon '{}' uninstalled from system service", self.name);
        Ok(())
    }

    /// Start the daemon
    ///
    /// If installed as a service, uses the service manager.
    /// Otherwise, spawns as a background process.
    pub async fn start(&self) -> Result<()> {
        if self.use_service_manager {
            let manager = get_service_manager();
            if manager.is_installed(&self.name).await? {
                manager.start(&self.name).await?;
                info!("Daemon '{}' started via service manager", self.name);
                return Ok(());
            }
        }

        // Fall back to direct spawn
        self.spawn().await?;
        Ok(())
    }

    /// Stop the daemon
    pub async fn stop(&self) -> Result<()> {
        if self.use_service_manager {
            let manager = get_service_manager();
            if manager.is_installed(&self.name).await? {
                manager.stop(&self.name).await?;
                info!("Daemon '{}' stopped via service manager", self.name);
                return Ok(());
            }
        }

        // Fall back to direct kill
        if let Some(pid) = self.get_pid()? {
            if is_process_running(pid) {
                kill_process(pid)?;
                
                // Wait for process to exit
                if wait_for_exit(pid, Duration::from_secs(10)).await {
                    info!("Daemon '{}' stopped (PID {})", self.name, pid);
                } else {
                    warn!("Daemon '{}' did not exit gracefully", self.name);
                }

                // Clean up PID file
                if self.pid_file.exists() {
                    std::fs::remove_file(&self.pid_file).ok();
                }
            }
        }

        Ok(())
    }

    /// Restart the daemon
    pub async fn restart(&self) -> Result<()> {
        self.stop().await.ok(); // Ignore stop errors
        tokio::time::sleep(Duration::from_millis(500)).await;
        self.start().await
    }

    /// Get the daemon status
    pub async fn status(&self) -> Result<DaemonStatus> {
        if self.use_service_manager {
            let manager = get_service_manager();
            if manager.is_installed(&self.name).await? {
                let status = manager.status(&self.name).await?;
                let autostart = manager.is_autostart_enabled(&self.name).await.unwrap_or(false);
                
                return Ok(DaemonStatus {
                    name: self.name.clone(),
                    state: status,
                    pid_file: self.pid_file.clone(),
                    socket_path: self.socket_path.clone(),
                    log_file: self.log_file.clone(),
                    installed: true,
                    autostart_enabled: autostart,
                });
            }
        }

        let state = if let Some(pid) = self.get_pid()? {
            if is_process_running(pid) {
                ServiceStatus::Running { pid: Some(pid) }
            } else {
                ServiceStatus::Stopped
            }
        } else {
            ServiceStatus::Stopped
        };

        Ok(DaemonStatus {
            name: self.name.clone(),
            state,
            pid_file: self.pid_file.clone(),
            socket_path: self.socket_path.clone(),
            log_file: self.log_file.clone(),
            installed: false,
            autostart_enabled: false,
        })
    }

    /// Check if daemon is running
    pub async fn is_running(&self) -> bool {
        match self.status().await {
            Ok(status) => status.state.is_running(),
            Err(_) => false,
        }
    }

    /// Enable autostart
    pub async fn enable_autostart(&self) -> Result<()> {
        let manager = get_service_manager();
        manager.enable_autostart(&self.name).await
    }

    /// Disable autostart
    pub async fn disable_autostart(&self) -> Result<()> {
        let manager = get_service_manager();
        manager.disable_autostart(&self.name).await
    }

    /// Get recent logs
    pub async fn logs(&self, lines: usize) -> Result<String> {
        if self.use_service_manager {
            let manager = get_service_manager();
            if manager.is_installed(&self.name).await? {
                return manager.logs(&self.name, lines).await;
            }
        }

        if self.log_file.exists() {
            let output = tokio::process::Command::new("tail")
                .args(["-n", &lines.to_string()])
                .arg(&self.log_file)
                .output()
                .await?;
            return Ok(String::from_utf8_lossy(&output.stdout).to_string());
        }

        Ok(String::new())
    }

    /// Create an IPC client to communicate with the daemon
    pub fn ipc_client(&self) -> IpcClient {
        IpcClient::for_path(&self.socket_path)
    }

    /// Create an IPC server (to be used by the daemon process itself)
    pub async fn ipc_server(&self) -> Result<IpcServer> {
        let endpoint = IpcEndpoint::for_path(&self.socket_path);
        IpcServer::bind(endpoint).await
    }

    /// Send a request to the daemon and get a response
    pub async fn request<Req, Resp>(&self, request: &Req) -> Result<Resp>
    where
        Req: Serialize,
        Resp: for<'de> Deserialize<'de>,
    {
        self.ipc_client().request(request).await
    }

    /// Get the PID from the PID file
    fn get_pid(&self) -> Result<Option<u32>> {
        let pid_file = PidFile::new(&self.pid_file);
        pid_file.is_running()
    }

    /// Convert to ServiceConfig for service manager
    fn to_service_config(&self) -> ServiceConfig {
        let mut config = ServiceConfig::new(&self.name, &self.executable)
            .description(&self.description)
            .args(self.args.clone())
            .stdout_log(&self.log_file)
            .stderr_log(&self.error_log_file)
            .restart_policy(self.restart_policy)
            .autostart(self.autostart);

        if let Some(ref dir) = self.working_dir {
            config = config.working_dir(dir.clone());
        }

        for (key, value) in &self.env {
            config = config.env(key.clone(), value.clone());
        }

        config
    }
}

/// Daemon status information
#[derive(Debug, Clone)]
pub struct DaemonStatus {
    /// Daemon name
    pub name: String,
    /// Current state
    pub state: ServiceStatus,
    /// PID file path
    pub pid_file: PathBuf,
    /// Socket path
    pub socket_path: PathBuf,
    /// Log file path
    pub log_file: PathBuf,
    /// Whether installed as system service
    pub installed: bool,
    /// Whether autostart is enabled
    pub autostart_enabled: bool,
}

impl DaemonStatus {
    /// Check if daemon is running
    pub fn is_running(&self) -> bool {
        self.state.is_running()
    }

    /// Get the PID if running
    pub fn pid(&self) -> Option<u32> {
        match &self.state {
            ServiceStatus::Running { pid } => *pid,
            _ => None,
        }
    }
}

/// Convenience function to create a daemon builder
pub fn daemon<S: Into<String>>(name: S) -> DaemonBuilder {
    DaemonBuilder::new(name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_daemon_builder() {
        let temp_dir = TempDir::new().unwrap();
        let base = temp_dir.path();

        let daemon = DaemonBuilder::new("test-daemon")
            .executable("/usr/bin/test")
            .description("Test daemon")
            .args(["--port", "8080"])
            .working_dir("/tmp")
            .base_dir(base)
            .env("FOO", "bar")
            .autostart(true)
            .build()
            .unwrap();

        assert_eq!(daemon.name(), "test-daemon");
        assert_eq!(daemon.base_dir(), base);
        assert_eq!(daemon.pid_file(), base.join("daemon.pid"));
        assert_eq!(daemon.socket_path(), base.join("daemon.sock"));
    }

    #[test]
    fn test_daemon_builder_requires_executable() {
        let result = DaemonBuilder::new("test").build();
        assert!(result.is_err());
    }

    #[test]
    fn test_custom_paths() {
        let daemon = DaemonBuilder::new("test")
            .executable("/bin/test")
            .pid_file("/custom/test.pid")
            .socket_path("/custom/test.sock")
            .log_file("/custom/test.log")
            .build()
            .unwrap();

        assert_eq!(daemon.pid_file(), Path::new("/custom/test.pid"));
        assert_eq!(daemon.socket_path(), Path::new("/custom/test.sock"));
        assert_eq!(daemon.log_file(), Path::new("/custom/test.log"));
    }

    #[tokio::test]
    async fn test_daemon_status_not_running() {
        let temp_dir = TempDir::new().unwrap();

        let daemon = DaemonBuilder::new("test")
            .executable("/bin/false")
            .base_dir(temp_dir.path())
            .no_service_manager()
            .build()
            .unwrap();

        let status = daemon.status().await.unwrap();
        assert!(!status.is_running());
        assert_eq!(status.state, ServiceStatus::Stopped);
    }
}
