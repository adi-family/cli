//! Per-project runtime state database.
//!
//! Stores service runtime state (PIDs, status, timestamps, restart counts)
//! in a SQLite database at `.adi/hive/hive.db` within each project.
//! Replaces the old per-service PID file approach.

use crate::sqlite_backend::RuntimeState;
use anyhow::{anyhow, Context, Result};
use rusqlite::{params, Connection, OptionalExtension};
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};
use tracing::{debug, info, warn};

pub struct RuntimeDb {
    conn: Arc<Mutex<Connection>>,
}

impl RuntimeDb {
    /// Open the runtime database for a project (at `{project_root}/.adi/hive/hive.db`)
    pub fn open(project_root: &Path) -> Result<Self> {
        let db_dir = project_root.join(".adi/hive");
        std::fs::create_dir_all(&db_dir)
            .with_context(|| format!("Failed to create directory: {}", db_dir.display()))?;

        let db_path = db_dir.join("hive.db");
        let db = Self::open_at(&db_path)?;

        // Auto-migrate from PID files if they exist
        let pid_dir = db_dir.join("pids");
        if pid_dir.exists() {
            if let Err(e) = db.migrate_from_pid_files(&pid_dir) {
                warn!("Failed to migrate PID files: {}", e);
            }
        }

        Ok(db)
    }

