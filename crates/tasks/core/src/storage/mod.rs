//! Storage abstraction for task persistence.

mod sqlite;

pub use sqlite::SqliteTaskStorage;

use crate::error::Result;
use crate::types::{Task, TaskId, TaskStatus, TasksStatus};

/// Abstract storage interface for tasks.
///
/// Implementations must be thread-safe (`Send + Sync`).
pub trait TaskStorage: Send + Sync {
    /// Creates a new task and returns its ID.
    fn create_task(&self, task: &Task) -> Result<TaskId>;

    /// Retrieves a task by its ID.
    fn get_task(&self, id: TaskId) -> Result<Task>;

    /// Updates an existing task.
    fn update_task(&self, task: &Task) -> Result<()>;

    /// Deletes a task by its ID.
    fn delete_task(&self, id: TaskId) -> Result<()>;

    /// Lists tasks, optionally filtered by project path.
    fn list_tasks(&self, project_path: Option<&str>) -> Result<Vec<Task>>;

    /// Returns all tasks with the given status.
    fn get_tasks_by_status(&self, status: TaskStatus) -> Result<Vec<Task>>;

    /// Searches tasks using full-text search.
    fn search_tasks_fts(&self, query: &str, limit: usize) -> Result<Vec<Task>>;

    /// Adds a dependency edge from one task to another.
    fn add_dependency(&self, from: TaskId, to: TaskId) -> Result<()>;

    /// Removes a dependency edge.
    fn remove_dependency(&self, from: TaskId, to: TaskId) -> Result<()>;

    /// Returns direct dependencies of a task.
    fn get_dependencies(&self, id: TaskId) -> Result<Vec<Task>>;

    /// Returns tasks that directly depend on the given task.
    fn get_dependents(&self, id: TaskId) -> Result<Vec<Task>>;

    /// Checks if a dependency edge exists.
    fn dependency_exists(&self, from: TaskId, to: TaskId) -> Result<bool>;

    /// Returns tasks blocked by incomplete dependencies.
    fn get_blocked_tasks(&self) -> Result<Vec<Task>>;

    /// Returns tasks ready to start (no incomplete dependencies).
    fn get_ready_tasks(&self) -> Result<Vec<Task>>;

    /// Returns all dependency edges as (from, to) pairs.
    fn get_all_dependencies(&self) -> Result<Vec<(TaskId, TaskId)>>;

    /// Returns aggregate statistics about tasks.
    fn get_status(&self) -> Result<TasksStatus>;
}
