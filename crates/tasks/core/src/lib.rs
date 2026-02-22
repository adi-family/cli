//! Core library for ADI Tasks - task management with dependency graphs.
//!
//! This crate provides the business logic for managing tasks with:
//! - CRUD operations for tasks
//! - Dependency tracking between tasks
//! - Cycle detection in dependency graphs
//! - Full-text search capabilities
//! - Project-scoped and global task stores
//!
//! # Example
//!
//! ```no_run
//! use tasks_core::{TaskManager, CreateTask, TaskStatus};
//!
//! let manager = TaskManager::open_global().unwrap();
//! let id = manager.create_task(CreateTask::new("My task")).unwrap();
//! manager.update_status(id, TaskStatus::InProgress).unwrap();
//! ```

pub mod error;
pub mod graph;
mod migrations;
pub mod storage;
pub mod types;

pub use error::{Error, Result};
pub use storage::{SqliteTaskStorage, TaskStorage};
pub use types::{
    unix_timestamp_now, CreateTask, Task, TaskId, TaskStatus, TaskWithDependencies, TasksStatus,
    COMPLETE_STATUSES_SQL,
};

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Manages tasks for a single project or the global store.
pub struct TaskManager {
    storage: Arc<dyn TaskStorage>,
    path: PathBuf,
}

impl TaskManager {
    /// Creates the `.adi/tasks/` directory if it doesn't exist.
    pub fn open(project_path: &Path) -> Result<Self> {
        let tasks_dir = project_path.join(".adi").join("tasks");
        std::fs::create_dir_all(&tasks_dir)?;

        let storage = SqliteTaskStorage::open(&tasks_dir.join("tasks.sqlite"))?;

        Ok(Self {
            storage: Arc::new(storage),
            path: project_path.to_path_buf(),
        })
    }

    /// The global store is located in the user's local data directory.
    pub fn open_global() -> Result<Self> {
        let global_dir = Self::global_path();
        std::fs::create_dir_all(&global_dir)?;

        let storage = SqliteTaskStorage::open(&global_dir.join("tasks.sqlite"))?;

        Ok(Self {
            storage: Arc::new(storage),
            path: global_dir,
        })
    }

