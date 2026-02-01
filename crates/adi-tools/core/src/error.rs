use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Storage error: {0}")]
    Storage(#[from] rusqlite::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Tool not found: {0}")]
    NotFound(String),

    #[error("Failed to parse help: {0}")]
    HelpParse(String),

    #[error("Discovery error: {0}")]
    Discovery(String),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
