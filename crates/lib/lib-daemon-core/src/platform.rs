//! Platform abstraction layer for cross-platform daemon operations
//!
//! Provides unified APIs for:
//! - Process spawning in background
//! - Process existence checking
//! - Platform detection

use crate::error::{DaemonError, Result};
use std::path::Path;
use std::process::Command;
use tracing::{debug, info};

/// Supported platforms
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    /// Linux (systemd-based)
    Linux,
    /// macOS (launchd-based)
    MacOS,
    /// Windows (Windows Service-based)
    Windows,
    /// Unknown/unsupported platform
    Unknown,
}

impl Platform {
    /// Detect the current platform
    pub fn current() -> Self {
        #[cfg(target_os = "linux")]
        {
            Platform::Linux
        }
        #[cfg(target_os = "macos")]
        {
            Platform::MacOS
        }
        #[cfg(target_os = "windows")]
        {
            Platform::Windows
        }
        #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
        {
            Platform::Unknown
        }
    }

    /// Check if this is a Unix-like platform
    pub fn is_unix(&self) -> bool {
        matches!(self, Platform::Linux | Platform::MacOS)
    }

    /// Get human-readable platform name
    pub fn name(&self) -> &'static str {
        match self {
            Platform::Linux => "Linux",
            Platform::MacOS => "macOS",
            Platform::Windows => "Windows",
            Platform::Unknown => "Unknown",
        }
    }
}

/// Check if a process with the given PID is running
///
/// # Platform behavior
/// - Unix: Uses `kill(pid, 0)` to check process existence
/// - Windows: Uses `OpenProcess` + `GetExitCodeProcess`
pub fn is_process_running(pid: u32) -> bool {
    #[cfg(unix)]
    {
        let result = unsafe { libc::kill(pid as i32, 0) };
        result == 0
    }

    #[cfg(windows)]
    {
        use std::ptr::null_mut;
        
        const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000;
        const STILL_ACTIVE: u32 = 259;

        #[link(name = "kernel32")]
        extern "system" {
            fn OpenProcess(dwDesiredAccess: u32, bInheritHandles: i32, dwProcessId: u32) -> *mut std::ffi::c_void;
            fn CloseHandle(hObject: *mut std::ffi::c_void) -> i32;
            fn GetExitCodeProcess(hProcess: *mut std::ffi::c_void, lpExitCode: *mut u32) -> i32;
        }

        unsafe {
            let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
            if handle.is_null() {
                return false;
            }

            let mut exit_code: u32 = 0;
            let result = GetExitCodeProcess(handle, &mut exit_code);
            CloseHandle(handle);

            result != 0 && exit_code == STILL_ACTIVE
        }
    }

    #[cfg(not(any(unix, windows)))]
    {
        // Cannot determine on unknown platforms
        false
    }
}

/// Configuration for spawning a background process
#[derive(Debug, Clone)]
pub struct SpawnConfig {
    /// Path to the executable
    pub executable: String,
    /// Command-line arguments
    pub args: Vec<String>,
    /// Working directory (optional)
    pub working_dir: Option<String>,
    /// Environment variables to set
    pub env: Vec<(String, String)>,
    /// Path to redirect stdout (optional)
    pub stdout_file: Option<String>,
    /// Path to redirect stderr (optional)
    pub stderr_file: Option<String>,
    /// Path to PID file (optional)
    pub pid_file: Option<String>,
}