    #[must_use]
    pub fn global_path() -> PathBuf {
        dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("adi")
            .join("tasks")
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    #[must_use]
    pub fn is_global(&self) -> bool {
        self.path == Self::global_path()
    }

    pub fn create_task(&self, input: CreateTask) -> Result<TaskId> {
        let mut task = Task::new(&input.title);
        task.description = input.description;
        task.symbol_id = input.symbol_id;

        let id = self.storage.create_task(&task)?;

        for dep_id in input.depends_on {
            self.storage.add_dependency(id, dep_id)?;
        }

        Ok(id)
    }

    pub fn get_task(&self, id: TaskId) -> Result<Task> {
        self.storage.get_task(id)
    }

    pub fn update_task(&self, task: &Task) -> Result<()> {
        self.storage.update_task(task)
    }

    pub fn update_status(&self, id: TaskId, status: TaskStatus) -> Result<()> {
        let mut task = self.get_task(id)?;
        task.status = status;
        self.update_task(&task)
    }

    pub fn delete_task(&self, id: TaskId) -> Result<()> {
        self.storage.delete_task(id)
    }

    pub fn list(&self) -> Result<Vec<Task>> {
        self.storage.list_tasks(None)
    }

    pub fn get_by_status(&self, status: TaskStatus) -> Result<Vec<Task>> {
        self.storage.get_tasks_by_status(status)
    }

    /// Searches tasks using full-text search.
    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<Task>> {
        self.storage.search_tasks_fts(query, limit)
    }

    /// Adds a dependency. Circular dependencies are allowed and tracked via [`detect_cycles`](Self::detect_cycles).
    pub fn add_dependency(&self, from: TaskId, to: TaskId) -> Result<()> {
        self.storage.add_dependency(from, to)
    }

    pub fn remove_dependency(&self, from: TaskId, to: TaskId) -> Result<()> {
        self.storage.remove_dependency(from, to)
    }

    /// Returns direct dependencies of a task.
    pub fn get_dependencies(&self, id: TaskId) -> Result<Vec<Task>> {
        self.storage.get_dependencies(id)
    }

    /// Returns tasks that directly depend on the given task.
    pub fn get_dependents(&self, id: TaskId) -> Result<Vec<Task>> {
        self.storage.get_dependents(id)
    }

    pub fn get_task_with_dependencies(&self, id: TaskId) -> Result<TaskWithDependencies> {
        let task = self.get_task(id)?;

        Ok(TaskWithDependencies {
            depends_on: self.storage.get_dependencies(id)?,
            dependents: self.storage.get_dependents(id)?,
            task,
        })
    }

    /// Returns tasks with no incomplete dependencies (ready to start).
    pub fn get_ready(&self) -> Result<Vec<Task>> {
        self.storage.get_ready_tasks()
    }

    /// Returns tasks waiting on incomplete dependencies.
    pub fn get_blocked(&self) -> Result<Vec<Task>> {
        self.storage.get_blocked_tasks()
    }

    pub fn detect_cycles(&self) -> Result<Vec<Vec<TaskId>>> {
        graph::detect_cycles(self.storage.as_ref())
    }

    /// Returns all tasks that the given task transitively depends on.
    pub fn get_transitive_dependencies(&self, id: TaskId) -> Result<Vec<TaskId>> {
        graph::get_transitive_dependencies(self.storage.as_ref(), id)
    }

    /// Returns all tasks that transitively depend on the given task.
    pub fn get_transitive_dependents(&self, id: TaskId) -> Result<Vec<TaskId>> {
        graph::get_transitive_dependents(self.storage.as_ref(), id)
    }

    pub fn status(&self) -> Result<TasksStatus> {
        let mut status = self.storage.get_status()?;
        let cycles = self.detect_cycles()?;
        status.has_cycles = !cycles.is_empty();
        Ok(status)
    }

    /// Links a task to an indexer symbol.
    pub fn link_to_symbol(&self, task_id: TaskId, symbol_id: i64) -> Result<()> {
        let mut task = self.get_task(task_id)?;
        task.symbol_id = Some(symbol_id);
        self.update_task(&task)
    }

    /// Unlinks a task from its symbol.
    pub fn unlink_symbol(&self, task_id: TaskId) -> Result<()> {
        let mut task = self.get_task(task_id)?;
        task.symbol_id = None;
        self.update_task(&task)
    }
}

/// Manages multiple [`TaskManager`] instances for different projects.
pub struct TaskManagerCollection {
    managers: HashMap<PathBuf, TaskManager>,
}

impl Default for TaskManagerCollection {
    fn default() -> Self {
        Self::new()
    }
}

impl TaskManagerCollection {
    #[must_use]
    pub fn new() -> Self {
        Self {
            managers: HashMap::new(),
        }
    }

    /// Creates the manager if needed.
    pub fn add(&mut self, path: &Path) -> Result<&TaskManager> {
        let canonical = Self::canonicalize_path(path);

        if !self.managers.contains_key(&canonical) {
            let manager = TaskManager::open(&canonical)?;
            self.managers.insert(canonical.clone(), manager);
        }

        Ok(self.managers.get(&canonical).unwrap())
    }

    /// Creates the global manager if needed.
    pub fn add_global(&mut self) -> Result<&TaskManager> {
        let global_path = TaskManager::global_path();

        if !self.managers.contains_key(&global_path) {
            let manager = TaskManager::open_global()?;
            self.managers.insert(global_path.clone(), manager);
        }

        Ok(self.managers.get(&global_path).unwrap())
    }

    #[must_use]
    pub fn get(&self, path: &Path) -> Option<&TaskManager> {
        let canonical = Self::canonicalize_path(path);
        self.managers.get(&canonical)
    }

    #[must_use]
    pub fn get_global(&self) -> Option<&TaskManager> {
        self.managers.get(&TaskManager::global_path())
    }

    pub fn remove(&mut self, path: &Path) -> Option<TaskManager> {
        let canonical = Self::canonicalize_path(path);
        self.managers.remove(&canonical)
    }

    #[must_use]
    pub fn contains(&self, path: &Path) -> bool {
        let canonical = Self::canonicalize_path(path);
        self.managers.contains_key(&canonical)
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.managers.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.managers.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&PathBuf, &TaskManager)> {
        self.managers.iter()
    }

