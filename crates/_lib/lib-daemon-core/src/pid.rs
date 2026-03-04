//! PID file management

use crate::error::{DaemonError, Result};
use std::path::{Path, PathBuf};
use tracing::{debug, info};

/// PID file manager
#[derive(Debug)]
pub struct PidFile {
    path: PathBuf,
    /// Whether this instance wrote the PID file (and should remove it on drop)
    owns_file: bool,
}

impl PidFile {
    /// Create a new PID file manager
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the PID file
    ///
    /// # Example
    ///
    /// ```
    /// use lib_daemon_core::PidFile;
    ///
    /// let pid_file = PidFile::new("/var/run/mydaemon/daemon.pid");
    /// ```
    ///
    /// Note: This creates a read-only PID file handle. The file will NOT be removed
    /// when this instance is dropped. Call `write()` to take ownership and enable
    /// auto-cleanup on drop.
    pub fn new<P: Into<PathBuf>>(path: P) -> Self {
        Self {
            path: path.into(),
            owns_file: false, // Read-only by default, no auto-cleanup
        }
    }

    /// Get the PID file path
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Check if daemon is running by reading PID file and checking process existence
    ///
    /// Returns:
    /// - `Ok(Some(pid))` if daemon is running
    /// - `Ok(None)` if daemon is not running
    /// - `Err(...)` if there's an error reading the PID file
    ///
    /// This function automatically removes stale PID files (when process doesn't exist).
    pub fn is_running(&self) -> Result<Option<u32>> {
        if !self.path.exists() {
            return Ok(None);
        }

        let pid_str = std::fs::read_to_string(&self.path)
            .map_err(|e| DaemonError::InvalidPidFile(format!("Failed to read PID file: {}", e)))?;

        let pid: u32 = pid_str
            .trim()
            .parse()
            .map_err(|e| DaemonError::InvalidPidFile(format!("Invalid PID format: {}", e)))?;

        // Check if process exists
        #[cfg(unix)]
        {
            let result = unsafe { libc::kill(pid as i32, 0) };
            if result == 0 {
                debug!("Daemon is running with PID {}", pid);
                return Ok(Some(pid));
            }
        }

        // For non-Unix platforms, assume not running if we can't verify
        #[cfg(not(unix))]
        {
            // On non-Unix platforms, we can't reliably check if process exists
            // Return None and let the caller handle it
            return Ok(None);
        }

        // Process doesn't exist - this might be a stale PID file
        // Don't auto-remove here to avoid race conditions with newly forked processes
        // Cleanup should be explicit via remove() or Drop
        debug!("PID file exists but process {} not found", pid);
        Ok(None)
    }

    /// Write the current process ID to the PID file
    ///
    /// Creates parent directories if they don't exist.
    /// After calling this method, this `PidFile` instance "owns" the file
    /// and will automatically remove it when dropped.
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Cannot create parent directory
    /// - Cannot write PID file
    pub fn write(&mut self) -> Result<()> {
        let pid = std::process::id();

        // Ensure directory exists
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        std::fs::write(&self.path, pid.to_string())?;

        // Now we own this file and should clean it up on drop
        self.owns_file = true;

        info!("PID file written: {} (pid: {})", self.path.display(), pid);
        Ok(())
    }

    /// Remove the PID file
    ///
    /// Ignores errors if file doesn't exist.
    pub fn remove(&self) {
        if self.path.exists() {
            let _ = std::fs::remove_file(&self.path);
            debug!("PID file removed: {}", self.path.display());
        }
    }

    /// Check if daemon is running and return error if it is
    ///
    /// This is a convenience method for daemon startup validation.
    ///
    /// # Errors
    ///
    /// Returns `DaemonError::AlreadyRunning` if daemon is running.
    pub fn ensure_not_running(&self) -> Result<()> {
        if let Some(pid) = self.is_running()? {
            return Err(DaemonError::AlreadyRunning(pid));
        }
        Ok(())
    }
}

impl Drop for PidFile {
    fn drop(&mut self) {
        // Only remove the file if we wrote it (own it)
        // This prevents read-only PidFile instances from accidentally
        // deleting PID files they didn't create
        if self.owns_file {
            self.remove();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_pid_file_write_and_read() {
        let temp_dir = TempDir::new().unwrap();
        let pid_path = temp_dir.path().join("test.pid");
        let mut pid_file = PidFile::new(&pid_path);

        // Write PID file
        pid_file.write().unwrap();

        // Verify it exists and contains current PID
        assert!(pid_path.exists());
        let content = std::fs::read_to_string(&pid_path).unwrap();
        assert_eq!(content, std::process::id().to_string());

        // Check if running (should return current PID)
        let running_pid = pid_file.is_running().unwrap();
        assert_eq!(running_pid, Some(std::process::id()));
    }

    #[test]
    fn test_pid_file_remove() {
        let temp_dir = TempDir::new().unwrap();
        let pid_path = temp_dir.path().join("test.pid");
        let mut pid_file = PidFile::new(&pid_path);

        pid_file.write().unwrap();
        assert!(pid_path.exists());

        pid_file.remove();
        assert!(!pid_path.exists());
    }

    #[test]
    fn test_pid_file_stale_detection() {
        let temp_dir = TempDir::new().unwrap();
        let pid_path = temp_dir.path().join("test.pid");

        // Write a fake PID that definitely doesn't exist
        std::fs::write(&pid_path, "999999").unwrap();

        let pid_file = PidFile::new(&pid_path);

        // Should detect stale PID (process not running) but NOT remove file
        // Stale file cleanup should be explicit, not automatic
        let running = pid_file.is_running().unwrap();
        assert_eq!(running, None);
        assert!(pid_path.exists()); // File NOT auto-removed (prevents race conditions)

        // Explicit cleanup is still available
        pid_file.remove();
        assert!(!pid_path.exists());
    }

    #[test]
    fn test_ensure_not_running() {
        let temp_dir = TempDir::new().unwrap();
        let pid_path = temp_dir.path().join("test.pid");
        let mut pid_file = PidFile::new(&pid_path);

        // Should pass when not running
        pid_file.ensure_not_running().unwrap();

        // Write PID file
        pid_file.write().unwrap();

        // Should fail when running
        let result = pid_file.ensure_not_running();
        assert!(matches!(result, Err(DaemonError::AlreadyRunning(_))));
    }

    #[test]
    fn test_drop_only_removes_owned_file() {
        let temp_dir = TempDir::new().unwrap();
        let pid_path = temp_dir.path().join("test.pid");

        // Externally create a PID file (simulates another process)
        std::fs::write(&pid_path, "12345").unwrap();
        assert!(pid_path.exists());

        {
            // Create a read-only PidFile (did not write)
            let pid_file = PidFile::new(&pid_path);
            let _ = pid_file.is_running(); // Just reading
                                           // pid_file drops here
        }

        // File should still exist because we didn't write it
        assert!(
            pid_path.exists(),
            "PID file was incorrectly removed by Drop"
        );

        // Now test that write() + drop does remove the file
        {
            let mut pid_file = PidFile::new(&pid_path);
            pid_file.write().unwrap();
            // pid_file drops here
        }

        // File should be removed because we wrote it
        assert!(
            !pid_path.exists(),
            "PID file was not removed by Drop after write()"
        );
    }
}
