//! Task storage abstraction for hybrid cloud deployment.
//!
//! Provides a device-agnostic interface for task storage, allowing each device
//! to choose its own backend (SQLite, PostgreSQL, etc.) while maintaining
//! a consistent protocol for querying and aggregation.

pub mod error;
pub mod models;

#[cfg(feature = "sqlite")]
pub mod sqlite;

#[cfg(feature = "postgres")]
pub mod postgres;

use async_trait::async_trait;
pub use error::{Result, TaskStoreError};
pub use models::{CreateTask, Task, TaskFilter, TaskStatus, UpdateTask};
use uuid::Uuid;

/// Abstract task storage interface.
///
/// Each device implements this trait with its chosen backend.
/// The signaling server sees only the protocol responses, not the implementation.
#[async_trait]
pub trait TaskStore: Send + Sync {
    /// Create a new task
    async fn create_task(&self, task: CreateTask) -> Result<Task>;

    /// Get a task by ID
    async fn get_task(&self, id: Uuid) -> Result<Option<Task>>;

    /// List tasks matching filter
    async fn list_tasks(&self, filter: TaskFilter) -> Result<Vec<Task>>;

    /// Update a task
    async fn update_task(&self, id: Uuid, update: UpdateTask) -> Result<Task>;

    /// Delete a task
    async fn delete_task(&self, id: Uuid) -> Result<()>;

    /// Check if this device can run the given task
    /// (based on required capabilities, resources, etc.)
    async fn can_run(&self, task: &Task) -> bool;
}

/// Task store backend selection
#[derive(Debug, Clone)]
pub enum TaskStoreBackend {
    #[cfg(feature = "sqlite")]
    Sqlite { path: std::path::PathBuf },

    #[cfg(feature = "postgres")]
    Postgres { url: String },
}

/// Create a task store from backend configuration
pub async fn create_task_store(backend: TaskStoreBackend) -> Result<Box<dyn TaskStore>> {
    match backend {
        #[cfg(feature = "sqlite")]
        TaskStoreBackend::Sqlite { path } => {
            let store = sqlite::SqliteTaskStore::new(path).await?;
            Ok(Box::new(store))
        }

        #[cfg(feature = "postgres")]
        TaskStoreBackend::Postgres { url } => {
            let store = postgres::PostgresTaskStore::new(url).await?;
            Ok(Box::new(store))
        }
    }
}
