use thiserror::Error;

#[derive(Error, Debug)]
pub enum KnowledgebaseError {
    #[error("Not found: {0}")]
    NotFound(String),

    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    #[error("Embedding error: {0}")]
    Embedding(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Invalid params: {0}")]
    InvalidParams(String),

    #[error("Approval error: {0}")]
    ApprovalError(String),
}

pub type Result<T> = std::result::Result<T, KnowledgebaseError>;
