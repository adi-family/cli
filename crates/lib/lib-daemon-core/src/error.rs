//! Error types for daemon operations

use std::io;
use thiserror::Error;

/// Daemon-specific errors
#[derive(Debug, Error)]
pub enum DaemonError {
    /// Daemon is already running
    #[error("Daemon is already running (pid: {0})")]
    AlreadyRunning(u32),

    /// Daemon is not running
    #[error("Daemon is not running")]
    NotRunning,

    /// Invalid PID file
    #[error("Invalid PID file: {0}")]
    InvalidPidFile(String),

    /// Socket error
    #[error("Socket error: {0}")]
    SocketError(String),

    /// IPC protocol error
    #[error("IPC protocol error: {0}")]
    ProtocolError(String),

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Other error
    #[error("{0}")]
    Other(#[from] anyhow::Error),
}

/// Result type alias for daemon operations
pub type Result<T> = std::result::Result<T, DaemonError>;