    /// Open a runtime database at a specific path (for testing)
    pub fn open_at(path: &Path) -> Result<Self> {
        debug!(path = %path.display(), "Opening runtime database");

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).ok();
        }

        let conn = Connection::open(path)
            .with_context(|| format!("Failed to open runtime database: {}", path.display()))?;

        let db = Self {
            conn: Arc::new(Mutex::new(conn)),
        };
        db.init_schema()?;
        Ok(db)
    }

    fn init_schema(&self) -> Result<()> {
        let conn = self.conn.lock().map_err(|e| anyhow!("Lock error: {}", e))?;

        conn.execute_batch(
            r#"
            PRAGMA journal_mode=WAL;

            CREATE TABLE IF NOT EXISTS runtime_meta (
                key TEXT PRIMARY KEY,
                value TEXT
            );

            CREATE TABLE IF NOT EXISTS runtime_state (
                service_name TEXT PRIMARY KEY,
                state TEXT NOT NULL DEFAULT 'stopped',
                pid INTEGER,
                container_id TEXT,
                started_at TIMESTAMP,
                stopped_at TIMESTAMP,
                restart_count INTEGER DEFAULT 0,
                last_exit_code INTEGER,
                last_error TEXT
            );

            INSERT OR REPLACE INTO runtime_meta (key, value) VALUES ('schema_version', '1');
            "#,
        )
        .context("Failed to initialize runtime database schema")?;

        debug!("Runtime database schema initialized");
        Ok(())
    }

    pub fn save_pid(&self, service_name: &str, pid: u32) -> Result<()> {
        let conn = self.conn.lock().map_err(|e| anyhow!("Lock error: {}", e))?;
        conn.execute(
            r#"INSERT INTO runtime_state (service_name, state, pid, started_at, restart_count)
               VALUES (?1, 'running', ?2, datetime('now'), 0)
               ON CONFLICT(service_name) DO UPDATE SET
                   state = 'running',
                   pid = ?2,
                   started_at = datetime('now'),
                   stopped_at = NULL,
                   last_exit_code = NULL,
                   last_error = NULL"#,
            params![service_name, pid],
        )?;
        Ok(())
    }

    pub fn read_pid(&self, service_name: &str) -> Option<u32> {
        let conn = self.conn.lock().ok()?;
        conn.query_row(
            "SELECT pid FROM runtime_state WHERE service_name = ?1 AND pid IS NOT NULL",
            params![service_name],
            |row| {
                let pid: i64 = row.get(0)?;
                Ok(pid as u32)
            },
        )
        .optional()
        .ok()
        .flatten()
    }

    /// Clear the PID for a service (set to NULL without removing the row)
    pub fn clear_pid(&self, service_name: &str) -> Result<()> {
        let conn = self.conn.lock().map_err(|e| anyhow!("Lock error: {}", e))?;
        conn.execute(
            "UPDATE runtime_state SET pid = NULL WHERE service_name = ?1",
            params![service_name],
        )?;
        Ok(())
    }

    pub fn update_state(&self, service_name: &str, state: &RuntimeState) -> Result<()> {
        let conn = self.conn.lock().map_err(|e| anyhow!("Lock error: {}", e))?;
        conn.execute(
            r#"INSERT INTO runtime_state
               (service_name, state, pid, container_id, started_at, stopped_at, restart_count, last_exit_code, last_error)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
               ON CONFLICT(service_name) DO UPDATE SET
                   state = ?2, pid = ?3, container_id = ?4, started_at = ?5,
                   stopped_at = ?6, restart_count = ?7, last_exit_code = ?8, last_error = ?9"#,
            params![
                service_name,
                state.state,
                state.pid,
                state.container_id,
                state.started_at,
                state.stopped_at,
                state.restart_count,
                state.last_exit_code,
                state.last_error
            ],
        )?;
        Ok(())
    }

    pub fn get_state(&self, service_name: &str) -> Result<Option<RuntimeState>> {
        let conn = self.conn.lock().map_err(|e| anyhow!("Lock error: {}", e))?;
        conn.query_row(
            r#"SELECT state, pid, container_id, started_at, stopped_at,
                      restart_count, last_exit_code, last_error
               FROM runtime_state WHERE service_name = ?1"#,
            params![service_name],
            |row| {
                Ok(RuntimeState {
                    state: row.get(0)?,
                    pid: row.get(1)?,
                    container_id: row.get(2)?,
                    started_at: row.get(3)?,
                    stopped_at: row.get(4)?,
                    restart_count: row.get(5)?,
                    last_exit_code: row.get(6)?,
                    last_error: row.get(7)?,
                })
            },
        )
        .optional()
        .context("Failed to query runtime state")
    }

    pub fn get_all_states(&self) -> Result<HashMap<String, RuntimeState>> {
        let conn = self.conn.lock().map_err(|e| anyhow!("Lock error: {}", e))?;
        let mut stmt = conn.prepare(
            r#"SELECT service_name, state, pid, container_id, started_at, stopped_at,
                      restart_count, last_exit_code, last_error
               FROM runtime_state"#,
        )?;

        let rows = stmt.query_map([], |row| {
            let name: String = row.get(0)?;
            let state = RuntimeState {
                state: row.get(1)?,
                pid: row.get(2)?,
                container_id: row.get(3)?,
                started_at: row.get(4)?,
                stopped_at: row.get(5)?,
                restart_count: row.get(6)?,
                last_exit_code: row.get(7)?,
                last_error: row.get(8)?,
            };
            Ok((name, state))
        })?;

        let mut result = HashMap::new();
        for row in rows {
            let (name, state) = row?;
            result.insert(name, state);
        }
        Ok(result)
    }

    /// Clear all runtime state (e.g., on daemon restart)
    pub fn clear_all(&self) -> Result<()> {
        let conn = self.conn.lock().map_err(|e| anyhow!("Lock error: {}", e))?;
        conn.execute("DELETE FROM runtime_state", [])?;
        debug!("Cleared all runtime state");
        Ok(())
    }

    /// Migrate from old PID files into SQLite
    fn migrate_from_pid_files(&self, pid_dir: &Path) -> Result<()> {
        let entries = std::fs::read_dir(pid_dir)
            .with_context(|| format!("Failed to read PID directory: {}", pid_dir.display()))?;

        let mut migrated = 0;
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map(|e| e == "pid").unwrap_or(false) {
                let service_name = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or_default();

                if service_name.is_empty() {
                    continue;
                }

                if let Ok(content) = std::fs::read_to_string(&path) {
                    if let Ok(pid) = content.trim().parse::<u32>() {
                        // Only import if process is still alive
                        if is_pid_running(pid) {
                            self.save_pid(service_name, pid)?;
                            migrated += 1;
                        }
                    }
                }
            }
        }

        if migrated > 0 {
            info!("Migrated {} PID files to runtime database", migrated);
        }

        // Remove the PID directory after migration
        std::fs::remove_dir_all(pid_dir).ok();
        Ok(())
    }
}

