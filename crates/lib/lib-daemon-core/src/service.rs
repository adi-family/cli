//! Service manager abstraction for system service integration
//!
//! Provides a unified interface for managing system services across platforms:
//! - Linux: systemd (user services)
//! - macOS: launchd (user agents)
//! - Windows: Windows Services (future)

use crate::error::{DaemonError, Result};
use crate::platform::Platform;
use async_trait::async_trait;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};

/// Service status
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServiceStatus {
    /// Service is running with the given PID
    Running { pid: Option<u32> },
    /// Service is stopped
    Stopped,
    /// Service is starting
    Starting,
    /// Service is stopping
    Stopping,
    /// Service status is unknown
    Unknown,
    /// Service is not installed
    NotInstalled,
    /// Service failed
    Failed { reason: Option<String> },
}

impl ServiceStatus {
    /// Check if service is running
    pub fn is_running(&self) -> bool {
        matches!(self, ServiceStatus::Running { .. })
    }

    /// Check if service is stopped
    pub fn is_stopped(&self) -> bool {
        matches!(self, ServiceStatus::Stopped | ServiceStatus::NotInstalled)
    }
}

/// Service configuration
#[derive(Debug, Clone)]
pub struct ServiceConfig {
    /// Unique service identifier (e.g., "adi-hive")
    pub name: String,
    /// Human-readable description
    pub description: String,
    /// Path to the executable
    pub executable: PathBuf,
    /// Command-line arguments
    pub args: Vec<String>,
    /// Working directory
    pub working_dir: Option<PathBuf>,
    /// Environment variables
    pub env: HashMap<String, String>,
    /// Path to stdout log file
    pub stdout_log: Option<PathBuf>,
    /// Path to stderr log file
    pub stderr_log: Option<PathBuf>,
    /// Restart policy
    pub restart_policy: RestartPolicy,
    /// Start on boot/login
    pub autostart: bool,
}

impl ServiceConfig {
    /// Create a new service configuration
    pub fn new<S: Into<String>, P: Into<PathBuf>>(name: S, executable: P) -> Self {
        Self {
            name: name.into(),
            description: String::new(),
            executable: executable.into(),
            args: Vec::new(),
            working_dir: None,
            env: HashMap::new(),
            stdout_log: None,
            stderr_log: None,
            restart_policy: RestartPolicy::OnFailure,
            autostart: false,
        }
    }

    /// Set service description
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

    /// Set working directory
    pub fn working_dir<P: Into<PathBuf>>(mut self, dir: P) -> Self {
        self.working_dir = Some(dir.into());
        self
    }

    /// Add environment variable
    pub fn env<K: Into<String>, V: Into<String>>(mut self, key: K, value: V) -> Self {
        self.env.insert(key.into(), value.into());
        self
    }

    /// Set stdout log file
    pub fn stdout_log<P: Into<PathBuf>>(mut self, path: P) -> Self {
        self.stdout_log = Some(path.into());
        self
    }

    /// Set stderr log file
    pub fn stderr_log<P: Into<PathBuf>>(mut self, path: P) -> Self {
        self.stderr_log = Some(path.into());
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
}

/// Service restart policy
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RestartPolicy {
    /// Never restart
    Never,
    /// Restart on failure
    #[default]
    OnFailure,
    /// Always restart
    Always,
}

/// Trait for platform-specific service managers
#[async_trait]
pub trait ServiceManager: Send + Sync {
    /// Install/register the service
    async fn install(&self, config: &ServiceConfig) -> Result<()>;

    /// Uninstall/remove the service
    async fn uninstall(&self, name: &str) -> Result<()>;

    /// Start the service
    async fn start(&self, name: &str) -> Result<()>;

    /// Stop the service
    async fn stop(&self, name: &str) -> Result<()>;

    /// Restart the service
    async fn restart(&self, name: &str) -> Result<()> {
        self.stop(name).await.ok(); // Ignore stop errors
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        self.start(name).await
    }

    /// Get service status
    async fn status(&self, name: &str) -> Result<ServiceStatus>;

    /// Enable autostart
    async fn enable_autostart(&self, name: &str) -> Result<()>;

