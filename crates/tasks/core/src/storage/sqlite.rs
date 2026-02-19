use crate::error::{Error, Result};
use crate::migrations::migrations;
use crate::types::{Task, TaskId, TaskStatus, TasksStatus};
use lib_migrations::{MigrationRunner, SqliteMigrationBackend};
use rusqlite::{params, Connection};
use std::path::Path;
use std::sync::Mutex;

use super::TaskStorage;

/// SQLite-based task storage with FTS5 support
pub struct SqliteTaskStorage {
    conn: Mutex<Connection>,
}

impl SqliteTaskStorage {
    pub fn open(path: &Path) -> Result<Self> {
        let backend = SqliteMigrationBackend::open(path)
            .map_err(|e| Error::Storage(format!("Failed to open db: {}", e)))?;

        let runner = MigrationRunner::new(backend).add_migrations(migrations());

        runner
            .init()
            .map_err(|e| Error::Storage(format!("Migration init failed: {}", e)))?;

        let applied = runner
            .migrate()
            .map_err(|e| Error::Storage(format!("Migration failed: {}", e)))?;

        if applied > 0 {
            tracing::info!("Applied {} migration(s)", applied);
        }

        let conn = runner.into_backend().into_connection();
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA synchronous=NORMAL;
             PRAGMA foreign_keys=ON;",
        )?;

        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    fn row_to_task(row: &rusqlite::Row) -> rusqlite::Result<Task> {
        let status_str: String = row.get(3)?;
        let status = status_str.parse().unwrap_or(TaskStatus::Todo);

        Ok(Task {
            id: TaskId(row.get(0)?),
            title: row.get(1)?,
            description: row.get(2)?,
            status,
            symbol_id: row.get(4)?,
            project_path: row.get(5)?,
            created_at: row.get(6)?,
            updated_at: row.get(7)?,
        })
    }
}

impl TaskStorage for SqliteTaskStorage {
    fn create_task(&self, task: &Task) -> Result<TaskId> {
        let conn = self.conn.lock().unwrap();

        conn.execute(
            r#"INSERT INTO tasks (title, description, status, symbol_id, project_path, created_at, updated_at)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)"#,
            params![
                task.title,
                task.description,
                task.status.as_str(),
                task.symbol_id,
                task.project_path,
                task.created_at,
                task.updated_at,
            ],
        )?;

        Ok(TaskId(conn.last_insert_rowid()))
    }

