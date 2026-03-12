use crate::error::{Error, Result};

/// Represents the status of a single migration
#[derive(Debug, Clone)]
pub struct MigrationStatus {
    pub version: i64,
    pub name: String,
    pub applied: bool,
    pub applied_at: Option<i64>,
}

/// Trait for database backends that support migrations
pub trait MigrationBackend {
    /// Initialize the migrations tracking table
    fn init(&self) -> Result<()>;

    /// Get the current schema version (highest applied migration)
    fn current_version(&self) -> Result<i64>;

    /// Check if a specific migration has been applied
    fn is_applied(&self, version: i64) -> Result<bool>;

    /// Record a migration as applied
    fn mark_applied(&self, version: i64) -> Result<()>;

    /// Record a migration as rolled back
    fn mark_rolled_back(&self, version: i64) -> Result<()>;

    /// Execute raw SQL within a transaction
    fn execute_sql(&self, sql: &str) -> Result<()>;

    /// Begin a transaction
    fn begin_transaction(&self) -> Result<()>;

    /// Commit the current transaction
    fn commit_transaction(&self) -> Result<()>;

    /// Rollback the current transaction
    fn rollback_transaction(&self) -> Result<()>;

    /// Get list of all applied migrations
    fn applied_migrations(&self) -> Result<Vec<(i64, i64)>>; // (version, applied_at)
}

/// A single database migration
pub trait Migration: Send + Sync {
    /// Unique version number (must be sequential)
    fn version(&self) -> i64;

    /// Human-readable name
    fn name(&self) -> &str;

    /// SQL to apply the migration
    fn up(&self) -> &str;

    /// SQL to rollback the migration (optional)
    fn down(&self) -> Option<&str> {
        None
    }
}

/// Simple migration implementation using SQL strings
pub struct SqlMigration {
    pub version: i64,
    pub name: String,
    pub up_sql: String,
    pub down_sql: Option<String>,
}

impl SqlMigration {
    pub fn new(version: i64, name: impl Into<String>, up_sql: impl Into<String>) -> Self {
        Self {
            version,
            name: name.into(),
            up_sql: up_sql.into(),
            down_sql: None,
        }
    }

    pub fn with_down(mut self, down_sql: impl Into<String>) -> Self {
        self.down_sql = Some(down_sql.into());
        self
    }
}

impl Migration for SqlMigration {
    fn version(&self) -> i64 {
        self.version
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn up(&self) -> &str {
        &self.up_sql
    }

    fn down(&self) -> Option<&str> {
        self.down_sql.as_deref()
    }
}

/// Runs migrations against a database backend
pub struct MigrationRunner<B: MigrationBackend> {
    backend: B,
    migrations: Vec<Box<dyn Migration>>,
}

impl<B: MigrationBackend> MigrationRunner<B> {
    pub fn new(backend: B) -> Self {
        Self {
            backend,
            migrations: Vec::new(),
        }
    }

    /// Consume the runner and return the backend
    pub fn into_backend(self) -> B {
        self.backend
    }

    /// Add a migration to the runner
    pub fn add_migration<M: Migration + 'static>(mut self, migration: M) -> Self {
        self.migrations.push(Box::new(migration));
        self
    }

    /// Add multiple migrations
    pub fn add_migrations<I, M>(mut self, migrations: I) -> Self
    where
        I: IntoIterator<Item = M>,
        M: Migration + 'static,
    {
        for m in migrations {
            self.migrations.push(Box::new(m));
        }
        self
    }

    /// Initialize backend and validate migrations
    pub fn init(&self) -> Result<()> {
        self.backend.init()?;
        self.validate_migrations()?;
        Ok(())
    }

    /// Validate migration versions are sequential and unique
    fn validate_migrations(&self) -> Result<()> {
        let mut versions: Vec<i64> = self.migrations.iter().map(|m| m.version()).collect();
        versions.sort();

        for (i, &version) in versions.iter().enumerate() {
            let expected = (i + 1) as i64;
            if version != expected {
                return Err(Error::InvalidOrder(format!(
                    "Expected migration version {}, found {}",
                    expected, version
                )));
            }
        }

        Ok(())
    }

