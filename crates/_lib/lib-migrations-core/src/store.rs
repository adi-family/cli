use crate::Result;

/// Record of an applied migration
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MigrationRecord {
    pub version: u64,
    pub name: String,
    pub applied_at: u64, // Unix timestamp
}

/// Storage backend for tracking applied migrations.
///
/// Implement this trait to store migration state in your preferred backend:
/// - SQLite, PostgreSQL, MySQL
/// - JSON/YAML file
/// - Redis, etcd
/// - In-memory (for testing)
///
/// The store is responsible for:
/// - Initializing any required schema/structure
/// - Recording when migrations are applied/rolled back
/// - Querying which migrations have been applied
pub trait MigrationStore {
    /// Initialize the store (create tables, files, etc.)
    fn init(&mut self) -> Result<()>;

    /// Get all applied migrations, sorted by version ascending
    fn applied(&self) -> Result<Vec<MigrationRecord>>;

    /// Check if a specific version has been applied
    fn is_applied(&self, version: u64) -> Result<bool> {
        Ok(self.applied()?.iter().any(|r| r.version == version))
    }

    /// Get the highest applied version (0 if none)
    fn current_version(&self) -> Result<u64> {
        Ok(self.applied()?.last().map(|r| r.version).unwrap_or(0))
    }

    /// Record a migration as applied
    fn mark_applied(&mut self, version: u64, name: &str) -> Result<()>;

    /// Record a migration as rolled back (remove from applied)
    fn mark_rolled_back(&mut self, version: u64) -> Result<()>;
}

/// In-memory store for testing
#[derive(Debug, Default)]
pub struct MemoryStore {
    records: Vec<MigrationRecord>,
}

impl MemoryStore {
    pub fn new() -> Self {
        Self::default()
    }
}

impl MigrationStore for MemoryStore {
    fn init(&mut self) -> Result<()> {
        Ok(())
    }

    fn applied(&self) -> Result<Vec<MigrationRecord>> {
        let mut records = self.records.clone();
        records.sort_by_key(|r| r.version);
        Ok(records)
    }

    fn mark_applied(&mut self, version: u64, name: &str) -> Result<()> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        self.records.push(MigrationRecord {
            version,
            name: name.to_string(),
            applied_at: now,
        });
        Ok(())
    }

    fn mark_rolled_back(&mut self, version: u64) -> Result<()> {
        self.records.retain(|r| r.version != version);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_store() {
        let mut store = MemoryStore::new();
        store.init().unwrap();

        assert_eq!(store.current_version().unwrap(), 0);
        assert!(!store.is_applied(1).unwrap());

        store.mark_applied(1, "first").unwrap();
        assert!(store.is_applied(1).unwrap());
        assert_eq!(store.current_version().unwrap(), 1);

        store.mark_applied(2, "second").unwrap();
        assert_eq!(store.current_version().unwrap(), 2);

        let applied = store.applied().unwrap();
        assert_eq!(applied.len(), 2);
        assert_eq!(applied[0].version, 1);
        assert_eq!(applied[1].version, 2);

        store.mark_rolled_back(2).unwrap();
        assert_eq!(store.current_version().unwrap(), 1);
        assert!(!store.is_applied(2).unwrap());
    }
}
