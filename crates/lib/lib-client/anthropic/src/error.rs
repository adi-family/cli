//! Error types for the Anthropic client.

use thiserror::Error;

/// Anthropic API error type.
#[derive(Debug, Error)]
pub enum AnthropicError {
    /// HTTP request failed.
    #[error("Request failed: {0}")]
    Request(#[from] reqwest::Error),

    /// API returned an error response.
    #[error("API error ({status}): {message}")]
    Api { status: u16, message: String },

    /// Rate limited by the API.
    #[error("Rate limited, retry after {retry_after}s")]
    RateLimited { retry_after: u64 },

    /// Authentication failed.
    #[error("Unauthorized: invalid API key")]
    Unauthorized,

    /// Request was forbidden.
    #[error("Forbidden: {0}")]
    Forbidden(String),

    /// Resource not found.
    #[error("Not found: {0}")]
    NotFound(String),

    /// JSON serialization/deserialization error.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// Invalid request parameters.
    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    /// Server overloaded.
    #[error("Server overloaded, retry later")]
    Overloaded,
}

/// Result type alias for Anthropic operations.
pub type Result<T> = std::result::Result<T, AnthropicError>;
