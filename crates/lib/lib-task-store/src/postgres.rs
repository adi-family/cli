//! PostgreSQL backend for task storage

use crate::error::{Result, TaskStoreError};
use crate::models::{CreateTask, Task, TaskFilter, TaskStatus, UpdateTask};
use crate::TaskStore;
use async_trait::async_trait;
use chrono::Utc;
use sqlx::postgres::{PgPool, PgPoolOptions};
use sqlx::Row;
use uuid::Uuid;

pub struct PostgresTaskStore {
    pool: PgPool,
}

impl PostgresTaskStore {
    pub async fn new(url: String) -> Result<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(10)
            .connect(&url)
            .await
            .map_err(|e| TaskStoreError::Connection(e.to_string()))?;

        // Run migrations
        Self::migrate(&pool).await?;

        Ok(Self { pool })
    }

    async fn migrate(pool: &PgPool) -> Result<()> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS tasks (
                id UUID PRIMARY KEY,
                title TEXT NOT NULL,
                description TEXT,
                status TEXT NOT NULL,
                command TEXT,
                input JSONB NOT NULL,
                output JSONB,
                logs TEXT,
                exit_code INTEGER,
                created_at TIMESTAMPTZ NOT NULL,
                updated_at TIMESTAMPTZ NOT NULL
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
impl TaskStore for PostgresTaskStore {
    async fn create_task(&self, task: CreateTask) -> Result<Task> {
        let id = Uuid::new_v4();
        let now = Utc::now();
        let status = TaskStatus::Pending;

        sqlx::query(
            r#"
            INSERT INTO tasks (id, title, description, status, command, input, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#,
        )
        .bind(id)
        .bind(&task.title)
        .bind(&task.description)
        .bind("pending")
        .bind(&task.command)
        .bind(&task.input)
        .bind(now)
        .bind(now)
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
        let row = sqlx::query("SELECT * FROM tasks WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;

        match row {
            Some(row) => Ok(Some(Task {
                id: row.get("id"),
                title: row.get("title"),
                description: row.get("description"),
                status: parse_status(row.get("status")),
                command: row.get("command"),
                input: row.get("input"),
                output: row.get("output"),
                logs: row.get("logs"),
                exit_code: row.get("exit_code"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            })),
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
            .map(|row| Task {
                id: row.get("id"),
                title: row.get("title"),
                description: row.get("description"),
                status: parse_status(row.get("status")),
                command: row.get("command"),
                input: row.get("input"),
                output: row.get("output"),
                logs: row.get("logs"),
                exit_code: row.get("exit_code"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            })
            .collect();

        Ok(tasks)
    }

    async fn update_task(&self, id: Uuid, update: UpdateTask) -> Result<Task> {
        let now = Utc::now();
        let mut updates = Vec::new();
        let mut param_num = 2; // $1 is reserved for id

        if update.title.is_some() {
            updates.push(format!("title = ${}", param_num));
            param_num += 1;
        }
        if update.description.is_some() {
            updates.push(format!("description = ${}", param_num));
            param_num += 1;
        }
        if update.status.is_some() {
            updates.push(format!("status = ${}", param_num));
            param_num += 1;
        }
        if update.command.is_some() {
            updates.push(format!("command = ${}", param_num));
            param_num += 1;
        }
        if update.output.is_some() {
            updates.push(format!("output = ${}", param_num));
            param_num += 1;
        }
        if update.logs.is_some() {
            updates.push(format!("logs = ${}", param_num));
            param_num += 1;
        }
        if update.exit_code.is_some() {
            updates.push(format!("exit_code = ${}", param_num));
            param_num += 1;
        }

        updates.push(format!("updated_at = ${}", param_num));

        if updates.is_empty() {
            return self.get_task(id).await?.ok_or(TaskStoreError::NotFound(id));
        }

        let query_str = format!(
            "UPDATE tasks SET {} WHERE id = $1 RETURNING *",
            updates.join(", ")
        );

        let mut query = sqlx::query(&query_str).bind(id);

        if let Some(title) = &update.title {
            query = query.bind(title);
        }
        if let Some(description) = &update.description {
            query = query.bind(description);
        }
        if let Some(status) = update.status {
            query = query.bind(status_to_str(status));
        }
        if let Some(command) = &update.command {
            query = query.bind(command);
        }
        if let Some(output) = &update.output {
            query = query.bind(output);
        }
        if let Some(logs) = &update.logs {
            query = query.bind(logs);
        }
        if let Some(exit_code) = update.exit_code {
            query = query.bind(exit_code);
        }

        query = query.bind(now);

        let row = query.fetch_one(&self.pool).await?;

        Ok(Task {
            id: row.get("id"),
            title: row.get("title"),
            description: row.get("description"),
            status: parse_status(row.get("status")),
            command: row.get("command"),
            input: row.get("input"),
            output: row.get("output"),
            logs: row.get("logs"),
            exit_code: row.get("exit_code"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        })
    }

    async fn delete_task(&self, id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM tasks WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    async fn can_run(&self, _task: &Task) -> bool {
        // Default implementation: can run any task
        // Override this in specific implementations based on device capabilities
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