    /// Disable autostart
    async fn disable_autostart(&self, name: &str) -> Result<()>;

    /// Check if autostart is enabled
    async fn is_autostart_enabled(&self, name: &str) -> Result<bool>;

    /// Get service logs
    async fn logs(&self, name: &str, lines: usize) -> Result<String>;

    /// Check if service is installed
    async fn is_installed(&self, name: &str) -> Result<bool>;
}

/// Get the appropriate service manager for the current platform
pub fn get_service_manager() -> Box<dyn ServiceManager> {
    match Platform::current() {
        Platform::Linux => Box::new(SystemdManager::new()),
        Platform::MacOS => Box::new(LaunchdManager::new()),
        Platform::Windows => Box::new(WindowsServiceManager::new()),
        Platform::Unknown => Box::new(FallbackManager::new()),
    }
}

// ============================================================================
// systemd (Linux)
// ============================================================================

/// systemd service manager for Linux
pub struct SystemdManager {
    /// Use user services (systemctl --user)
    user_mode: bool,
}

impl SystemdManager {
    /// Create a new systemd manager (user mode by default)
    pub fn new() -> Self {
        Self { user_mode: true }
    }

    /// Create a system-level manager (requires root)
    pub fn system() -> Self {
        Self { user_mode: false }
    }

    fn service_dir(&self) -> PathBuf {
        if self.user_mode {
            dirs::config_dir()
                .unwrap_or_else(|| PathBuf::from("~/.config"))
                .join("systemd/user")
        } else {
            PathBuf::from("/etc/systemd/system")
        }
    }

    fn service_path(&self, name: &str) -> PathBuf {
        self.service_dir().join(format!("{}.service", name))
    }

    fn systemctl_args(&self) -> Vec<&str> {
        if self.user_mode {
            vec!["--user"]
        } else {
            vec![]
        }
    }

    fn generate_unit(&self, config: &ServiceConfig) -> String {
        let mut unit = String::new();

        unit.push_str("[Unit]\n");
        unit.push_str(&format!("Description={}\n", config.description));
        unit.push_str("After=network.target\n\n");

        unit.push_str("[Service]\n");
        unit.push_str("Type=simple\n");

        let mut exec_start = config.executable.display().to_string();
        for arg in &config.args {
            exec_start.push(' ');
            // Quote args with spaces
            if arg.contains(' ') {
                exec_start.push('"');
                exec_start.push_str(arg);
                exec_start.push('"');
            } else {
                exec_start.push_str(arg);
            }
        }
        unit.push_str(&format!("ExecStart={}\n", exec_start));

        if let Some(ref dir) = config.working_dir {
            unit.push_str(&format!("WorkingDirectory={}\n", dir.display()));
        }

        for (key, value) in &config.env {
            unit.push_str(&format!("Environment=\"{}={}\"\n", key, value));
        }

        match config.restart_policy {
            RestartPolicy::Never => unit.push_str("Restart=no\n"),
            RestartPolicy::OnFailure => {
                unit.push_str("Restart=on-failure\n");
                unit.push_str("RestartSec=5\n");
            }
            RestartPolicy::Always => {
                unit.push_str("Restart=always\n");
                unit.push_str("RestartSec=5\n");
            }
        }

        if let Some(ref stdout) = config.stdout_log {
            unit.push_str(&format!("StandardOutput=append:{}\n", stdout.display()));
        }
        if let Some(ref stderr) = config.stderr_log {
            unit.push_str(&format!("StandardError=append:{}\n", stderr.display()));
        }

        unit.push('\n');

        unit.push_str("[Install]\n");
        unit.push_str("WantedBy=default.target\n");

        unit
    }
}

impl Default for SystemdManager {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ServiceManager for SystemdManager {
    async fn install(&self, config: &ServiceConfig) -> Result<()> {
        let service_path = self.service_path(&config.name);

        if let Some(parent) = service_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let unit_content = self.generate_unit(config);
        std::fs::write(&service_path, &unit_content)?;
        info!("Created systemd unit: {}", service_path.display());

        let mut args = self.systemctl_args();
        args.push("daemon-reload");
        
        let output = tokio::process::Command::new("systemctl")
            .args(&args)
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(DaemonError::Other(anyhow::anyhow!(
                "Failed to reload systemd: {}",
                stderr
            )));
        }

