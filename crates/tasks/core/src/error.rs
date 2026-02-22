use crate::types::TaskId;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Task not found: {0}")]
    TaskNotFound(TaskId),

    #[error("Dependency not found: {from} -> {to}")]
    DependencyNotFound { from: TaskId, to: TaskId },

    #[error("Self dependency not allowed for task {0}")]
    SelfDependency(TaskId),

    #[error("Invalid status: {0}")]
    InvalidStatus(String),

    #[error("Storage error: {0}")]
    Storage(String),

    #[error("Not initialized: {0}")]
    NotInitialized(String),
}

pub type Result<T> = std::result::Result<T, Error>;
