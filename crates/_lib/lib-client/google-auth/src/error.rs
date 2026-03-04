use thiserror::Error;

/// Errors that can occur during Google authentication.
#[derive(Debug, Error)]
pub enum Error {
    /// HTTP request failed.
    #[error("Request failed: {0}")]
    Request(#[from] reqwest::Error),

    /// Token refresh failed.
    #[error("Token refresh failed: {message}")]
    TokenRefresh { message: String },

    /// Invalid credentials file.
    #[error("Invalid credentials: {0}")]
    InvalidCredentials(String),

    /// JWT encoding/signing failed.
    #[error("JWT error: {0}")]
    Jwt(#[from] jsonwebtoken::errors::Error),

    /// JSON parsing error.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// IO error reading credentials file.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Token expired and could not be refreshed.
    #[error("Token expired")]
    TokenExpired,

    /// OAuth2 authorization failed.
    #[error("Authorization failed: {0}")]
    AuthorizationFailed(String),
}

/// Result type for Google auth operations.
pub type Result<T> = std::result::Result<T, Error>;
