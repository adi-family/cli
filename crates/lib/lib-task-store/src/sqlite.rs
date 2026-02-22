//! SQLite backend for task storage

use crate::error::{Result, TaskStoreError};
use crate::models::{CreateTask, Task, TaskFilter, TaskStatus, UpdateTask};
use crate::TaskStore;
use async_trait::async_trait;
use chrono::Utc;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use sqlx::Row;
use std::path::PathBuf;
use std::str::FromStr;
use uuid::Uuid;

pub struct SqliteTaskStore {
    pool: SqlitePool,
}

impl SqliteTaskStore {
    pub async fn new(path: PathBuf) -> Result<Self> {
        // Create parent directory if it doesn't exist
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let options = SqliteConnectOptions::from_str(&format!("sqlite://{}", path.display()))?
            .create_if_missing(true);

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(options)
            .await
            .map_err(|e| TaskStoreError::Connection(e.to_string()))?;

        // Run migrations
        Self::migrate(&pool).await?;

        Ok(Self { pool })
    }

    async fn migrate(pool: &SqlitePool) -> Result<()> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS tasks (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                description TEXT,
                status TEXT NOT NULL,
                command TEXT,
                input TEXT NOT NULL,
                output TEXT,
                logs TEXT,
                exit_code INTEGER,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            )
            "#,
        )
        .execute(pool)
        .await
        .map_err(|e| TaskStoreError::Migration(e.to_string()))?;

        // Add index on status for faster filtering
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_tasks_status ON tasks(status)")
            .execute(pool)
            .await
            .map_err(|e| TaskStoreError::Migration(e.to_string()))?;

        Ok(())
    }
}