        if config.autostart {
            self.enable_autostart(&config.name).await?;
        }

        Ok(())
    }

    async fn uninstall(&self, name: &str) -> Result<()> {
        self.stop(name).await.ok();
        self.disable_autostart(name).await.ok();

        let service_path = self.service_path(name);
        if service_path.exists() {
            std::fs::remove_file(&service_path)?;
            info!("Removed systemd unit: {}", service_path.display());
        }

        // Reload systemd
        let mut args = self.systemctl_args();
        args.push("daemon-reload");
        
        tokio::process::Command::new("systemctl")
            .args(&args)
            .output()
            .await?;

        Ok(())
    }

    async fn start(&self, name: &str) -> Result<()> {
        let mut args = self.systemctl_args();
        args.push("start");
        args.push(name);

        let output = tokio::process::Command::new("systemctl")
            .args(&args)
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(DaemonError::Other(anyhow::anyhow!(
                "Failed to start service: {}",
                stderr
            )));
        }

        info!("Started service: {}", name);
        Ok(())
    }

    async fn stop(&self, name: &str) -> Result<()> {
        let mut args = self.systemctl_args();
        args.push("stop");
        args.push(name);

        let output = tokio::process::Command::new("systemctl")
            .args(&args)
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(DaemonError::Other(anyhow::anyhow!(
                "Failed to stop service: {}",
                stderr
            )));
        }

        info!("Stopped service: {}", name);
        Ok(())
    }

    async fn status(&self, name: &str) -> Result<ServiceStatus> {
        let mut args = self.systemctl_args();
        args.push("show");
        args.push("--property=ActiveState,SubState,MainPID");
        args.push(name);

        let output = tokio::process::Command::new("systemctl")
            .args(&args)
            .output()
            .await?;

        let stdout = String::from_utf8_lossy(&output.stdout);

        // Parse output
        let mut active_state = String::new();
        let mut main_pid: Option<u32> = None;

        for line in stdout.lines() {
            if let Some(value) = line.strip_prefix("ActiveState=") {
                active_state = value.to_string();
            } else if let Some(value) = line.strip_prefix("MainPID=") {
                main_pid = value.parse().ok().filter(|&pid| pid > 0);
            }
        }

        match active_state.as_str() {
            "active" => Ok(ServiceStatus::Running { pid: main_pid }),
            "inactive" => Ok(ServiceStatus::Stopped),
            "activating" => Ok(ServiceStatus::Starting),
            "deactivating" => Ok(ServiceStatus::Stopping),
            "failed" => Ok(ServiceStatus::Failed { reason: None }),
            "" => {
                // Check if service file exists
                if self.service_path(name).exists() {
                    Ok(ServiceStatus::Stopped)
                } else {
                    Ok(ServiceStatus::NotInstalled)
                }
            }
            _ => Ok(ServiceStatus::Unknown),
        }
    }

    async fn enable_autostart(&self, name: &str) -> Result<()> {
        let mut args = self.systemctl_args();
        args.push("enable");
        args.push(name);

        let output = tokio::process::Command::new("systemctl")
            .args(&args)
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(DaemonError::Other(anyhow::anyhow!(
                "Failed to enable autostart: {}",
                stderr
            )));
        }

        info!("Enabled autostart for: {}", name);
        Ok(())
    }

    async fn disable_autostart(&self, name: &str) -> Result<()> {
        let mut args = self.systemctl_args();
        args.push("disable");
        args.push(name);

        let output = tokio::process::Command::new("systemctl")
            .args(&args)
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!("Failed to disable autostart: {}", stderr);
        }

        Ok(())
    }

    async fn is_autostart_enabled(&self, name: &str) -> Result<bool> {
        let mut args = self.systemctl_args();
        args.push("is-enabled");
        args.push(name);

        let output = tokio::process::Command::new("systemctl")
            .args(&args)
            .output()
            .await?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(stdout.trim() == "enabled")
    }

    async fn logs(&self, name: &str, lines: usize) -> Result<String> {
        let mut args = self.systemctl_args();
        let lines_str = lines.to_string();
        args.extend(["--unit", name, "-n", &lines_str, "--no-pager"]);

        let output = tokio::process::Command::new("journalctl")
            .args(&args)
            .output()
            .await?;

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    async fn is_installed(&self, name: &str) -> Result<bool> {
        Ok(self.service_path(name).exists())
    }
}