    /// Get current schema version
    pub fn current_version(&self) -> Result<i64> {
        self.backend.current_version()
    }

    /// Get status of all migrations
    pub fn status(&self) -> Result<Vec<MigrationStatus>> {
        let applied = self.backend.applied_migrations()?;
        let applied_map: std::collections::HashMap<i64, i64> = applied.into_iter().collect();

        let mut statuses: Vec<MigrationStatus> = self
            .migrations
            .iter()
            .map(|m| {
                let version = m.version();
                let applied_at = applied_map.get(&version).copied();
                MigrationStatus {
                    version,
                    name: m.name().to_string(),
                    applied: applied_at.is_some(),
                    applied_at,
                }
            })
            .collect();

        statuses.sort_by_key(|s| s.version);
        Ok(statuses)
    }

    /// Get pending migrations
    pub fn pending(&self) -> Result<Vec<&dyn Migration>> {
        let current = self.backend.current_version()?;
        let mut pending: Vec<&dyn Migration> = self
            .migrations
            .iter()
            .filter(|m| m.version() > current)
            .map(|m| m.as_ref())
            .collect();

        pending.sort_by_key(|m| m.version());
        Ok(pending)
    }

    /// Run all pending migrations
    pub fn migrate(&self) -> Result<usize> {
        let pending = self.pending()?;
        let count = pending.len();

        for migration in pending {
            self.apply_migration(migration)?;
        }

        Ok(count)
    }

    /// Run migrations up to a specific version
    pub fn migrate_to(&self, target_version: i64) -> Result<usize> {
        let current = self.backend.current_version()?;
        let mut count = 0;

        if target_version > current {
            // Migrate up
            let to_apply: Vec<&dyn Migration> = self
                .migrations
                .iter()
                .filter(|m| m.version() > current && m.version() <= target_version)
                .map(|m| m.as_ref())
                .collect();

            for migration in to_apply {
                self.apply_migration(migration)?;
                count += 1;
            }
        } else if target_version < current {
            // Migrate down
            let mut to_rollback: Vec<&dyn Migration> = self
                .migrations
                .iter()
                .filter(|m| m.version() > target_version && m.version() <= current)
                .map(|m| m.as_ref())
                .collect();

            to_rollback.sort_by_key(|m| std::cmp::Reverse(m.version()));

            for migration in to_rollback {
                self.rollback_migration(migration)?;
                count += 1;
            }
        }

        Ok(count)
    }

    /// Apply a single migration
    fn apply_migration(&self, migration: &dyn Migration) -> Result<()> {
        let version = migration.version();

        self.backend.begin_transaction()?;

        match self.backend.execute_sql(migration.up()) {
            Ok(()) => {
                self.backend.mark_applied(version)?;
                self.backend.commit_transaction()?;
                Ok(())
            }
            Err(e) => {
                let _ = self.backend.rollback_transaction();
                Err(Error::MigrationFailed {
                    version,
                    message: e.to_string(),
                })
            }
        }
    }

    /// Rollback a single migration
    fn rollback_migration(&self, migration: &dyn Migration) -> Result<()> {
        let version = migration.version();

        let down_sql = migration.down().ok_or(Error::MigrationFailed {
            version,
            message: "No rollback SQL defined".to_string(),
        })?;

        self.backend.begin_transaction()?;

        match self.backend.execute_sql(down_sql) {
            Ok(()) => {
                self.backend.mark_rolled_back(version)?;
                self.backend.commit_transaction()?;
                Ok(())
            }
            Err(e) => {
                let _ = self.backend.rollback_transaction();
                Err(Error::MigrationFailed {
                    version,
                    message: e.to_string(),
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sql_migration_builder() {
        let migration = SqlMigration::new(1, "create_users", "CREATE TABLE users (id INTEGER)")
            .with_down("DROP TABLE users");

        assert_eq!(migration.version(), 1);
        assert_eq!(migration.name(), "create_users");
        assert_eq!(migration.up(), "CREATE TABLE users (id INTEGER)");
        assert_eq!(migration.down(), Some("DROP TABLE users"));
    }
}