    fn get_task(&self, id: TaskId) -> Result<Task> {
        let conn = self.conn.lock().unwrap();

        conn.query_row(
            "SELECT id, title, description, status, symbol_id, project_path, created_at, updated_at
             FROM tasks WHERE id = ?1",
            params![id.0],
            Self::row_to_task,
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => Error::TaskNotFound(id),
            _ => Error::Sqlite(e),
        })
    }

    fn update_task(&self, task: &Task) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        let rows = conn.execute(
            r#"UPDATE tasks
               SET title = ?1, description = ?2, status = ?3, symbol_id = ?4, project_path = ?5, updated_at = ?6
               WHERE id = ?7"#,
            params![
                task.title,
                task.description,
                task.status.as_str(),
                task.symbol_id,
                task.project_path,
                now,
                task.id.0,
            ],
        )?;

        if rows == 0 {
            return Err(Error::TaskNotFound(task.id));
        }

        Ok(())
    }

    fn delete_task(&self, id: TaskId) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        let rows = conn.execute("DELETE FROM tasks WHERE id = ?1", params![id.0])?;

        if rows == 0 {
            return Err(Error::TaskNotFound(id));
        }

        Ok(())
    }

    fn list_tasks(&self, project_path: Option<&str>) -> Result<Vec<Task>> {
        let conn = self.conn.lock().unwrap();

        if let Some(path) = project_path {
            let mut stmt = conn.prepare(
                "SELECT id, title, description, status, symbol_id, project_path, created_at, updated_at
                 FROM tasks WHERE project_path = ?1 ORDER BY created_at DESC",
            )?;
            let tasks = stmt
                .query_map(params![path], Self::row_to_task)?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            return Ok(tasks);
        }

        let mut stmt = conn.prepare(
            "SELECT id, title, description, status, symbol_id, project_path, created_at, updated_at
             FROM tasks ORDER BY created_at DESC",
        )?;
        let tasks = stmt
            .query_map([], Self::row_to_task)?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        Ok(tasks)
    }

    fn get_tasks_by_status(&self, status: TaskStatus) -> Result<Vec<Task>> {
        let conn = self.conn.lock().unwrap();

        let mut stmt = conn.prepare(
            "SELECT id, title, description, status, symbol_id, project_path, created_at, updated_at
             FROM tasks WHERE status = ?1 ORDER BY created_at DESC",
        )?;

        let tasks = stmt
            .query_map(params![status.as_str()], Self::row_to_task)?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        Ok(tasks)
    }

    fn search_tasks_fts(&self, query: &str, limit: usize) -> Result<Vec<Task>> {
        let conn = self.conn.lock().unwrap();

        let mut stmt = conn.prepare(
            "SELECT t.id, t.title, t.description, t.status, t.symbol_id, t.project_path, t.created_at, t.updated_at
             FROM tasks t
             JOIN tasks_fts fts ON t.id = fts.rowid
             WHERE tasks_fts MATCH ?1
             ORDER BY rank
             LIMIT ?2",
        )?;

        let tasks = stmt
            .query_map(params![query, limit as i64], Self::row_to_task)?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        Ok(tasks)
    }

    fn add_dependency(&self, from: TaskId, to: TaskId) -> Result<()> {
        if from == to {
            return Err(Error::SelfDependency(from));
        }

        let conn = self.conn.lock().unwrap();

        // Verify both tasks exist
        let from_exists: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM tasks WHERE id = ?1)",
            params![from.0],
            |row| row.get(0),
        )?;

        if !from_exists {
            return Err(Error::TaskNotFound(from));
        }

        let to_exists: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM tasks WHERE id = ?1)",
            params![to.0],
            |row| row.get(0),
        )?;

        if !to_exists {
            return Err(Error::TaskNotFound(to));
        }

        conn.execute(
            "INSERT OR IGNORE INTO task_dependencies (from_task_id, to_task_id) VALUES (?1, ?2)",
            params![from.0, to.0],
        )?;

        Ok(())
    }

    fn remove_dependency(&self, from: TaskId, to: TaskId) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        let rows = conn.execute(
            "DELETE FROM task_dependencies WHERE from_task_id = ?1 AND to_task_id = ?2",
            params![from.0, to.0],
        )?;

        if rows == 0 {
            return Err(Error::DependencyNotFound { from, to });
        }

        Ok(())
    }

    fn get_dependencies(&self, id: TaskId) -> Result<Vec<Task>> {
        let conn = self.conn.lock().unwrap();

        let mut stmt = conn.prepare(
            "SELECT t.id, t.title, t.description, t.status, t.symbol_id, t.project_path, t.created_at, t.updated_at
             FROM tasks t
             JOIN task_dependencies d ON t.id = d.to_task_id
             WHERE d.from_task_id = ?1",
        )?;

        let tasks = stmt
            .query_map(params![id.0], Self::row_to_task)?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        Ok(tasks)
    }

    fn get_dependents(&self, id: TaskId) -> Result<Vec<Task>> {
        let conn = self.conn.lock().unwrap();

        let mut stmt = conn.prepare(
            "SELECT t.id, t.title, t.description, t.status, t.symbol_id, t.project_path, t.created_at, t.updated_at
             FROM tasks t
             JOIN task_dependencies d ON t.id = d.from_task_id
             WHERE d.to_task_id = ?1",
        )?;

        let tasks = stmt
            .query_map(params![id.0], Self::row_to_task)?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        Ok(tasks)
    }

    fn dependency_exists(&self, from: TaskId, to: TaskId) -> Result<bool> {
        let conn = self.conn.lock().unwrap();

        let exists: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM task_dependencies WHERE from_task_id = ?1 AND to_task_id = ?2)",
            params![from.0, to.0],
            |row| row.get(0),
        )?;

        Ok(exists)
    }

    fn get_blocked_tasks(&self) -> Result<Vec<Task>> {
        let conn = self.conn.lock().unwrap();

        // Tasks that have incomplete dependencies
        let mut stmt = conn.prepare(
            r#"SELECT DISTINCT t.id, t.title, t.description, t.status, t.symbol_id, t.project_path, t.created_at, t.updated_at
               FROM tasks t
               JOIN task_dependencies d ON t.id = d.from_task_id
               JOIN tasks dep ON d.to_task_id = dep.id
               WHERE t.status NOT IN ('done', 'cancelled')
                 AND dep.status NOT IN ('done', 'cancelled')"#,
        )?;

        let tasks = stmt
            .query_map([], Self::row_to_task)?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        Ok(tasks)
    }

    fn get_ready_tasks(&self) -> Result<Vec<Task>> {
        let conn = self.conn.lock().unwrap();

        // Tasks with no incomplete dependencies (or no dependencies at all)
        let mut stmt = conn.prepare(
            r#"SELECT t.id, t.title, t.description, t.status, t.symbol_id, t.project_path, t.created_at, t.updated_at
               FROM tasks t
               WHERE t.status NOT IN ('done', 'cancelled')
                 AND NOT EXISTS (
                     SELECT 1 FROM task_dependencies d
                     JOIN tasks dep ON d.to_task_id = dep.id
                     WHERE d.from_task_id = t.id
                       AND dep.status NOT IN ('done', 'cancelled')
                 )
               ORDER BY t.created_at ASC"#,
        )?;

        let tasks = stmt
            .query_map([], Self::row_to_task)?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        Ok(tasks)
    }

    fn get_all_dependencies(&self) -> Result<Vec<(TaskId, TaskId)>> {
        let conn = self.conn.lock().unwrap();

        let mut stmt = conn.prepare("SELECT from_task_id, to_task_id FROM task_dependencies")?;

        let deps = stmt
            .query_map([], |row| Ok((TaskId(row.get(0)?), TaskId(row.get(1)?))))?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        Ok(deps)
    }

    fn get_status(&self) -> Result<TasksStatus> {
        let conn = self.conn.lock().unwrap();

        let total_tasks: u64 =
            conn.query_row("SELECT COUNT(*) FROM tasks", [], |row| row.get(0))?;

        let todo_count: u64 = conn.query_row(
            "SELECT COUNT(*) FROM tasks WHERE status = 'todo'",
            [],
            |row| row.get(0),
        )?;

        let in_progress_count: u64 = conn.query_row(
            "SELECT COUNT(*) FROM tasks WHERE status = 'in_progress'",
            [],
            |row| row.get(0),
        )?;

        let done_count: u64 = conn.query_row(
            "SELECT COUNT(*) FROM tasks WHERE status = 'done'",
            [],
            |row| row.get(0),
        )?;

        let blocked_count: u64 = conn.query_row(
            "SELECT COUNT(*) FROM tasks WHERE status = 'blocked'",
            [],
            |row| row.get(0),
        )?;

        let cancelled_count: u64 = conn.query_row(
            "SELECT COUNT(*) FROM tasks WHERE status = 'cancelled'",
            [],
            |row| row.get(0),
        )?;

        let total_dependencies: u64 =
            conn.query_row("SELECT COUNT(*) FROM task_dependencies", [], |row| {
                row.get(0)
            })?;

        Ok(TasksStatus {
            total_tasks,
            todo_count,
            in_progress_count,
            done_count,
            blocked_count,
            cancelled_count,
            total_dependencies,
            has_cycles: false, // Will be computed by graph module
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn create_test_storage() -> (SqliteTaskStorage, tempfile::TempDir) {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("tasks.sqlite");
        let storage = SqliteTaskStorage::open(&db_path).unwrap();
        (storage, dir)
    }

    #[test]
    fn test_create_and_get_task() {
        let (storage, _dir) = create_test_storage();

        let task = Task::new("Test task").with_description("A test");
        let id = storage.create_task(&task).unwrap();

        let retrieved = storage.get_task(id).unwrap();
        assert_eq!(retrieved.title, "Test task");
        assert_eq!(retrieved.description, Some("A test".to_string()));
    }

    #[test]
    fn test_dependencies() {
        let (storage, _dir) = create_test_storage();

        let task1 = Task::new("Task 1");
        let task2 = Task::new("Task 2");

        let id1 = storage.create_task(&task1).unwrap();
        let id2 = storage.create_task(&task2).unwrap();

        storage.add_dependency(id2, id1).unwrap(); // task2 depends on task1

        let deps = storage.get_dependencies(id2).unwrap();
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].id, id1);

        let dependents = storage.get_dependents(id1).unwrap();
        assert_eq!(dependents.len(), 1);
        assert_eq!(dependents[0].id, id2);
    }

    #[test]
    fn test_ready_and_blocked_tasks() {
        let (storage, _dir) = create_test_storage();

        let task1 = Task::new("Task 1");
        let task2 = Task::new("Task 2");

        let id1 = storage.create_task(&task1).unwrap();
        let id2 = storage.create_task(&task2).unwrap();

        storage.add_dependency(id2, id1).unwrap();

        // task1 should be ready (no dependencies)
        // task2 should be blocked (depends on incomplete task1)
        let ready = storage.get_ready_tasks().unwrap();
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].id, id1);

        let blocked = storage.get_blocked_tasks().unwrap();
        assert_eq!(blocked.len(), 1);
        assert_eq!(blocked[0].id, id2);
    }

    #[test]
    fn test_self_dependency_error() {
        let (storage, _dir) = create_test_storage();

        let task = Task::new("Task 1");
        let id = storage.create_task(&task).unwrap();

        let result = storage.add_dependency(id, id);
        assert!(matches!(result, Err(Error::SelfDependency(_))));
    }
}
