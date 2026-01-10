use crate::error::{AgentError, Result};
use crate::migrations::migrations;
use crate::storage::{SessionCounts, SessionStorage};
use crate::types::{LoopConfig, LoopState};

use super::session::{Session, SessionId, SessionStatus, SessionSummary};
use chrono::{TimeZone, Utc};
use lib_migrations::{MigrationRunner, SqliteMigrationBackend};
use rusqlite::{params, Connection};
use std::path::Path;
use std::sync::Mutex;

/// SQLite-based session storage
pub struct SqliteSessionStorage {
    conn: Mutex<Connection>,
}

impl SqliteSessionStorage {
    /// Open a SQLite database at the given path, running migrations if needed
    pub fn open(path: &Path) -> Result<Self> {
        let backend = SqliteMigrationBackend::open(path)
            .map_err(|e| AgentError::Storage(format!("Failed to open database: {}", e)))?;

        let runner = MigrationRunner::new(backend).add_migrations(migrations());

        runner
            .init()
            .map_err(|e| AgentError::Storage(format!("Migration init failed: {}", e)))?;

        let applied = runner
            .migrate()
            .map_err(|e| AgentError::Storage(format!("Migration failed: {}", e)))?;

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

    /// Open an in-memory database (for testing)
    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;

        // Run migrations manually for in-memory
        for migration in migrations() {
            conn.execute_batch(&migration.up_sql)?;
        }

        conn.execute_batch(
            "PRAGMA journal_mode=MEMORY;
             PRAGMA synchronous=OFF;
             PRAGMA foreign_keys=ON;",
        )?;

        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    fn row_to_session(row: &rusqlite::Row) -> rusqlite::Result<Session> {
        let id: String = row.get(0)?;
        let title: String = row.get(1)?;
        let description: Option<String> = row.get(2)?;
        let status_str: String = row.get(3)?;
        let project_path: Option<String> = row.get(4)?;
        let system_prompt: Option<String> = row.get(5)?;
        let messages_json: String = row.get(6)?;
        let loop_config_json: String = row.get(7)?;
        let loop_state_json: String = row.get(8)?;
        let error_message: Option<String> = row.get(9)?;
        let metadata_json: String = row.get(10)?;
        let created_at_ts: i64 = row.get(11)?;
        let updated_at_ts: i64 = row.get(12)?;

        let status = SessionStatus::parse(&status_str).unwrap_or(SessionStatus::Active);
        let messages = serde_json::from_str(&messages_json).unwrap_or_default();
        let loop_config: LoopConfig = serde_json::from_str(&loop_config_json).unwrap_or_default();
        let loop_state: LoopState = serde_json::from_str(&loop_state_json).unwrap_or_default();
        let metadata: serde_json::Value =
            serde_json::from_str(&metadata_json).unwrap_or(serde_json::Value::Null);

        Ok(Session {
            id: SessionId::from_string(id),
            title,
            description,
            status,
            project_path,
            system_prompt,
            messages,
            loop_config,
            loop_state,
            error_message,
            metadata,
            created_at: Utc.timestamp_opt(created_at_ts, 0).unwrap(),
            updated_at: Utc.timestamp_opt(updated_at_ts, 0).unwrap(),
        })
    }

    fn row_to_summary(row: &rusqlite::Row) -> rusqlite::Result<SessionSummary> {
        let id: String = row.get(0)?;
        let title: String = row.get(1)?;
        let description: Option<String> = row.get(2)?;
        let status_str: String = row.get(3)?;
        let project_path: Option<String> = row.get(4)?;
        let messages_json: String = row.get(5)?;
        let error_message: Option<String> = row.get(6)?;
        let created_at_ts: i64 = row.get(7)?;
        let updated_at_ts: i64 = row.get(8)?;

        let status = SessionStatus::parse(&status_str).unwrap_or(SessionStatus::Active);
        let messages: Vec<crate::types::Message> =
            serde_json::from_str(&messages_json).unwrap_or_default();

        Ok(SessionSummary {
            id: SessionId::from_string(id),
            title,
            description,
            status,
            project_path,
            message_count: messages.len(),
            total_tokens: messages.iter().map(|m| m.estimated_tokens()).sum(),
            error_message,
            created_at: Utc.timestamp_opt(created_at_ts, 0).unwrap(),
            updated_at: Utc.timestamp_opt(updated_at_ts, 0).unwrap(),
        })
    }
}

impl SessionStorage for SqliteSessionStorage {
    fn create_session(&self, session: &Session) -> Result<SessionId> {
        let conn = self.conn.lock().unwrap();

        let messages_json = serde_json::to_string(&session.messages)?;
        let loop_config_json = serde_json::to_string(&session.loop_config)?;
        let loop_state_json = serde_json::to_string(&session.loop_state)?;
        let metadata_json = serde_json::to_string(&session.metadata)?;

        conn.execute(
            r#"INSERT INTO sessions (id, title, description, status, project_path, system_prompt,
                messages, loop_config, loop_state, error_message, metadata, created_at, updated_at)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)"#,
            params![
                session.id.0,
                session.title,
                session.description,
                session.status.as_str(),
                session.project_path,
                session.system_prompt,
                messages_json,
                loop_config_json,
                loop_state_json,
                session.error_message,
                metadata_json,
                session.created_at.timestamp(),
                session.updated_at.timestamp(),
            ],
        )?;

