//! Error types for plugin ABI

use thiserror::Error;

/// Plugin error type
#[derive(Error, Debug)]
pub enum PluginError {
    /// Plugin initialization failed
    #[error("Plugin initialization failed: {0}")]
    InitFailed(String),

    /// Plugin not found
    #[error("Plugin not found: {0}")]
    NotFound(String),

    /// Service not provided by plugin
    #[error("Service not provided by plugin")]
    ServiceNotProvided,

    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(String),

    /// Command execution failed
    #[error("Command execution failed: {0}")]
    CommandFailed(String),

    /// HTTP request failed
    #[error("HTTP request failed: {0}")]
    HttpRequestFailed(String),

    /// Health check failed
    #[error("Health check failed: {0}")]
    HealthCheckFailed(String),

    /// Runtime error
    #[error("Runtime error: {0}")]
    Runtime(String),

    /// Internal error (alias for Runtime, for compatibility)
    #[error("Internal error: {0}")]
    Internal(String),

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// UTF-8 conversion error
    #[error("UTF-8 conversion error: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),

    /// Generic error
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

/// Plugin result type
pub type Result<T> = std::result::Result<T, PluginError>;