    /// Sorted by creation date (newest first).
    pub fn list_all_tasks(&self) -> Result<Vec<Task>> {
        let mut tasks = self.collect_from_all(|m| m.list())?;
        tasks.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        Ok(tasks)
    }

    /// Returns combined statistics from all managers.
    pub fn status(&self) -> Result<TasksStatus> {
        self.managers.values().try_fold(
            TasksStatus::default(),
            |mut combined, manager| -> Result<TasksStatus> {
                let status = manager.status()?;
                combined.total_tasks += status.total_tasks;
                combined.todo_count += status.todo_count;
                combined.in_progress_count += status.in_progress_count;
                combined.done_count += status.done_count;
                combined.blocked_count += status.blocked_count;
                combined.cancelled_count += status.cancelled_count;
                combined.total_dependencies += status.total_dependencies;
                combined.has_cycles = combined.has_cycles || status.has_cycles;
                Ok(combined)
            },
        )
    }

    /// Searches all managers, returning up to `limit` results.
    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<Task>> {
        let mut tasks = Vec::new();

        for manager in self.managers.values() {
            if tasks.len() >= limit {
                break;
            }
            let remaining = limit - tasks.len();
            tasks.extend(manager.search(query, remaining)?);
        }

        Ok(tasks)
    }

    pub fn get_by_status(&self, status: TaskStatus) -> Result<Vec<Task>> {
        self.collect_from_all(|m| m.get_by_status(status))
    }

    pub fn get_ready(&self) -> Result<Vec<Task>> {
        self.collect_from_all(TaskManager::get_ready)
    }

    pub fn get_blocked(&self) -> Result<Vec<Task>> {
        self.collect_from_all(TaskManager::get_blocked)
    }

    fn canonicalize_path(path: &Path) -> PathBuf {
        path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
    }

    fn collect_from_all<F>(&self, f: F) -> Result<Vec<Task>>
    where
        F: Fn(&TaskManager) -> Result<Vec<Task>>,
    {
        let mut tasks = Vec::new();
        for manager in self.managers.values() {
            tasks.extend(f(manager)?);
        }
        Ok(tasks)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_task_manager() {
        let dir = tempdir().unwrap();
        let manager = TaskManager::open(dir.path()).unwrap();

        let id = manager.create_task(CreateTask::new("Test task")).unwrap();

        let task = manager.get_task(id).unwrap();
        assert_eq!(task.title, "Test task");

        manager.update_status(id, TaskStatus::InProgress).unwrap();
        let task = manager.get_task(id).unwrap();
        assert_eq!(task.status, TaskStatus::InProgress);

        let status = manager.status().unwrap();
        assert_eq!(status.total_tasks, 1);
        assert_eq!(status.in_progress_count, 1);
    }

    #[test]
    fn test_task_manager_collection() {
        let dir1 = tempdir().unwrap();
        let dir2 = tempdir().unwrap();

        let mut collection = TaskManagerCollection::new();

        collection.add(dir1.path()).unwrap();
        collection.add(dir2.path()).unwrap();

        assert_eq!(collection.len(), 2);

        let manager1 = collection.get(dir1.path()).unwrap();
        manager1.create_task(CreateTask::new("Task 1")).unwrap();

        let manager2 = collection.get(dir2.path()).unwrap();
        manager2.create_task(CreateTask::new("Task 2")).unwrap();

        let all_tasks = collection.list_all_tasks().unwrap();
        assert_eq!(all_tasks.len(), 2);

        let status = collection.status().unwrap();
        assert_eq!(status.total_tasks, 2);
    }

    #[test]
    fn test_circular_dependencies_allowed() {
        let dir = tempdir().unwrap();
        let manager = TaskManager::open(dir.path()).unwrap();

        let t1 = manager.create_task(CreateTask::new("Task 1")).unwrap();
        let t2 = manager.create_task(CreateTask::new("Task 2")).unwrap();

        manager.add_dependency(t2, t1).unwrap();
        manager.add_dependency(t1, t2).unwrap();

        let cycles = manager.detect_cycles().unwrap();
        assert!(!cycles.is_empty());
    }
}