        Ok(session.id.clone())
    }

    fn get_session(&self, id: &SessionId) -> Result<Session> {
        let conn = self.conn.lock().unwrap();

        conn.query_row(
            r#"SELECT id, title, description, status, project_path, system_prompt,
                      messages, loop_config, loop_state, error_message, metadata,
                      created_at, updated_at
               FROM sessions WHERE id = ?1"#,
            params![id.0],
            Self::row_to_session,
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => AgentError::SessionNotFound(id.0.clone()),
            _ => AgentError::Sqlite(e),
        })
    }

    fn update_session(&self, session: &Session) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        let messages_json = serde_json::to_string(&session.messages)?;
        let loop_config_json = serde_json::to_string(&session.loop_config)?;
        let loop_state_json = serde_json::to_string(&session.loop_state)?;
        let metadata_json = serde_json::to_string(&session.metadata)?;

        let rows = conn.execute(
            r#"UPDATE sessions SET
                title = ?1, description = ?2, status = ?3, project_path = ?4,
                system_prompt = ?5, messages = ?6, loop_config = ?7, loop_state = ?8,
                error_message = ?9, metadata = ?10, updated_at = ?11
               WHERE id = ?12"#,
            params![
                session.title,
                session.description,
                session.status.as_str(),
                session.project_path,
                session.system_prompt,
                messages_json,
                loop_config_json,
                loop_state_json,
                session.error_message,
                metadata_json,
                Utc::now().timestamp(),
                session.id.0,
            ],
        )?;

        if rows == 0 {
            return Err(AgentError::SessionNotFound(session.id.0.clone()));
        }

        Ok(())
    }

    fn delete_session(&self, id: &SessionId) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        let rows = conn.execute("DELETE FROM sessions WHERE id = ?1", params![id.0])?;

        if rows == 0 {
            return Err(AgentError::SessionNotFound(id.0.clone()));
        }

        Ok(())
    }

    fn list_sessions(&self, project_path: Option<&str>) -> Result<Vec<SessionSummary>> {
        let conn = self.conn.lock().unwrap();

        let sql = r#"SELECT id, title, description, status, project_path, messages,
                            error_message, created_at, updated_at
                     FROM sessions"#;

        match project_path {
            Some(path) => {
                let mut stmt = conn.prepare(&format!(
                    "{} WHERE project_path = ?1 ORDER BY updated_at DESC",
                    sql
                ))?;
                let sessions = stmt
                    .query_map(params![path], Self::row_to_summary)?
                    .collect::<rusqlite::Result<Vec<_>>>()?;
                Ok(sessions)
            }
            None => {
                let mut stmt = conn.prepare(&format!("{} ORDER BY updated_at DESC", sql))?;
                let sessions = stmt
                    .query_map([], Self::row_to_summary)?
                    .collect::<rusqlite::Result<Vec<_>>>()?;
                Ok(sessions)
            }
        }
    }

    fn list_sessions_by_status(&self, status: SessionStatus) -> Result<Vec<SessionSummary>> {
        let conn = self.conn.lock().unwrap();

        let mut stmt = conn.prepare(
            r#"SELECT id, title, description, status, project_path, messages,
                      error_message, created_at, updated_at
               FROM sessions WHERE status = ?1 ORDER BY updated_at DESC"#,
        )?;

        let sessions = stmt
            .query_map(params![status.as_str()], Self::row_to_summary)?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        Ok(sessions)
    }

    fn search_sessions(&self, query: &str, limit: usize) -> Result<Vec<SessionSummary>> {
        let conn = self.conn.lock().unwrap();

        let mut stmt = conn.prepare(
            r#"SELECT s.id, s.title, s.description, s.status, s.project_path, s.messages,
                      s.error_message, s.created_at, s.updated_at
               FROM sessions s
               JOIN sessions_fts fts ON s.rowid = fts.rowid
               WHERE sessions_fts MATCH ?1
               ORDER BY rank
               LIMIT ?2"#,
        )?;

        let sessions = stmt
            .query_map(params![query, limit as i64], Self::row_to_summary)?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        Ok(sessions)
    }

    fn get_session_counts(&self) -> Result<SessionCounts> {
        let conn = self.conn.lock().unwrap();

        let total: u64 = conn.query_row("SELECT COUNT(*) FROM sessions", [], |row| row.get(0))?;

        let active: u64 = conn.query_row(
            "SELECT COUNT(*) FROM sessions WHERE status = 'active'",
            [],
            |row| row.get(0),
        )?;

        let paused: u64 = conn.query_row(
            "SELECT COUNT(*) FROM sessions WHERE status = 'paused'",
            [],
            |row| row.get(0),
        )?;

        let completed: u64 = conn.query_row(
            "SELECT COUNT(*) FROM sessions WHERE status = 'completed'",
            [],
            |row| row.get(0),
        )?;

        let failed: u64 = conn.query_row(
            "SELECT COUNT(*) FROM sessions WHERE status = 'failed'",
            [],
            |row| row.get(0),
        )?;

        let archived: u64 = conn.query_row(
            "SELECT COUNT(*) FROM sessions WHERE status = 'archived'",
            [],
            |row| row.get(0),
        )?;

        Ok(SessionCounts {
            total,
            active,
            paused,
            completed,
            failed,
            archived,
        })
    }

    fn archive_old_sessions(&self, older_than_days: u32) -> Result<usize> {
        let conn = self.conn.lock().unwrap();

        let cutoff = Utc::now().timestamp() - (older_than_days as i64 * 24 * 60 * 60);

        let rows = conn.execute(
            r#"UPDATE sessions SET status = 'archived', updated_at = ?1
               WHERE status IN ('completed', 'failed')
                 AND updated_at < ?2"#,
            params![Utc::now().timestamp(), cutoff],
        )?;

        Ok(rows)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Message;

    fn create_test_storage() -> SqliteSessionStorage {
        SqliteSessionStorage::open_in_memory().unwrap()
    }

    #[test]
    fn test_create_and_get_session() {
        let storage = create_test_storage();

        let session = Session::new("Test Session")
            .with_description("A test")
            .with_project_path("/test/project");

        let id = storage.create_session(&session).unwrap();

        let retrieved = storage.get_session(&id).unwrap();
        assert_eq!(retrieved.title, "Test Session");
        assert_eq!(retrieved.description, Some("A test".to_string()));
        assert_eq!(retrieved.project_path, Some("/test/project".to_string()));
        assert_eq!(retrieved.status, SessionStatus::Active);
    }

    #[test]
    fn test_update_session() {
        let storage = create_test_storage();

        let mut session = Session::new("Original Title");
        let id = storage.create_session(&session).unwrap();

        session.title = "Updated Title".to_string();
        session.add_message(Message::user("Hello"));
        storage.update_session(&session).unwrap();

        let retrieved = storage.get_session(&id).unwrap();
        assert_eq!(retrieved.title, "Updated Title");
        assert_eq!(retrieved.messages.len(), 1);
    }

    #[test]
    fn test_delete_session() {
        let storage = create_test_storage();

        let session = Session::new("To Delete");
        let id = storage.create_session(&session).unwrap();

        storage.delete_session(&id).unwrap();

        let result = storage.get_session(&id);
        assert!(result.is_err());
    }

    #[test]
    fn test_list_sessions() {
        let storage = create_test_storage();

        let session1 = Session::new("Session 1").with_project_path("/project/a");
        let session2 = Session::new("Session 2").with_project_path("/project/b");
        let session3 = Session::new("Session 3").with_project_path("/project/a");

        storage.create_session(&session1).unwrap();
        storage.create_session(&session2).unwrap();
        storage.create_session(&session3).unwrap();

        // List all
        let all = storage.list_sessions(None).unwrap();
        assert_eq!(all.len(), 3);

        // List by project
        let project_a = storage.list_sessions(Some("/project/a")).unwrap();
        assert_eq!(project_a.len(), 2);
    }

    #[test]
    fn test_list_by_status() {
        let storage = create_test_storage();

        let session1 = Session::new("Active");
        let mut session2 = Session::new("Completed");
        session2.complete();
        let mut session3 = Session::new("Failed");
        session3.fail("Error!");

        storage.create_session(&session1).unwrap();
        storage.create_session(&session2).unwrap();
        storage.create_session(&session3).unwrap();

        let active = storage
            .list_sessions_by_status(SessionStatus::Active)
            .unwrap();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].title, "Active");

        let completed = storage
            .list_sessions_by_status(SessionStatus::Completed)
            .unwrap();
        assert_eq!(completed.len(), 1);
    }

    #[test]
    fn test_session_counts() {
        let storage = create_test_storage();

        let session1 = Session::new("Active");
        let mut session2 = Session::new("Completed");
        session2.complete();
        let mut session3 = Session::new("Paused");
        session3.pause();

        storage.create_session(&session1).unwrap();
        storage.create_session(&session2).unwrap();
        storage.create_session(&session3).unwrap();

        let counts = storage.get_session_counts().unwrap();
        assert_eq!(counts.total, 3);
        assert_eq!(counts.active, 1);
        assert_eq!(counts.completed, 1);
        assert_eq!(counts.paused, 1);
    }

    #[test]
    fn test_session_not_found() {
        let storage = create_test_storage();

        let result = storage.get_session(&SessionId::new());
        assert!(matches!(result, Err(AgentError::SessionNotFound(_))));
    }

    #[test]
    fn test_session_with_messages() {
        let storage = create_test_storage();

        let mut session = Session::new("Chat Session").with_system_prompt("You are helpful");

        session.add_message(Message::user("Hello"));
        session.add_message(Message::assistant("Hi there!"));

        let id = storage.create_session(&session).unwrap();

        let retrieved = storage.get_session(&id).unwrap();
        assert_eq!(retrieved.messages.len(), 2);
        assert_eq!(retrieved.system_prompt, Some("You are helpful".to_string()));
    }
}