impl SpawnConfig {
    /// Create a new spawn configuration
    pub fn new<S: Into<String>>(executable: S) -> Self {
        Self {
            executable: executable.into(),
            args: Vec::new(),
            working_dir: None,
            env: Vec::new(),
            stdout_file: None,
            stderr_file: None,
            pid_file: None,
        }
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
    pub fn working_dir<S: Into<String>>(mut self, dir: S) -> Self {
        self.working_dir = Some(dir.into());
        self
    }

    /// Add environment variable
    pub fn env<K: Into<String>, V: Into<String>>(mut self, key: K, value: V) -> Self {
        self.env.push((key.into(), value.into()));
        self
    }

    /// Set stdout redirect file
    pub fn stdout<S: Into<String>>(mut self, path: S) -> Self {
        self.stdout_file = Some(path.into());
        self
    }

    /// Set stderr redirect file
    pub fn stderr<S: Into<String>>(mut self, path: S) -> Self {
        self.stderr_file = Some(path.into());
        self
    }

    /// Set PID file path
    pub fn pid_file<S: Into<String>>(mut self, path: S) -> Self {
        self.pid_file = Some(path.into());
        self
    }
}

/// Spawn a process in the background (detached from current session)
///
/// # Platform behavior
/// - Unix: Uses `fork()` + `setsid()` for true daemon behavior
/// - Windows: Uses `CREATE_NEW_PROCESS_GROUP` and `DETACHED_PROCESS`
///
/// # Returns
/// - `Ok(pid)` - The PID of the spawned process
/// - `Err(...)` - If spawning failed
pub fn spawn_background(config: &SpawnConfig) -> Result<u32> {
    info!(
        "Spawning background process: {} {:?}",
        config.executable, config.args
    );

    #[cfg(unix)]
    {
        spawn_background_unix(config)
    }

    #[cfg(windows)]
    {
        spawn_background_windows(config)
    }

    #[cfg(not(any(unix, windows)))]
    {
        Err(DaemonError::Other(anyhow::anyhow!(
            "Background process spawning not supported on this platform"
        )))
    }
}

#[cfg(unix)]
fn spawn_background_unix(config: &SpawnConfig) -> Result<u32> {
    use std::fs::OpenOptions;
    use std::os::unix::process::CommandExt;

    // Build the command
    let mut cmd = Command::new(&config.executable);
    cmd.args(&config.args);

    // Set working directory
    if let Some(ref dir) = config.working_dir {
        cmd.current_dir(dir);
    }

    // Set environment variables
    for (key, value) in &config.env {
        cmd.env(key, value);
    }

    // Create a new session (detach from controlling terminal)
    // This is the key to making it a daemon
    unsafe {
        cmd.pre_exec(|| {
            // Create new session, become session leader
            libc::setsid();
            Ok(())
        });
    }

    // Handle stdout redirection
    if let Some(ref stdout_path) = config.stdout_file {
        ensure_parent_dir(stdout_path)?;
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(stdout_path)?;
        cmd.stdout(file);
    } else {
        cmd.stdout(std::process::Stdio::null());
    }

    // Handle stderr redirection
    if let Some(ref stderr_path) = config.stderr_file {
        ensure_parent_dir(stderr_path)?;
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(stderr_path)?;
        cmd.stderr(file);
    } else {
        cmd.stderr(std::process::Stdio::null());
    }

    // Stdin always null for daemon
    cmd.stdin(std::process::Stdio::null());

    // Spawn the process
    let child = cmd.spawn().map_err(|e| {
        DaemonError::Other(anyhow::anyhow!("Failed to spawn process: {}", e))
    })?;

    let pid = child.id();
    debug!("Spawned background process with PID {}", pid);

    // Write PID file if specified
    if let Some(ref pid_path) = config.pid_file {
        ensure_parent_dir(pid_path)?;
        std::fs::write(pid_path, pid.to_string())?;
        debug!("Wrote PID file: {}", pid_path);
    }

    Ok(pid)
}

#[cfg(windows)]
fn spawn_background_windows(config: &SpawnConfig) -> Result<u32> {
    use std::os::windows::process::CommandExt;

    const CREATE_NEW_PROCESS_GROUP: u32 = 0x00000200;
    const DETACHED_PROCESS: u32 = 0x00000008;
    const CREATE_NO_WINDOW: u32 = 0x08000000;

    let mut cmd = Command::new(&config.executable);
    cmd.args(&config.args);

    // Set working directory
    if let Some(ref dir) = config.working_dir {
        cmd.current_dir(dir);
    }

    // Set environment variables
    for (key, value) in &config.env {
        cmd.env(key, value);
    }

    // Windows-specific: detach from console
    cmd.creation_flags(CREATE_NEW_PROCESS_GROUP | DETACHED_PROCESS | CREATE_NO_WINDOW);

    // Handle stdout redirection
    if let Some(ref stdout_path) = config.stdout_file {
        ensure_parent_dir(stdout_path)?;
        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(stdout_path)?;
        cmd.stdout(file);
    } else {
        cmd.stdout(std::process::Stdio::null());
    }

    // Handle stderr redirection
    if let Some(ref stderr_path) = config.stderr_file {
        ensure_parent_dir(stderr_path)?;
        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(stderr_path)?;
        cmd.stderr(file);
    } else {
        cmd.stderr(std::process::Stdio::null());
    }

    cmd.stdin(std::process::Stdio::null());

    // Spawn the process
    let child = cmd.spawn().map_err(|e| {
        DaemonError::Other(anyhow::anyhow!("Failed to spawn process: {}", e))
    })?;

    let pid = child.id();
    debug!("Spawned background process with PID {}", pid);

    // Write PID file if specified
    if let Some(ref pid_path) = config.pid_file {
        ensure_parent_dir(pid_path)?;
        std::fs::write(pid_path, pid.to_string())?;
        debug!("Wrote PID file: {}", pid_path);
    }

    Ok(pid)
}

/// Ensure parent directory exists for a file path
fn ensure_parent_dir<P: AsRef<Path>>(path: P) -> Result<()> {
    if let Some(parent) = path.as_ref().parent() {
        std::fs::create_dir_all(parent)?;
    }
    Ok(())
}

/// Kill a process by PID
///
/// # Platform behavior
/// - Unix: Sends SIGTERM
/// - Windows: Uses TerminateProcess
pub fn kill_process(pid: u32) -> Result<()> {
    #[cfg(unix)]
    {
        let result = unsafe { libc::kill(pid as i32, libc::SIGTERM) };
        if result == 0 {
            debug!("Sent SIGTERM to process {}", pid);
            Ok(())
        } else {
            Err(DaemonError::Other(anyhow::anyhow!(
                "Failed to kill process {}: {}",
                pid,
                std::io::Error::last_os_error()
            )))
        }
    }

    #[cfg(windows)]
    {
        const PROCESS_TERMINATE: u32 = 0x0001;

        #[link(name = "kernel32")]
        extern "system" {
            fn OpenProcess(dwDesiredAccess: u32, bInheritHandles: i32, dwProcessId: u32) -> *mut std::ffi::c_void;
            fn CloseHandle(hObject: *mut std::ffi::c_void) -> i32;
            fn TerminateProcess(hProcess: *mut std::ffi::c_void, uExitCode: u32) -> i32;
        }

        unsafe {
            let handle = OpenProcess(PROCESS_TERMINATE, 0, pid);
            if handle.is_null() {
                return Err(DaemonError::Other(anyhow::anyhow!(
                    "Failed to open process {}: {}",
                    pid,
                    std::io::Error::last_os_error()
                )));
            }

            let result = TerminateProcess(handle, 1);
            CloseHandle(handle);

            if result != 0 {
                debug!("Terminated process {}", pid);
                Ok(())
            } else {
                Err(DaemonError::Other(anyhow::anyhow!(
                    "Failed to terminate process {}: {}",
                    pid,
                    std::io::Error::last_os_error()
                )))
            }
        }
    }

    #[cfg(not(any(unix, windows)))]
    {
        Err(DaemonError::Other(anyhow::anyhow!(
            "Process killing not supported on this platform"
        )))
    }
}

/// Wait for a process to exit (with timeout)
///
/// Returns `true` if process exited within timeout, `false` if timeout expired
pub async fn wait_for_exit(pid: u32, timeout: std::time::Duration) -> bool {
    let start = std::time::Instant::now();
    let check_interval = std::time::Duration::from_millis(100);

    while start.elapsed() < timeout {
        if !is_process_running(pid) {
            return true;
        }
        tokio::time::sleep(check_interval).await;
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform_detection() {
        let platform = Platform::current();
        
        #[cfg(target_os = "linux")]
        assert_eq!(platform, Platform::Linux);
        
        #[cfg(target_os = "macos")]
        assert_eq!(platform, Platform::MacOS);
        
        #[cfg(target_os = "windows")]
        assert_eq!(platform, Platform::Windows);
    }

    #[test]
    fn test_is_process_running_current() {
        // Current process should always be running
        let pid = std::process::id();
        assert!(is_process_running(pid));
    }

    #[test]
    fn test_is_process_running_nonexistent() {
        // Very high PID unlikely to exist
        assert!(!is_process_running(4_000_000_000));
    }

    #[test]
    fn test_spawn_config_builder() {
        let config = SpawnConfig::new("/bin/echo")
            .args(["hello", "world"])
            .working_dir("/tmp")
            .env("FOO", "bar")
            .stdout("/tmp/out.log")
            .stderr("/tmp/err.log")
            .pid_file("/tmp/test.pid");

        assert_eq!(config.executable, "/bin/echo");
        assert_eq!(config.args, vec!["hello", "world"]);
        assert_eq!(config.working_dir, Some("/tmp".to_string()));
        assert_eq!(config.env, vec![("FOO".to_string(), "bar".to_string())]);
        assert_eq!(config.stdout_file, Some("/tmp/out.log".to_string()));
        assert_eq!(config.stderr_file, Some("/tmp/err.log".to_string()));
        assert_eq!(config.pid_file, Some("/tmp/test.pid".to_string()));
    }
}