// ============================================================================
// launchd (macOS)
// ============================================================================

/// launchd service manager for macOS
pub struct LaunchdManager {
    /// Domain (user or system)
    user_mode: bool,
}

impl LaunchdManager {
    /// Create a new launchd manager (user mode by default)
    pub fn new() -> Self {
        Self { user_mode: true }
    }

    /// Create a system-level manager (requires root)
    pub fn system() -> Self {
        Self { user_mode: false }
    }

    fn plist_dir(&self) -> PathBuf {
        if self.user_mode {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("~"))
                .join("Library/LaunchAgents")
        } else {
            PathBuf::from("/Library/LaunchDaemons")
        }
    }

    fn plist_path(&self, name: &str) -> PathBuf {
        let label = self.service_label(name);
        self.plist_dir().join(format!("{}.plist", label))
    }

    fn service_label(&self, name: &str) -> String {
        format!("com.adi.{}", name)
    }

    fn generate_plist(&self, config: &ServiceConfig) -> String {
        let label = self.service_label(&config.name);
        let mut plist = String::new();

        plist.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        plist.push_str("<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n");
        plist.push_str("<plist version=\"1.0\">\n");
        plist.push_str("<dict>\n");

        // Label
        plist.push_str("    <key>Label</key>\n");
        plist.push_str(&format!("    <string>{}</string>\n", label));

        // Program and arguments
        plist.push_str("    <key>ProgramArguments</key>\n");
        plist.push_str("    <array>\n");
        plist.push_str(&format!(
            "        <string>{}</string>\n",
            config.executable.display()
        ));
        for arg in &config.args {
            plist.push_str(&format!("        <string>{}</string>\n", arg));
        }
        plist.push_str("    </array>\n");

        // Working directory
        if let Some(ref dir) = config.working_dir {
            plist.push_str("    <key>WorkingDirectory</key>\n");
            plist.push_str(&format!("    <string>{}</string>\n", dir.display()));
        }

        // Environment variables
        if !config.env.is_empty() {
            plist.push_str("    <key>EnvironmentVariables</key>\n");
            plist.push_str("    <dict>\n");
            for (key, value) in &config.env {
                plist.push_str(&format!("        <key>{}</key>\n", key));
                plist.push_str(&format!("        <string>{}</string>\n", value));
            }
            plist.push_str("    </dict>\n");
        }

        // Stdout log
        if let Some(ref stdout) = config.stdout_log {
            plist.push_str("    <key>StandardOutPath</key>\n");
            plist.push_str(&format!("    <string>{}</string>\n", stdout.display()));
        }

        // Stderr log
        if let Some(ref stderr) = config.stderr_log {
            plist.push_str("    <key>StandardErrorPath</key>\n");
            plist.push_str(&format!("    <string>{}</string>\n", stderr.display()));
        }

        // Restart policy
        match config.restart_policy {
            RestartPolicy::Never => {}
            RestartPolicy::OnFailure | RestartPolicy::Always => {
                plist.push_str("    <key>KeepAlive</key>\n");
                if config.restart_policy == RestartPolicy::Always {
                    plist.push_str("    <true/>\n");
                } else {
                    plist.push_str("    <dict>\n");
                    plist.push_str("        <key>SuccessfulExit</key>\n");
                    plist.push_str("        <false/>\n");
                    plist.push_str("    </dict>\n");
                }
            }
        }

        // Run at load (autostart)
        plist.push_str("    <key>RunAtLoad</key>\n");
        if config.autostart {
            plist.push_str("    <true/>\n");
        } else {
            plist.push_str("    <false/>\n");
        }

        plist.push_str("</dict>\n");
        plist.push_str("</plist>\n");

        plist
    }
}

