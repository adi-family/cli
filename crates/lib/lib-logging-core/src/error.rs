//! Error types for the logging library.

use thiserror::Error;

/// Result type for logging operations.
pub type Result<T> = std::result::Result<T, LoggingError>;

/// Errors that can occur in the logging library.
#[derive(Error, Debug)]
pub enum LoggingError {
    /// HTTP request failed
    #[error("HTTP request failed: {0}")]
    HttpError(#[from] reqwest::Error),

    /// Serialization failed
    #[error("Serialization failed: {0}")]
    SerializationError(#[from] serde_json::Error),

    /// Database error
    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),

    /// Invalid configuration
    #[error("Invalid configuration: {0}")]
    ConfigError(String),
}
