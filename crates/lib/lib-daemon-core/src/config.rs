//! Daemon configuration

use std::path::{Path, PathBuf};

/// Daemon configuration
#[derive(Debug, Clone)]
pub struct DaemonConfig {
    /// Base directory for daemon files
    base_dir: PathBuf,
    /// Socket filename (e.g., "daemon.sock")
    socket_name: String,
    /// PID filename (e.g., "daemon.pid")
    pid_name: String,
}

impl DaemonConfig {
    /// Create a new daemon configuration
    ///
    /// # Arguments
    ///
    /// * `base_dir` - Base directory for daemon files (e.g., "/var/run/mydaemon" or "~/.config/mydaemon")
    ///
    /// # Example
    ///
    /// ```
    /// use lib_daemon_core::DaemonConfig;
    ///
    /// let config = DaemonConfig::new("/var/run/mydaemon");
    /// ```
    pub fn new<P: Into<PathBuf>>(base_dir: P) -> Self {
        Self {
            base_dir: base_dir.into(),
            socket_name: "daemon.sock".to_string(),
            pid_name: "daemon.pid".to_string(),
        }
    }

    /// Set the socket filename
    pub fn with_socket_name<S: Into<String>>(mut self, name: S) -> Self {
        self.socket_name = name.into();
        self
    }

    /// Set the PID filename
    pub fn with_pid_name<S: Into<String>>(mut self, name: S) -> Self {
        self.pid_name = name.into();
        self
    }

    /// Get the base directory
    pub fn base_dir(&self) -> &Path {
        &self.base_dir
    }

    /// Get the full socket path
    pub fn socket_path(&self) -> PathBuf {
        self.base_dir.join(&self.socket_name)
    }

    /// Get the full PID file path
    pub fn pid_path(&self) -> PathBuf {
        self.base_dir.join(&self.pid_name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_daemon_config_defaults() {
        let config = DaemonConfig::new("/var/run/test");
        assert_eq!(config.base_dir(), Path::new("/var/run/test"));
        assert_eq!(config.socket_path(), PathBuf::from("/var/run/test/daemon.sock"));
        assert_eq!(config.pid_path(), PathBuf::from("/var/run/test/daemon.pid"));
    }

    #[test]
    fn test_daemon_config_custom_names() {
        let config = DaemonConfig::new("/tmp")
            .with_socket_name("my.sock")
            .with_pid_name("my.pid");
        assert_eq!(config.socket_path(), PathBuf::from("/tmp/my.sock"));
        assert_eq!(config.pid_path(), PathBuf::from("/tmp/my.pid"));
    }
}