impl Default for LaunchdManager {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ServiceManager for LaunchdManager {
    async fn install(&self, config: &ServiceConfig) -> Result<()> {
        let plist_path = self.plist_path(&config.name);

        if let Some(parent) = plist_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let plist_content = self.generate_plist(config);
        std::fs::write(&plist_path, &plist_content)?;
        info!("Created launchd plist: {}", plist_path.display());

        Ok(())
    }

    async fn uninstall(&self, name: &str) -> Result<()> {
        self.stop(name).await.ok();

        let plist_path = self.plist_path(name);
        if plist_path.exists() {
            std::fs::remove_file(&plist_path)?;
            info!("Removed launchd plist: {}", plist_path.display());
        }

        Ok(())
    }

    async fn start(&self, name: &str) -> Result<()> {
        let plist_path = self.plist_path(name);

        // launchctl load (or bootstrap for newer macOS)
        let output = tokio::process::Command::new("launchctl")
            .args(["load", "-w"])
            .arg(&plist_path)
            .output()
            .await?;

        if !output.status.success() {
            // Try newer bootstrap command
            let domain = if self.user_mode {
                format!("gui/{}", unsafe { libc::getuid() })
            } else {
                "system".to_string()
            };

            let output = tokio::process::Command::new("launchctl")
                .args(["bootstrap", &domain])
                .arg(&plist_path)
                .output()
                .await?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                // Ignore "already loaded" error
                if !stderr.contains("already loaded") && !stderr.contains("service already loaded") {
                    return Err(DaemonError::Other(anyhow::anyhow!(
                        "Failed to start service: {}",
                        stderr
                    )));
                }
            }
        }

        info!("Started service: {}", name);
        Ok(())
    }

    async fn stop(&self, name: &str) -> Result<()> {
        let plist_path = self.plist_path(name);

        // launchctl unload
        let output = tokio::process::Command::new("launchctl")
            .args(["unload"])
            .arg(&plist_path)
            .output()
            .await?;

        if !output.status.success() {
            // Try newer bootout command
            let label = self.service_label(name);
            let domain = if self.user_mode {
                format!("gui/{}", unsafe { libc::getuid() })
            } else {
                "system".to_string()
            };

            let output = tokio::process::Command::new("launchctl")
                .args(["bootout", &format!("{}/{}", domain, label)])
                .output()
                .await?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                // Ignore "not running" errors
                if !stderr.contains("not running") && !stderr.contains("No such process") {
                    debug!("Failed to stop service: {}", stderr);
                }
            }
        }

