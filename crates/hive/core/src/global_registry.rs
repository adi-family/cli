//! Global project registry database.
//!
//! Stores paths to all known Hive projects in a SQLite database
//! at `~/.adi/hive/registry.db`. Replaces the old `sources.json` file.

use crate::hive_config::SourceType;
use anyhow::{anyhow, Context, Result};
use rusqlite::{params, Connection, OptionalExtension};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tracing::{debug, info, warn};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RegisteredSource {
    pub name: String,
    pub path: PathBuf,
    pub source_type: String,
    pub enabled: bool,
    pub last_accessed: Option<String>,
    pub created_at: Option<String>,
}

pub struct GlobalRegistry {
    conn: Arc<Mutex<Connection>>,
}

impl GlobalRegistry {
    /// Open the global registry (at `~/.adi/hive/registry.db`), auto-migrating from `sources.json`
    pub fn open() -> Result<Self> {
        let db_dir = default_registry_dir();
        std::fs::create_dir_all(&db_dir)
            .with_context(|| format!("Failed to create directory: {}", db_dir.display()))?;

        let db_path = db_dir.join("registry.db");
        let registry = Self::open_at(&db_path)?;

        // Auto-migrate from sources.json if it exists
        let json_path = db_dir.join("sources.json");
        if json_path.exists() {
            if let Err(e) = registry.migrate_from_json(&json_path) {
                warn!("Failed to migrate sources.json: {}", e);
            }
        }

        Ok(registry)
    }

