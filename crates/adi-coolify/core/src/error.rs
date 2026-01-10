//! Error types for Coolify operations.

use thiserror::Error;

/// Coolify operation error.
#[derive(Error, Debug)]
pub enum CoolifyError {
    /// HTTP request failed
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    /// Invalid URL
    #[error("Invalid URL: {0}")]
    InvalidUrl(#[from] url::ParseError),

    /// API returned an error
    #[error("API error: {message}")]
    Api { message: String, code: Option<i32> },

    /// JSON parsing failed
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// Service not found
    #[error("Service not found: {0}")]
    ServiceNotFound(String),

    /// Deployment not found
    #[error("Deployment not found: {0}")]
    DeploymentNotFound(String),

    /// Authentication failed
    #[error("Authentication failed: {0}")]
    AuthFailed(String),
}

/// Result type for Coolify operations.
pub type Result<T> = std::result::Result<T, CoolifyError>;