        info!("Stopped service: {}", name);
        Ok(())
    }

    async fn status(&self, name: &str) -> Result<ServiceStatus> {
        let label = self.service_label(name);

        // launchctl list
        let output = tokio::process::Command::new("launchctl")
            .args(["list", &label])
            .output()
            .await?;

        if !output.status.success() {
            // Check if plist exists
            if self.plist_path(name).exists() {
                return Ok(ServiceStatus::Stopped);
            }
            return Ok(ServiceStatus::NotInstalled);
        }

        // Parse output for PID
        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut pid: Option<u32> = None;

        for line in stdout.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                if let Ok(p) = parts[0].parse::<u32>() {
                    if p > 0 {
                        pid = Some(p);
                        break;
                    }
                }
            }
        }

        if pid.is_some() {
            Ok(ServiceStatus::Running { pid })
        } else {
            Ok(ServiceStatus::Stopped)
        }
    }

    async fn enable_autostart(&self, name: &str) -> Result<()> {
        // In launchd, this is done by setting RunAtLoad in the plist
        // We need to modify the plist file
        let plist_path = self.plist_path(name);
        if !plist_path.exists() {
            return Err(DaemonError::NotRunning);
        }

        // Read plist, modify RunAtLoad, write back
        let content = std::fs::read_to_string(&plist_path)?;
        let modified = content.replace(
            "<key>RunAtLoad</key>\n    <false/>",
            "<key>RunAtLoad</key>\n    <true/>",
        );
        std::fs::write(&plist_path, &modified)?;

        info!("Enabled autostart for: {}", name);
        Ok(())
    }

    async fn disable_autostart(&self, name: &str) -> Result<()> {
        let plist_path = self.plist_path(name);
        if !plist_path.exists() {
            return Ok(());
        }

        let content = std::fs::read_to_string(&plist_path)?;
        let modified = content.replace(
            "<key>RunAtLoad</key>\n    <true/>",
            "<key>RunAtLoad</key>\n    <false/>",
        );
        std::fs::write(&plist_path, &modified)?;

        info!("Disabled autostart for: {}", name);
        Ok(())
    }

    async fn is_autostart_enabled(&self, name: &str) -> Result<bool> {
        let plist_path = self.plist_path(name);
        if !plist_path.exists() {
            return Ok(false);
        }

        let content = std::fs::read_to_string(&plist_path)?;
        Ok(content.contains("<key>RunAtLoad</key>\n    <true/>"))
    }

    async fn logs(&self, name: &str, lines: usize) -> Result<String> {
        // launchd doesn't have built-in log access
        // Try reading from the stdout log file if configured
        let plist_path = self.plist_path(name);
        if !plist_path.exists() {
            return Ok(String::new());
        }

        // Parse plist for StandardOutPath
        let content = std::fs::read_to_string(&plist_path)?;
        if let Some(start) = content.find("<key>StandardOutPath</key>") {
            if let Some(string_start) = content[start..].find("<string>") {
                if let Some(string_end) = content[start + string_start..].find("</string>") {
                    let path_start = start + string_start + 8;
                    let path_end = start + string_start + string_end;
                    let log_path = &content[path_start..path_end];

                    if Path::new(log_path).exists() {
                        let output = tokio::process::Command::new("tail")
                            .args(["-n", &lines.to_string(), log_path])
                            .output()
                            .await?;
                        return Ok(String::from_utf8_lossy(&output.stdout).to_string());
                    }
                }
            }
        }

        Ok(String::from("Log file not configured or not found"))
    }

    async fn is_installed(&self, name: &str) -> Result<bool> {
        Ok(self.plist_path(name).exists())
    }
}

// ============================================================================
// Windows Service Manager (stub)
// ============================================================================

/// Windows service manager (placeholder for future implementation)
pub struct WindowsServiceManager;

impl WindowsServiceManager {
    pub fn new() -> Self {
        Self
    }
}

impl Default for WindowsServiceManager {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ServiceManager for WindowsServiceManager {
    async fn install(&self, _config: &ServiceConfig) -> Result<()> {
        Err(DaemonError::Other(anyhow::anyhow!(
            "Windows service management not yet implemented"
        )))
    }

    async fn uninstall(&self, _name: &str) -> Result<()> {
        Err(DaemonError::Other(anyhow::anyhow!(
            "Windows service management not yet implemented"
        )))
    }

    async fn start(&self, _name: &str) -> Result<()> {
        Err(DaemonError::Other(anyhow::anyhow!(
            "Windows service management not yet implemented"
        )))
    }

    async fn stop(&self, _name: &str) -> Result<()> {
        Err(DaemonError::Other(anyhow::anyhow!(
            "Windows service management not yet implemented"
        )))
    }

    async fn status(&self, _name: &str) -> Result<ServiceStatus> {
        Ok(ServiceStatus::Unknown)
    }

    async fn enable_autostart(&self, _name: &str) -> Result<()> {
        Err(DaemonError::Other(anyhow::anyhow!(
            "Windows service management not yet implemented"
        )))
    }

    async fn disable_autostart(&self, _name: &str) -> Result<()> {
        Err(DaemonError::Other(anyhow::anyhow!(
            "Windows service management not yet implemented"
        )))
    }

    async fn is_autostart_enabled(&self, _name: &str) -> Result<bool> {
        Ok(false)
    }

    async fn logs(&self, _name: &str, _lines: usize) -> Result<String> {
        Ok(String::new())
    }

    async fn is_installed(&self, _name: &str) -> Result<bool> {
        Ok(false)
    }
}

// ============================================================================
// Fallback Manager (for unsupported platforms)
// ============================================================================

/// Fallback manager that uses PID files and direct process management
pub struct FallbackManager;

impl FallbackManager {
    pub fn new() -> Self {
        Self
    }
}

impl Default for FallbackManager {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ServiceManager for FallbackManager {
    async fn install(&self, _config: &ServiceConfig) -> Result<()> {
        warn!("Service installation not supported on this platform, using fallback");
        Ok(())
    }

