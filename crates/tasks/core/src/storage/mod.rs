mod sqlite;

pub use sqlite::SqliteTaskStorage;

use crate::error::Result;
use crate::types::{Task, TaskId, TaskStatus, TasksStatus};

/// Storage trait for task persistence
pub trait TaskStorage: Send + Sync {
    // Task CRUD operations
    fn create_task(&self, task: &Task) -> Result<TaskId>;
    fn get_task(&self, id: TaskId) -> Result<Task>;
    fn update_task(&self, task: &Task) -> Result<()>;
    fn delete_task(&self, id: TaskId) -> Result<()>;

    // Query operations
    fn list_tasks(&self, project_path: Option<&str>) -> Result<Vec<Task>>;
    fn get_tasks_by_status(&self, status: TaskStatus) -> Result<Vec<Task>>;
    fn search_tasks_fts(&self, query: &str, limit: usize) -> Result<Vec<Task>>;

    // Dependency operations
    fn add_dependency(&self, from: TaskId, to: TaskId) -> Result<()>;
    fn remove_dependency(&self, from: TaskId, to: TaskId) -> Result<()>;
    fn get_dependencies(&self, id: TaskId) -> Result<Vec<Task>>;
    fn get_dependents(&self, id: TaskId) -> Result<Vec<Task>>;
    fn dependency_exists(&self, from: TaskId, to: TaskId) -> Result<bool>;

    // Graph queries
    fn get_blocked_tasks(&self) -> Result<Vec<Task>>;
    fn get_ready_tasks(&self) -> Result<Vec<Task>>;
    fn get_all_dependencies(&self) -> Result<Vec<(TaskId, TaskId)>>;

    // Status
    fn get_status(&self) -> Result<TasksStatus>;
}