    /// Open the registry at a specific path (for testing)
    pub fn open_at(path: &Path) -> Result<Self> {
        debug!(path = %path.display(), "Opening global registry");

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).ok();
        }

        let conn = Connection::open(path)
            .with_context(|| format!("Failed to open registry database: {}", path.display()))?;

        let registry = Self {
            conn: Arc::new(Mutex::new(conn)),
        };
        registry.init_schema()?;
        Ok(registry)
    }

    fn init_schema(&self) -> Result<()> {
        let conn = self.conn.lock().map_err(|e| anyhow!("Lock error: {}", e))?;

        conn.execute_batch(
            r#"
            PRAGMA journal_mode=WAL;

            CREATE TABLE IF NOT EXISTS registry_meta (
                key TEXT PRIMARY KEY,
                value TEXT
            );

            CREATE TABLE IF NOT EXISTS sources (
                name TEXT PRIMARY KEY,
                path TEXT NOT NULL UNIQUE,
                source_type TEXT NOT NULL DEFAULT 'yaml',
                enabled BOOLEAN NOT NULL DEFAULT 1,
                last_accessed TIMESTAMP,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            );

            INSERT OR REPLACE INTO registry_meta (key, value) VALUES ('schema_version', '1');
            "#,
        )
        .context("Failed to initialize registry schema")?;

        debug!("Global registry schema initialized");
        Ok(())
    }

    pub fn add_source(
        &self,
        name: &str,
        path: &Path,
        source_type: SourceType,
        enabled: bool,
    ) -> Result<()> {
        let conn = self.conn.lock().map_err(|e| anyhow!("Lock error: {}", e))?;
        let type_str = match source_type {
            SourceType::Yaml => "yaml",
            SourceType::Sqlite => "sqlite",
        };

        conn.execute(
            r#"INSERT INTO sources (name, path, source_type, enabled, last_accessed)
               VALUES (?1, ?2, ?3, ?4, datetime('now'))
               ON CONFLICT(name) DO UPDATE SET
                   path = ?2, source_type = ?3, enabled = ?4, last_accessed = datetime('now')"#,
            params![name, path.to_string_lossy().as_ref(), type_str, enabled],
        )?;

        debug!("Added source '{}' at {}", name, path.display());
        Ok(())
    }

    pub fn remove_source(&self, name: &str) -> Result<bool> {
        let conn = self.conn.lock().map_err(|e| anyhow!("Lock error: {}", e))?;
        let rows = conn.execute("DELETE FROM sources WHERE name = ?1", params![name])?;
        Ok(rows > 0)
    }

    pub fn set_enabled(&self, name: &str, enabled: bool) -> Result<()> {
        let conn = self.conn.lock().map_err(|e| anyhow!("Lock error: {}", e))?;
        conn.execute(
            "UPDATE sources SET enabled = ?1 WHERE name = ?2",
            params![enabled, name],
        )?;
        Ok(())
    }

    pub fn update_last_accessed(&self, name: &str) -> Result<()> {
        let conn = self.conn.lock().map_err(|e| anyhow!("Lock error: {}", e))?;
        conn.execute(
            "UPDATE sources SET last_accessed = datetime('now') WHERE name = ?1",
            params![name],
        )?;
        Ok(())
    }

    pub fn get_source(&self, name: &str) -> Result<Option<RegisteredSource>> {
        let conn = self.conn.lock().map_err(|e| anyhow!("Lock error: {}", e))?;
        conn.query_row(
            "SELECT name, path, source_type, enabled, last_accessed, created_at FROM sources WHERE name = ?1",
            params![name],
            |row| {
                Ok(RegisteredSource {
                    name: row.get(0)?,
                    path: PathBuf::from(row.get::<_, String>(1)?),
                    source_type: row.get(2)?,
                    enabled: row.get(3)?,
                    last_accessed: row.get(4)?,
                    created_at: row.get(5)?,
                })
            },
        )
        .optional()
        .context("Failed to query source")
    }

    pub fn get_source_by_path(&self, path: &Path) -> Result<Option<RegisteredSource>> {
        let conn = self.conn.lock().map_err(|e| anyhow!("Lock error: {}", e))?;
        conn.query_row(
            "SELECT name, path, source_type, enabled, last_accessed, created_at FROM sources WHERE path = ?1",
            params![path.to_string_lossy().as_ref()],
            |row| {
                Ok(RegisteredSource {
                    name: row.get(0)?,
                    path: PathBuf::from(row.get::<_, String>(1)?),
                    source_type: row.get(2)?,
                    enabled: row.get(3)?,
                    last_accessed: row.get(4)?,
                    created_at: row.get(5)?,
                })
            },
        )
        .optional()
        .context("Failed to query source by path")
    }

    pub fn list_sources(&self) -> Result<Vec<RegisteredSource>> {
        let conn = self.conn.lock().map_err(|e| anyhow!("Lock error: {}", e))?;
        let mut stmt = conn.prepare(
            "SELECT name, path, source_type, enabled, last_accessed, created_at FROM sources ORDER BY name",
        )?;

        let rows = stmt.query_map([], |row| {
            Ok(RegisteredSource {
                name: row.get(0)?,
                path: PathBuf::from(row.get::<_, String>(1)?),
                source_type: row.get(2)?,
                enabled: row.get(3)?,
                last_accessed: row.get(4)?,
                created_at: row.get(5)?,
            })
        })?;

        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    pub fn list_enabled_sources(&self) -> Result<Vec<RegisteredSource>> {
        let conn = self.conn.lock().map_err(|e| anyhow!("Lock error: {}", e))?;
        let mut stmt = conn.prepare(
            "SELECT name, path, source_type, enabled, last_accessed, created_at FROM sources WHERE enabled = 1 ORDER BY name",
        )?;

        let rows = stmt.query_map([], |row| {
            Ok(RegisteredSource {
                name: row.get(0)?,
                path: PathBuf::from(row.get::<_, String>(1)?),
                source_type: row.get(2)?,
                enabled: row.get(3)?,
                last_accessed: row.get(4)?,
                created_at: row.get(5)?,
            })
        })?;

        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    /// Migrate from the old sources.json file
    fn migrate_from_json(&self, json_path: &Path) -> Result<()> {
        let content = std::fs::read_to_string(json_path)
            .with_context(|| format!("Failed to read {}", json_path.display()))?;

        let saved: Vec<LegacySavedSource> = serde_json::from_str(&content)
            .with_context(|| "Failed to parse sources.json")?;

        let mut migrated = 0;
        for source in saved {
            let source_type = if source.path.join("hive.db").exists() {
                SourceType::Sqlite
            } else {
                SourceType::Yaml
            };

            if let Err(e) = self.add_source(&source.name, &source.path, source_type, source.enabled)
            {
                warn!("Failed to migrate source '{}': {}", source.name, e);
            } else {
                migrated += 1;
            }
        }

        if migrated > 0 {
            info!(
                "Migrated {} sources from {} to registry.db",
                migrated,
                json_path.display()
            );
        }

        // Rename old file for safety
        let backup_path = json_path.with_extension("json.bak");
        std::fs::rename(json_path, &backup_path).ok();

        Ok(())
    }
}

/// Legacy format from sources.json
#[derive(Debug, serde::Deserialize)]
struct LegacySavedSource {
    name: String,
    path: PathBuf,
    enabled: bool,
}

fn default_registry_dir() -> PathBuf {
    dirs::home_dir()
        .map(|h| h.join(".adi/hive"))
        .unwrap_or_else(|| PathBuf::from(".adi/hive"))
}

/// Read the sources registry without requiring the daemon.
/// Returns a map of source name -> source path.
/// This is a standalone sync function for CLI use.
pub fn read_sources_registry() -> HashMap<String, PathBuf> {
    match GlobalRegistry::open() {
        Ok(registry) => registry
            .list_enabled_sources()
            .unwrap_or_default()
            .into_iter()
            .map(|s| (s.name, s.path))
            .collect(),
        Err(_) => HashMap::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_registry_init() {
        let dir = tempdir().unwrap();
        let registry = GlobalRegistry::open_at(&dir.path().join("test.db")).unwrap();
        assert!(registry.list_sources().unwrap().is_empty());
    }

    #[test]
    fn test_add_and_get_source() {
        let dir = tempdir().unwrap();
        let registry = GlobalRegistry::open_at(&dir.path().join("test.db")).unwrap();

        let path = PathBuf::from("/home/user/project-a");
        registry
            .add_source("project-a", &path, SourceType::Yaml, true)
            .unwrap();

        let source = registry.get_source("project-a").unwrap().unwrap();
        assert_eq!(source.name, "project-a");
        assert_eq!(source.path, path);
        assert_eq!(source.source_type, "yaml");
        assert!(source.enabled);
    }

    #[test]
    fn test_get_source_by_path() {
        let dir = tempdir().unwrap();
        let registry = GlobalRegistry::open_at(&dir.path().join("test.db")).unwrap();

        let path = PathBuf::from("/home/user/project-b");
        registry
            .add_source("project-b", &path, SourceType::Yaml, true)
            .unwrap();

        let source = registry.get_source_by_path(&path).unwrap().unwrap();
        assert_eq!(source.name, "project-b");
    }

    #[test]
    fn test_remove_source() {
        let dir = tempdir().unwrap();
        let registry = GlobalRegistry::open_at(&dir.path().join("test.db")).unwrap();

        let path = PathBuf::from("/home/user/project-c");
        registry
            .add_source("project-c", &path, SourceType::Yaml, true)
            .unwrap();

        assert!(registry.remove_source("project-c").unwrap());
        assert!(registry.get_source("project-c").unwrap().is_none());
        assert!(!registry.remove_source("nonexistent").unwrap());
    }

    #[test]
    fn test_set_enabled() {
        let dir = tempdir().unwrap();
        let registry = GlobalRegistry::open_at(&dir.path().join("test.db")).unwrap();

        let path = PathBuf::from("/home/user/project-d");
        registry
            .add_source("project-d", &path, SourceType::Yaml, true)
            .unwrap();

        registry.set_enabled("project-d", false).unwrap();
        let source = registry.get_source("project-d").unwrap().unwrap();
        assert!(!source.enabled);
    }

    #[test]
    fn test_list_enabled_sources() {
        let dir = tempdir().unwrap();
        let registry = GlobalRegistry::open_at(&dir.path().join("test.db")).unwrap();

        registry
            .add_source(
                "enabled",
                &PathBuf::from("/a"),
                SourceType::Yaml,
                true,
            )
            .unwrap();
        registry
            .add_source(
                "disabled",
                &PathBuf::from("/b"),
                SourceType::Yaml,
                false,
            )
            .unwrap();

        let all = registry.list_sources().unwrap();
        assert_eq!(all.len(), 2);

        let enabled = registry.list_enabled_sources().unwrap();
        assert_eq!(enabled.len(), 1);
        assert_eq!(enabled[0].name, "enabled");
    }

    #[test]
    fn test_upsert_source() {
        let dir = tempdir().unwrap();
        let registry = GlobalRegistry::open_at(&dir.path().join("test.db")).unwrap();

        let path = PathBuf::from("/home/user/project-e");
        registry
            .add_source("project-e", &path, SourceType::Yaml, true)
            .unwrap();

        // Update same name with different path
        let new_path = PathBuf::from("/home/user/project-e-moved");
        registry
            .add_source("project-e", &new_path, SourceType::Sqlite, false)
            .unwrap();

        let source = registry.get_source("project-e").unwrap().unwrap();
        assert_eq!(source.path, new_path);
        assert_eq!(source.source_type, "sqlite");
        assert!(!source.enabled);

        // Only one entry
        assert_eq!(registry.list_sources().unwrap().len(), 1);
    }

    #[test]
    fn test_migrate_from_json() {
        let dir = tempdir().unwrap();

        // Write a mock sources.json
        let json_content = r#"[
            {"name": "proj-a", "path": "/home/user/proj-a", "enabled": true},
            {"name": "proj-b", "path": "/home/user/proj-b", "enabled": false}
        ]"#;
        let json_path = dir.path().join("sources.json");
        std::fs::write(&json_path, json_content).unwrap();

        let registry = GlobalRegistry::open_at(&dir.path().join("test.db")).unwrap();
        registry.migrate_from_json(&json_path).unwrap();

        let sources = registry.list_sources().unwrap();
        assert_eq!(sources.len(), 2);

        let enabled = registry.list_enabled_sources().unwrap();
        assert_eq!(enabled.len(), 1);
        assert_eq!(enabled[0].name, "proj-a");

        // Old file should be renamed
        assert!(!json_path.exists());
        assert!(dir.path().join("sources.json.bak").exists());
    }
}
