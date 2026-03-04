//! Error types for the OpenRouter client.

use thiserror::Error;

/// OpenRouter API error type.
#[derive(Debug, Error)]
pub enum OpenRouterError {
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

    /// Server error.
    #[error("Server error: {0}")]
    ServerError(String),

    /// Context length exceeded.
    #[error("Context length exceeded: {0}")]
    ContextLengthExceeded(String),

    /// Insufficient credits.
    #[error("Insufficient credits: {0}")]
    InsufficientCredits(String),

    /// Model not available.
    #[error("Model not available: {0}")]
    ModelNotAvailable(String),
}

/// Result type alias for OpenRouter operations.
pub type Result<T> = std::result::Result<T, OpenRouterError>;
