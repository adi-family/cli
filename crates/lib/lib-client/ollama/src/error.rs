//! Error types for the Ollama client.

use thiserror::Error;

/// Ollama API error type.
#[derive(Debug, Error)]
pub enum OllamaError {
    /// HTTP request failed.
    #[error("Request failed: {0}")]
    Request(#[from] reqwest::Error),

    /// API returned an error response.
    #[error("API error ({status}): {message}")]
    Api { status: u16, message: String },

    /// Model not found.
    #[error("Model not found: {0}")]
    ModelNotFound(String),

    /// Connection refused (Ollama not running).
    #[error("Connection refused: is Ollama running?")]
    ConnectionRefused,

    /// JSON serialization/deserialization error.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// Invalid request parameters.
    #[error("Invalid request: {0}")]
    InvalidRequest(String),
}

/// Result type alias for Ollama operations.
pub type Result<T> = std::result::Result<T, OllamaError>;
