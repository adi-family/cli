//! Core types for ADI Tasks - task management with dependency graphs.
//!
//! This module defines the fundamental data structures used throughout the tasks system:
//! - [`TaskId`] - Unique identifier for tasks
//! - [`TaskStatus`] - Task lifecycle states
//! - [`Task`] - The main task entity
//! - [`CreateTask`] - Input DTO for creating tasks

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// Returns the current Unix timestamp in seconds.
#[inline]
pub fn unix_timestamp_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Unique identifier for a task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TaskId(i64);

impl TaskId {
    /// Creates a new TaskId from a raw i64 value.
    #[inline]
    pub const fn new(id: i64) -> Self {
        Self(id)
    }

    /// Returns the raw i64 value of this TaskId.
    #[inline]
    #[must_use]
    pub const fn get(self) -> i64 {
        self.0
    }
}

impl fmt::Display for TaskId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<i64> for TaskId {
    fn from(id: i64) -> Self {
        Self::new(id)
    }
}

/// Status of a task in its lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    /// Task is pending, not yet started
    Todo,
    /// Task is actively being worked on
    InProgress,
    /// Task has been completed successfully
    Done,
    /// Task is blocked by dependencies
    Blocked,
    /// Task has been cancelled
    Cancelled,
}

/// SQL fragment for filtering complete statuses.
pub const COMPLETE_STATUSES_SQL: &str = "('done', 'cancelled')";

impl TaskStatus {
    /// Returns the string representation of this status.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Todo => "todo",
            Self::InProgress => "in_progress",
            Self::Done => "done",
            Self::Blocked => "blocked",
            Self::Cancelled => "cancelled",
        }
    }

    /// Parses a status from a string, returning None if invalid.
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        s.parse().ok()
    }

    /// Returns true if this status represents a completed task.
    #[must_use]
    pub const fn is_complete(&self) -> bool {
        matches!(self, Self::Done | Self::Cancelled)
    }

    /// Returns a Unicode icon representing this status.
    #[must_use]
    pub const fn icon(&self) -> &'static str {
        match self {
            Self::Todo => "○",
            Self::InProgress => "◐",
            Self::Done => "●",
            Self::Blocked => "✕",
            Self::Cancelled => "⊘",
        }
    }

    /// Returns a color name for use in DOT graphs.
    #[must_use]
    pub const fn color(&self) -> &'static str {
        match self {
            Self::Todo => "black",
            Self::InProgress => "blue",
            Self::Done => "green",
            Self::Blocked => "red",
            Self::Cancelled => "gray",
        }
    }
}

impl fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for TaskStatus {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "todo" => Ok(Self::Todo),
            "in_progress" | "in-progress" | "inprogress" | "wip" => Ok(Self::InProgress),
            "done" => Ok(Self::Done),
            "blocked" => Ok(Self::Blocked),
            "cancelled" | "canceled" => Ok(Self::Cancelled),
            _ => Err(()),
        }
    }
}

/// A task with its metadata and relationships.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    /// Unique identifier for this task
    pub id: TaskId,
    /// Short title describing the task
    pub title: String,
    /// Optional detailed description
    pub description: Option<String>,
    /// Current status in the task lifecycle
    pub status: TaskStatus,
    /// Optional link to indexer symbol (adi-core SymbolId)
    pub symbol_id: Option<i64>,
    /// Project path for project-scoped tasks, None for global tasks
    pub project_path: Option<String>,
    /// Unix timestamp when task was created
    pub created_at: i64,
    /// Unix timestamp when task was last updated
    pub updated_at: i64,
}

impl Task {
    /// Creates a new task with the given title.
    ///
    /// The task starts with `Todo` status and timestamps set to now.
    /// The `id` will be assigned by storage when persisted.
    pub fn new(title: impl Into<String>) -> Self {
        let now = unix_timestamp_now();

        Self {
            id: TaskId::new(0), // Will be set by storage
            title: title.into(),
            description: None,
            status: TaskStatus::Todo,
            symbol_id: None,
            project_path: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Sets the description for this task.
    #[must_use]
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Sets the project path for this task.
    #[must_use]
    pub fn with_project(mut self, project_path: impl Into<String>) -> Self {
        self.project_path = Some(project_path.into());
        self
    }

    /// Links this task to an indexer symbol.
    #[must_use]
    pub fn with_symbol(mut self, symbol_id: i64) -> Self {
        self.symbol_id = Some(symbol_id);
        self
    }

    /// Returns true if this is a global task (not project-scoped).
    #[must_use]
    pub fn is_global(&self) -> bool {
        self.project_path.is_none()
    }
}

/// A task bundled with its dependency relationships.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskWithDependencies {
    /// The task itself
    pub task: Task,
    /// Tasks this task depends on (must complete first)
    pub depends_on: Vec<Task>,
    /// Tasks that depend on this task
    pub dependents: Vec<Task>,
}

/// Aggregate statistics about tasks in a store.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TasksStatus {
    /// Total number of tasks
    pub total_tasks: u64,
    /// Number of tasks in Todo status
    pub todo_count: u64,
    /// Number of tasks in InProgress status
    pub in_progress_count: u64,
    /// Number of tasks in Done status
    pub done_count: u64,
    /// Number of tasks in Blocked status
    pub blocked_count: u64,
    /// Number of tasks in Cancelled status
    pub cancelled_count: u64,
    /// Total number of dependency edges
    pub total_dependencies: u64,
    /// Whether any dependency cycles exist
    pub has_cycles: bool,
}

/// Input data for creating a new task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTask {
    /// Title for the new task
    pub title: String,
    /// Optional description
    pub description: Option<String>,
    /// Optional symbol ID to link to
    pub symbol_id: Option<i64>,
    /// Task IDs this task depends on
    pub depends_on: Vec<TaskId>,
}

impl CreateTask {
    /// Creates a new CreateTask input with the given title.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            description: None,
            symbol_id: None,
            depends_on: vec![],
        }
    }

    /// Sets the description for the new task.
    #[must_use]
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Sets the dependencies for the new task.
    #[must_use]
    pub fn with_dependencies(mut self, deps: Vec<TaskId>) -> Self {
        self.depends_on = deps;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_status_conversion() {
        assert_eq!(TaskStatus::Todo.as_str(), "todo");
        assert_eq!(
            "in_progress".parse::<TaskStatus>(),
            Ok(TaskStatus::InProgress)
        );
        assert!("invalid".parse::<TaskStatus>().is_err());
    }

    #[test]
    fn test_task_builder() {
        let task = Task::new("Test task")
            .with_description("Description")
            .with_project("/path/to/project");

        assert_eq!(task.title, "Test task");
        assert_eq!(task.description, Some("Description".to_string()));
        assert!(!task.is_global());
    }

    #[test]
    fn test_task_status_is_complete() {
        assert!(!TaskStatus::Todo.is_complete());
        assert!(!TaskStatus::InProgress.is_complete());
        assert!(TaskStatus::Done.is_complete());
        assert!(TaskStatus::Cancelled.is_complete());
    }
}