/// Check if a process with given PID is running
fn is_pid_running(pid: u32) -> bool {
    #[cfg(unix)]
    {
        unsafe { libc::kill(pid as i32, 0) == 0 }
    }
    #[cfg(not(unix))]
    {
        let _ = pid;
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_runtime_db_init() {
        let dir = tempdir().unwrap();
        let db = RuntimeDb::open_at(&dir.path().join("test.db")).unwrap();
        // Should be able to read empty state
        assert!(db.get_all_states().unwrap().is_empty());
    }

    #[test]
    fn test_save_and_read_pid() {
        let dir = tempdir().unwrap();
        let db = RuntimeDb::open_at(&dir.path().join("test.db")).unwrap();

        db.save_pid("web", 12345).unwrap();
        assert_eq!(db.read_pid("web"), Some(12345));
        assert_eq!(db.read_pid("nonexistent"), None);
    }

    #[test]
    fn test_clear_pid() {
        let dir = tempdir().unwrap();
        let db = RuntimeDb::open_at(&dir.path().join("test.db")).unwrap();

        db.save_pid("web", 12345).unwrap();
        db.clear_pid("web").unwrap();
        assert_eq!(db.read_pid("web"), None);

        // Row should still exist with state
        let state = db.get_state("web").unwrap().unwrap();
        assert_eq!(state.state, "running");
    }

    #[test]
    fn test_update_and_get_state() {
        let dir = tempdir().unwrap();
        let db = RuntimeDb::open_at(&dir.path().join("test.db")).unwrap();

        let state = RuntimeState {
            state: "running".to_string(),
            pid: Some(42),
            container_id: None,
            started_at: Some("2025-01-01T00:00:00Z".to_string()),
            stopped_at: None,
            restart_count: 2,
            last_exit_code: None,
            last_error: None,
        };
        db.update_state("api", &state).unwrap();

        let loaded = db.get_state("api").unwrap().unwrap();
        assert_eq!(loaded.state, "running");
        assert_eq!(loaded.pid, Some(42));
        assert_eq!(loaded.restart_count, 2);
    }

    #[test]
    fn test_get_all_states() {
        let dir = tempdir().unwrap();
        let db = RuntimeDb::open_at(&dir.path().join("test.db")).unwrap();

        db.save_pid("web", 100).unwrap();
        db.save_pid("api", 200).unwrap();

        let states = db.get_all_states().unwrap();
        assert_eq!(states.len(), 2);
        assert!(states.contains_key("web"));
        assert!(states.contains_key("api"));
    }

    #[test]
    fn test_clear_all() {
        let dir = tempdir().unwrap();
        let db = RuntimeDb::open_at(&dir.path().join("test.db")).unwrap();

        db.save_pid("web", 100).unwrap();
        db.save_pid("api", 200).unwrap();
        db.clear_all().unwrap();

        assert!(db.get_all_states().unwrap().is_empty());
    }

    #[test]
    fn test_migrate_from_pid_files() {
        let dir = tempdir().unwrap();
        let pid_dir = dir.path().join("pids");
        std::fs::create_dir_all(&pid_dir).unwrap();

        // Write a PID file with current process PID (guaranteed to be running)
        let current_pid = std::process::id();
        std::fs::write(pid_dir.join("self.pid"), current_pid.to_string()).unwrap();

        // Write a PID file with a non-existent PID
        std::fs::write(pid_dir.join("dead.pid"), "999999999").unwrap();

        let db = RuntimeDb::open_at(&dir.path().join("test.db")).unwrap();
        db.migrate_from_pid_files(&pid_dir).unwrap();

        // Only the running process should be migrated
        assert_eq!(db.read_pid("self"), Some(current_pid));
        assert_eq!(db.read_pid("dead"), None);

        // PID directory should be removed
        assert!(!pid_dir.exists());
    }
}
