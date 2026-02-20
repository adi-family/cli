//! Error types for the tasks crate.

use crate::types::TaskId;
use thiserror::Error;

/// Errors that can occur in task operations.
#[derive(Error, Debug)]
pub enum Error {
    /// File system I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// SQLite database error.
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    /// JSON serialization/deserialization error.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// The requested task was not found.
    #[error("Task not found: {0}")]
    TaskNotFound(TaskId),

    /// The requested dependency edge was not found.
    #[error("Dependency not found: {from} -> {to}")]
    DependencyNotFound {
        /// The task that was expected to depend on another.
        from: TaskId,
        /// The task that was expected to be depended upon.
        to: TaskId,
    },

    /// A circular dependency was detected in the graph.
    #[error("Circular dependency detected: {0:?}")]
    CircularDependency(Vec<TaskId>),

    /// A task cannot depend on itself.
    #[error("Self dependency not allowed for task {0}")]
    SelfDependency(TaskId),

    /// Adding this dependency would create a cycle.
    #[error("Dependency would create cycle: {from} -> {to}")]
    WouldCreateCycle {
        /// The task that would depend on another.
        from: TaskId,
        /// The task that would be depended upon.
        to: TaskId,
    },

    /// An invalid task status string was provided.
    #[error("Invalid status: {0}")]
    InvalidStatus(String),

    /// A generic storage error.
    #[error("Storage error: {0}")]
    Storage(String),

    /// The storage has not been initialized.
    #[error("Not initialized: {0}")]
    NotInitialized(String),
}

/// Result type alias for task operations.
pub type Result<T> = std::result::Result<T, Error>;
