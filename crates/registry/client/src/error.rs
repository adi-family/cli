//! Error types for registry operations.

use thiserror::Error;

/// Errors that can occur during registry operations.
#[derive(Debug, Error)]
pub enum RegistryError {
    /// HTTP request failed
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON parsing error
    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),

    /// Plugin not found in registry
    #[error("Plugin not found: {0}")]
    NotFound(String),

    /// Version not found
    #[error("Version not found: {0}@{1}")]
    VersionNotFound(String, String),

    /// Platform not supported
    #[error("Platform not supported: {0}")]
    PlatformNotSupported(String),

    /// Invalid response from registry
    #[error("Invalid response: {0}")]
    InvalidResponse(String),

    /// Cache error
    #[error("Cache error: {0}")]
    Cache(String),
}