    async fn uninstall(&self, _name: &str) -> Result<()> {
        Ok(())
    }

    async fn start(&self, _name: &str) -> Result<()> {
        Err(DaemonError::Other(anyhow::anyhow!(
            "Direct service start not supported, use spawn_background instead"
        )))
    }

    async fn stop(&self, _name: &str) -> Result<()> {
        Err(DaemonError::Other(anyhow::anyhow!(
            "Direct service stop not supported, use kill_process instead"
        )))
    }

    async fn status(&self, _name: &str) -> Result<ServiceStatus> {
        Ok(ServiceStatus::Unknown)
    }

    async fn enable_autostart(&self, _name: &str) -> Result<()> {
        warn!("Autostart not supported on this platform");
        Ok(())
    }

    async fn disable_autostart(&self, _name: &str) -> Result<()> {
        Ok(())
    }

    async fn is_autostart_enabled(&self, _name: &str) -> Result<bool> {
        Ok(false)
    }

    async fn logs(&self, _name: &str, _lines: usize) -> Result<String> {
        Ok(String::new())
    }

    async fn is_installed(&self, _name: &str) -> Result<bool> {
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_config_builder() {
        let config = ServiceConfig::new("test-service", "/usr/bin/test")
            .description("Test service")
            .args(["--port", "8080"])
            .working_dir("/tmp")
            .env("FOO", "bar")
            .stdout_log("/var/log/test.log")
            .restart_policy(RestartPolicy::Always)
            .autostart(true);

        assert_eq!(config.name, "test-service");
        assert_eq!(config.description, "Test service");
        assert_eq!(config.executable, PathBuf::from("/usr/bin/test"));
        assert_eq!(config.args, vec!["--port", "8080"]);
        assert_eq!(config.working_dir, Some(PathBuf::from("/tmp")));
        assert_eq!(config.env.get("FOO"), Some(&"bar".to_string()));
        assert_eq!(config.restart_policy, RestartPolicy::Always);
        assert!(config.autostart);
    }

    #[test]
    fn test_service_status_checks() {
        assert!(ServiceStatus::Running { pid: Some(123) }.is_running());
        assert!(ServiceStatus::Running { pid: None }.is_running());
        assert!(!ServiceStatus::Stopped.is_running());

        assert!(ServiceStatus::Stopped.is_stopped());
        assert!(ServiceStatus::NotInstalled.is_stopped());
        assert!(!ServiceStatus::Running { pid: None }.is_stopped());
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn test_systemd_unit_generation() {
        let manager = SystemdManager::new();
        let config = ServiceConfig::new("test", "/usr/bin/test")
            .description("Test service")
            .args(["--flag"])
            .env("KEY", "value")
            .restart_policy(RestartPolicy::OnFailure);

        let unit = manager.generate_unit(&config);

        assert!(unit.contains("[Unit]"));
        assert!(unit.contains("Description=Test service"));
        assert!(unit.contains("[Service]"));
        assert!(unit.contains("ExecStart=/usr/bin/test --flag"));
        assert!(unit.contains("Environment=\"KEY=value\""));
        assert!(unit.contains("Restart=on-failure"));
        assert!(unit.contains("[Install]"));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn test_launchd_plist_generation() {
        let manager = LaunchdManager::new();
        let config = ServiceConfig::new("test", "/usr/bin/test")
            .description("Test service")
            .args(["--flag"])
            .env("KEY", "value")
            .autostart(true);

        let plist = manager.generate_plist(&config);

        assert!(plist.contains("<key>Label</key>"));
        assert!(plist.contains("com.adi.test"));
        assert!(plist.contains("<key>ProgramArguments</key>"));
        assert!(plist.contains("/usr/bin/test"));
        assert!(plist.contains("--flag"));
        assert!(plist.contains("<key>EnvironmentVariables</key>"));
        assert!(plist.contains("<key>RunAtLoad</key>"));
        assert!(plist.contains("<true/>"));
    }
}
