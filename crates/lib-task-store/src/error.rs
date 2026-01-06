//! Error types for task store operations

use thiserror::Error;

pub type Result<T> = std::result::Result<T, TaskStoreError>;

#[derive(Debug, Error)]
pub enum TaskStoreError {
    #[error("Task not found: {0}")]
    NotFound(uuid::Uuid),

    #[cfg(feature = "sqlite")]
    #[error("SQLite error: {0}")]
    Sqlite(#[from] sqlx::Error),

    #[cfg(feature = "postgres")]
    #[error("PostgreSQL error: {0}")]
    Postgres(sqlx::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Database connection error: {0}")]
    Connection(String),

    #[error("Migration error: {0}")]
    Migration(String),
}