#[async_trait]
impl TaskStore for SqliteTaskStore {
    async fn create_task(&self, task: CreateTask) -> Result<Task> {
        let id = Uuid::new_v4();
        let now = Utc::now();
        let status = TaskStatus::Pending;

        let input_json = serde_json::to_string(&task.input)?;

        sqlx::query(
            r#"
            INSERT INTO tasks (id, title, description, status, command, input, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(id.to_string())
        .bind(&task.title)
        .bind(&task.description)
        .bind("pending")
        .bind(&task.command)
        .bind(&input_json)
        .bind(now.to_rfc3339())
        .bind(now.to_rfc3339())
        .execute(&self.pool)
        .await?;

        Ok(Task {
            id,
            title: task.title,
            description: task.description,
            status,
            command: task.command,
            input: task.input,
            output: None,
            logs: None,
            exit_code: None,
            created_at: now,
            updated_at: now,
        })
    }

    async fn get_task(&self, id: Uuid) -> Result<Option<Task>> {
        let row = sqlx::query("SELECT * FROM tasks WHERE id = ?")
            .bind(id.to_string())
            .fetch_optional(&self.pool)
            .await?;

        match row {
            Some(row) => {
                let input_json: String = row.get("input");
                let output_json: Option<String> = row.get("output");

                Ok(Some(Task {
                    id: Uuid::parse_str(row.get("id")).unwrap(),
                    title: row.get("title"),
                    description: row.get("description"),
                    status: parse_status(row.get("status")),
                    command: row.get("command"),
                    input: serde_json::from_str(&input_json)?,
                    output: output_json.and_then(|s| serde_json::from_str(&s).ok()),
                    logs: row.get("logs"),
                    exit_code: row.get("exit_code"),
                    created_at: chrono::DateTime::parse_from_rfc3339(row.get("created_at"))
                        .unwrap()
                        .with_timezone(&Utc),
                    updated_at: chrono::DateTime::parse_from_rfc3339(row.get("updated_at"))
                        .unwrap()
                        .with_timezone(&Utc),
                }))
            }
            None => Ok(None),
        }
    }

    async fn list_tasks(&self, filter: TaskFilter) -> Result<Vec<Task>> {
        let mut query = String::from("SELECT * FROM tasks WHERE 1=1");

        if let Some(status) = &filter.status {
            query.push_str(&format!(" AND status = '{}'", status_to_str(*status)));
        }

        query.push_str(" ORDER BY created_at DESC");

        if let Some(limit) = filter.limit {
            query.push_str(&format!(" LIMIT {}", limit));
        }

        if let Some(offset) = filter.offset {
            query.push_str(&format!(" OFFSET {}", offset));
        }

        let rows = sqlx::query(&query).fetch_all(&self.pool).await?;

        let tasks = rows
            .into_iter()
            .filter_map(|row| {
                let input_json: String = row.get("input");
                let output_json: Option<String> = row.get("output");

                Some(Task {
                    id: Uuid::parse_str(row.get("id")).ok()?,
                    title: row.get("title"),
                    description: row.get("description"),
                    status: parse_status(row.get("status")),
                    command: row.get("command"),
                    input: serde_json::from_str(&input_json).ok()?,
                    output: output_json.and_then(|s| serde_json::from_str(&s).ok()),
                    logs: row.get("logs"),
                    exit_code: row.get("exit_code"),
                    created_at: chrono::DateTime::parse_from_rfc3339(row.get("created_at"))
                        .ok()?
                        .with_timezone(&Utc),
                    updated_at: chrono::DateTime::parse_from_rfc3339(row.get("updated_at"))
                        .ok()?
                        .with_timezone(&Utc),
                })
            })
            .collect();

        Ok(tasks)
    }

    async fn update_task(&self, id: Uuid, update: UpdateTask) -> Result<Task> {
        let now = Utc::now();
        let mut updates = Vec::new();

        if let Some(title) = &update.title {
            updates.push(format!("title = '{}'", title));
        }
        if let Some(description) = &update.description {
            updates.push(format!("description = '{}'", description));
        }
        if let Some(status) = update.status {
            updates.push(format!("status = '{}'", status_to_str(status)));
        }
        if let Some(command) = &update.command {
            updates.push(format!("command = '{}'", command));
        }
        if let Some(output) = &update.output {
            let output_json = serde_json::to_string(output)?;
            updates.push(format!("output = '{}'", output_json));
        }
        if let Some(logs) = &update.logs {
            updates.push(format!("logs = '{}'", logs));
        }
        if let Some(exit_code) = update.exit_code {
            updates.push(format!("exit_code = {}", exit_code));
        }

        updates.push(format!("updated_at = '{}'", now.to_rfc3339()));

        if updates.is_empty() {
            return self.get_task(id).await?.ok_or(TaskStoreError::NotFound(id));
        }

        let query = format!("UPDATE tasks SET {} WHERE id = ?", updates.join(", "));

        sqlx::query(&query)
            .bind(id.to_string())
            .execute(&self.pool)
            .await?;

        self.get_task(id).await?.ok_or(TaskStoreError::NotFound(id))
    }

    async fn delete_task(&self, id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM tasks WHERE id = ?")
            .bind(id.to_string())
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    async fn can_run(&self, _task: &Task) -> bool {
        true
    }
}

fn status_to_str(status: TaskStatus) -> &'static str {
    match status {
        TaskStatus::Pending => "pending",
        TaskStatus::Running => "running",
        TaskStatus::Completed => "completed",
        TaskStatus::Failed => "failed",
        TaskStatus::Cancelled => "cancelled",
    }
}

fn parse_status(s: &str) -> TaskStatus {
    match s {
        "pending" => TaskStatus::Pending,
        "running" => TaskStatus::Running,
        "completed" => TaskStatus::Completed,
        "failed" => TaskStatus::Failed,
        "cancelled" => TaskStatus::Cancelled,
        _ => TaskStatus::Pending,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_create_and_get_task() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let store = SqliteTaskStore::new(db_path).await.unwrap();

        let create = CreateTask {
            title: "Test Task".to_string(),
            description: Some("Test description".to_string()),
            command: Some("echo hello".to_string()),
            input: serde_json::json!({}),
        };

        let task = store.create_task(create).await.unwrap();
        assert_eq!(task.title, "Test Task");
        assert_eq!(task.status, TaskStatus::Pending);

        let retrieved = store.get_task(task.id).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().title, "Test Task");
    }

    #[tokio::test]
    async fn test_list_tasks() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let store = SqliteTaskStore::new(db_path).await.unwrap();

        // Create multiple tasks
        for i in 0..5 {
            let create = CreateTask {
                title: format!("Task {}", i),
                description: None,
                command: None,
                input: serde_json::json!({}),
            };
            store.create_task(create).await.unwrap();
        }

        let tasks = store
            .list_tasks(TaskFilter {
                status: Some(TaskStatus::Pending),
                limit: Some(10),
                offset: None,
            })
            .await
            .unwrap();

        assert_eq!(tasks.len(), 5);
    }

    #[tokio::test]
    async fn test_update_task() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let store = SqliteTaskStore::new(db_path).await.unwrap();

        let create = CreateTask {
            title: "Test Task".to_string(),
            description: None,
            command: None,
            input: serde_json::json!({}),
        };

        let task = store.create_task(create).await.unwrap();

        let update = UpdateTask {
            status: Some(TaskStatus::Running),
            ..Default::default()
        };

        let updated = store.update_task(task.id, update).await.unwrap();
        assert_eq!(updated.status, TaskStatus::Running);
    }
}
