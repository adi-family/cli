use thiserror::Error;

#[derive(Debug, Error)]
pub enum RegistryError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Plugin not found: {0}")]
    NotFound(String),

    #[error("Version not found: {0}@{1}")]
    VersionNotFound(String, String),

    #[error("Platform not supported: {0}")]
    PlatformNotSupported(String),

    #[error("Invalid response: {0}")]
    InvalidResponse(String),

    #[error("Cache error: {0}")]
    Cache(String),
}
