use crate::error::Result;
use crate::runner::MigrationBackend;
use rusqlite::Connection;
use std::sync::Mutex;

/// SQLite implementation of MigrationBackend
pub struct SqliteMigrationBackend {
    conn: Mutex<Connection>,
    table_name: String,
}

impl SqliteMigrationBackend {
    /// Create a new backend wrapping an existing connection
    pub fn new(conn: Connection) -> Self {
        Self {
            conn: Mutex::new(conn),
            table_name: "schema_migrations".to_string(),
        }
    }

    /// Create with a custom migrations table name
    pub fn with_table_name(conn: Connection, table_name: impl Into<String>) -> Self {
        Self {
            conn: Mutex::new(conn),
            table_name: table_name.into(),
        }
    }

    /// Open a SQLite database file
    pub fn open(path: &std::path::Path) -> Result<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;")?;
        Ok(Self::new(conn))
    }

    /// Create an in-memory database (useful for testing)
    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        Ok(Self::new(conn))
    }

    /// Get reference to the underlying connection (for running additional queries)
    pub fn connection(&self) -> &Mutex<Connection> {
        &self.conn
    }

    /// Take ownership of the connection
    pub fn into_connection(self) -> Connection {
        self.conn.into_inner().unwrap()
    }
}

impl MigrationBackend for SqliteMigrationBackend {
    fn init(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            &format!(
                "CREATE TABLE IF NOT EXISTS {} (
                    version INTEGER PRIMARY KEY,
                    applied_at INTEGER NOT NULL
                )",
                self.table_name
            ),
            [],
        )?;
        Ok(())
    }

    fn current_version(&self) -> Result<i64> {
        let conn = self.conn.lock().unwrap();
        let version: Option<i64> = conn
            .query_row(
                &format!("SELECT MAX(version) FROM {}", self.table_name),
                [],
                |row| row.get(0),
            )
            .unwrap_or(None);

        Ok(version.unwrap_or(0))
    }

    fn is_applied(&self, version: i64) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        let count: i64 = conn.query_row(
            &format!(
                "SELECT COUNT(*) FROM {} WHERE version = ?1",
                self.table_name
            ),
            [version],
            |row| row.get(0),
        )?;

        Ok(count > 0)
    }

    fn mark_applied(&self, version: i64) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        conn.execute(
            &format!(
                "INSERT INTO {} (version, applied_at) VALUES (?1, ?2)",
                self.table_name
            ),
            [version, timestamp],
        )?;

        Ok(())
    }

    fn mark_rolled_back(&self, version: i64) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            &format!("DELETE FROM {} WHERE version = ?1", self.table_name),
            [version],
        )?;

        Ok(())
    }

    fn execute_sql(&self, sql: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch(sql)?;
        Ok(())
    }

    fn begin_transaction(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("BEGIN TRANSACTION", [])?;
        Ok(())
    }

    fn commit_transaction(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("COMMIT", [])?;
        Ok(())
    }

    fn rollback_transaction(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("ROLLBACK", [])?;
        Ok(())
    }

    fn applied_migrations(&self) -> Result<Vec<(i64, i64)>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(&format!(
            "SELECT version, applied_at FROM {} ORDER BY version",
            self.table_name
        ))?;

        let rows = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(rows)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runner::{MigrationRunner, SqlMigration};

    fn create_test_migrations() -> Vec<SqlMigration> {
        vec![
            SqlMigration::new(
                1,
                "create_users",
                "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT NOT NULL)",
            )
            .with_down("DROP TABLE users"),
            SqlMigration::new(
                2,
                "create_orders",
                "CREATE TABLE orders (id INTEGER PRIMARY KEY, user_id INTEGER)",
            )
            .with_down("DROP TABLE orders"),
            SqlMigration::new(
                3,
                "create_posts",
                "CREATE TABLE posts (id INTEGER PRIMARY KEY, user_id INTEGER, title TEXT)",
            )
            .with_down("DROP TABLE posts"),
        ]
    }

    #[test]
    fn test_backend_init() {
        let backend = SqliteMigrationBackend::open_in_memory().unwrap();
        backend.init().unwrap();

        let version = backend.current_version().unwrap();
        assert_eq!(version, 0);
    }

    #[test]
    fn test_migration_runner_migrate_all() {
        let backend = SqliteMigrationBackend::open_in_memory().unwrap();
        let migrations = create_test_migrations();

        let runner = MigrationRunner::new(backend).add_migrations(migrations);

        runner.init().unwrap();
        assert_eq!(runner.current_version().unwrap(), 0);

        let count = runner.migrate().unwrap();
        assert_eq!(count, 3);
        assert_eq!(runner.current_version().unwrap(), 3);

        // Running again should apply nothing
        let count = runner.migrate().unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_migration_status() {
        let backend = SqliteMigrationBackend::open_in_memory().unwrap();
        let migrations = create_test_migrations();

        let runner = MigrationRunner::new(backend).add_migrations(migrations);

        runner.init().unwrap();
        runner.migrate().unwrap();

        let statuses = runner.status().unwrap();
        assert_eq!(statuses.len(), 3);
        assert!(statuses.iter().all(|s| s.applied));
    }

    #[test]
    fn test_migrate_to_specific_version() {
        let backend = SqliteMigrationBackend::open_in_memory().unwrap();
        let migrations = create_test_migrations();

        let runner = MigrationRunner::new(backend).add_migrations(migrations);

        runner.init().unwrap();

        // Migrate to version 2
        let count = runner.migrate_to(2).unwrap();
        assert_eq!(count, 2);
        assert_eq!(runner.current_version().unwrap(), 2);

        // Verify pending
        let pending = runner.pending().unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].version(), 3);
    }

    #[test]
    fn test_rollback() {
        let backend = SqliteMigrationBackend::open_in_memory().unwrap();
        let migrations = create_test_migrations();

        let runner = MigrationRunner::new(backend).add_migrations(migrations);

        runner.init().unwrap();
        runner.migrate().unwrap();
        assert_eq!(runner.current_version().unwrap(), 3);

        // Rollback to version 1
        let count = runner.migrate_to(1).unwrap();
        assert_eq!(count, 2); // Rolled back migrations 3 and 2
        assert_eq!(runner.current_version().unwrap(), 1);
    }

    #[test]
    fn test_invalid_migration_order() {
        let backend = SqliteMigrationBackend::open_in_memory().unwrap();

        let runner = MigrationRunner::new(backend)
            .add_migration(SqlMigration::new(1, "first", "SELECT 1"))
            .add_migration(SqlMigration::new(3, "third", "SELECT 3")); // Missing 2

        let result = runner.init();
        assert!(result.is_err());
    }

    #[test]
    fn test_custom_table_name() {
        let conn = Connection::open_in_memory().unwrap();
        let backend = SqliteMigrationBackend::with_table_name(conn, "my_migrations");

        backend.init().unwrap();

        let version = backend.current_version().unwrap();
        assert_eq!(version, 0);
    }
}
