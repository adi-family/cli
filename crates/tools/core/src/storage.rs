use crate::{Error, MatchType, Result, SearchResult, Tool, ToolSource, ToolUsage};
use rusqlite::{params, Connection};
use std::path::Path;
use std::sync::Mutex;

pub struct Storage {
    conn: Mutex<Connection>,
}

impl Storage {
    pub fn open(path: &Path) -> Result<Self> {
        let conn = Connection::open(path)?;
        let storage = Self {
            conn: Mutex::new(conn),
        };
        storage.migrate()?;
        Ok(storage)
    }

    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        let storage = Self {
            conn: Mutex::new(conn),
        };
        storage.migrate()?;
        Ok(storage)
    }

    fn migrate(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS tools (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT NOT NULL,
                source_type TEXT NOT NULL,
                source_data TEXT,
                updated_at INTEGER NOT NULL
            );

            CREATE VIRTUAL TABLE IF NOT EXISTS tools_fts USING fts5(
                name, description,
                content='tools',
                content_rowid='rowid'
            );

            CREATE TABLE IF NOT EXISTS tool_usage (
                tool_id TEXT PRIMARY KEY REFERENCES tools(id),
                help_text TEXT NOT NULL,
                examples TEXT,
                flags TEXT
            );

            CREATE TRIGGER IF NOT EXISTS tools_ai AFTER INSERT ON tools BEGIN
                INSERT INTO tools_fts(rowid, name, description)
                VALUES (new.rowid, new.name, new.description);
            END;

            CREATE TRIGGER IF NOT EXISTS tools_ad AFTER DELETE ON tools BEGIN
                INSERT INTO tools_fts(tools_fts, rowid, name, description)
                VALUES ('delete', old.rowid, old.name, old.description);
            END;

            CREATE TRIGGER IF NOT EXISTS tools_au AFTER UPDATE ON tools BEGIN
                INSERT INTO tools_fts(tools_fts, rowid, name, description)
                VALUES ('delete', old.rowid, old.name, old.description);
                INSERT INTO tools_fts(rowid, name, description)
                VALUES (new.rowid, new.name, new.description);
            END;
        "#,
        )?;
        Ok(())
    }

    pub fn upsert_tool(&self, tool: &Tool) -> Result<()> {
        let source_type = match &tool.source {
            ToolSource::Plugin { .. } => "plugin",
            ToolSource::ToolDir { .. } => "tooldir",
            ToolSource::System { .. } => "system",
        };
        let source_data = serde_json::to_string(&tool.source)?;

        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO tools (id, name, description, source_type, source_data, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                tool.id,
                tool.name,
                tool.description,
                source_type,
                source_data,
                tool.updated_at
            ],
        )?;
        Ok(())
    }

    pub fn get_tool(&self, id: &str) -> Result<Option<Tool>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, name, description, source_data, updated_at FROM tools WHERE id = ?1",
        )?;

        let mut rows = stmt.query(params![id])?;
        if let Some(row) = rows.next()? {
            let source_str: String = row.get(3)?;
            let source: ToolSource = serde_json::from_str(&source_str)?;

            Ok(Some(Tool {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                source,
                updated_at: row.get(4)?,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn list_tools(&self) -> Result<Vec<Tool>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, name, description, source_data, updated_at FROM tools ORDER BY name",
        )?;

        let rows = stmt.query_map([], |row| {
            let source_str: String = row.get(3)?;
            let source: ToolSource =
                serde_json::from_str(&source_str).unwrap_or(ToolSource::System {
                    path: std::path::PathBuf::new(),
                });

            Ok(Tool {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                source,
                updated_at: row.get(4)?,
            })
        })?;

        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Error::from)
    }

    pub fn search_fts(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>> {
        // Escape special FTS5 characters and prepare query
        let escaped_query = escape_fts_query(query);

        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT t.id, t.name, t.description, t.source_data, t.updated_at,
                    bm25(tools_fts) as score
             FROM tools_fts f
             JOIN tools t ON t.rowid = f.rowid
             WHERE tools_fts MATCH ?1
             ORDER BY score
             LIMIT ?2",
        )?;

        let rows = stmt.query_map(params![escaped_query, limit as i64], |row| {
            let source_str: String = row.get(3)?;
            let source: ToolSource =
                serde_json::from_str(&source_str).unwrap_or(ToolSource::System {
                    path: std::path::PathBuf::new(),
                });

            Ok(SearchResult {
                tool: Tool {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    description: row.get(2)?,
                    source,
                    updated_at: row.get(4)?,
                },
                score: -row.get::<_, f64>(5)? as f32, // BM25 returns negative
                match_type: MatchType::Keyword,
            })
        })?;

        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Error::from)
    }

    pub fn upsert_usage(&self, usage: &ToolUsage) -> Result<()> {
        let examples = serde_json::to_string(&usage.examples)?;
        let flags = serde_json::to_string(&usage.flags)?;

        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO tool_usage (tool_id, help_text, examples, flags)
             VALUES (?1, ?2, ?3, ?4)",
            params![usage.tool_id, usage.help_text, examples, flags],
        )?;
        Ok(())
    }

    pub fn get_usage(&self, tool_id: &str) -> Result<Option<ToolUsage>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT tool_id, help_text, examples, flags FROM tool_usage WHERE tool_id = ?1",
        )?;

        let mut rows = stmt.query(params![tool_id])?;
        if let Some(row) = rows.next()? {
            let examples_str: String = row.get(2)?;
            let flags_str: String = row.get(3)?;

            let examples: Vec<String> = serde_json::from_str(&examples_str).unwrap_or_default();
            let flags = serde_json::from_str(&flags_str).unwrap_or_default();

            Ok(Some(ToolUsage {
                tool_id: row.get(0)?,
                help_text: row.get(1)?,
                examples,
                flags,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn delete_tool(&self, id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM tool_usage WHERE tool_id = ?1", params![id])?;
        conn.execute("DELETE FROM tools WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn clear(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM tool_usage", [])?;
        conn.execute("DELETE FROM tools", [])?;
        Ok(())
    }

    pub fn count(&self) -> Result<usize> {
        let conn = self.conn.lock().unwrap();
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM tools", [], |row| row.get(0))?;
        Ok(count as usize)
    }
}

/// Escape special FTS5 characters in query
fn escape_fts_query(query: &str) -> String {
    // For simple queries, wrap each word in quotes to treat as literal
    query
        .split_whitespace()
        .map(|word| {
            // Remove special chars that could break FTS5
            let clean: String = word
                .chars()
                .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
                .collect();
            if clean.is_empty() {
                String::new()
            } else {
                format!("\"{}\"", clean)
            }
        })
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(" OR ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_storage_basic() {
        let storage = Storage::open_in_memory().unwrap();

        let tool = Tool {
            id: "test-tool".to_string(),
            name: "test".to_string(),
            description: "A test tool".to_string(),
            source: ToolSource::System {
                path: "/usr/bin/test".into(),
            },
            updated_at: 1234567890,
        };

        storage.upsert_tool(&tool).unwrap();

        let retrieved = storage.get_tool("test-tool").unwrap().unwrap();
        assert_eq!(retrieved.name, "test");
        assert_eq!(retrieved.description, "A test tool");
    }

    #[test]
    fn test_fts_search() {
        let storage = Storage::open_in_memory().unwrap();

        let tools = vec![
            Tool {
                id: "docker-ps".to_string(),
                name: "docker ps".to_string(),
                description: "List running containers".to_string(),
                source: ToolSource::System {
                    path: "/usr/bin/docker".into(),
                },
                updated_at: 1234567890,
            },
            Tool {
                id: "git-status".to_string(),
                name: "git status".to_string(),
                description: "Show working tree status".to_string(),
                source: ToolSource::System {
                    path: "/usr/bin/git".into(),
                },
                updated_at: 1234567890,
            },
        ];

        for tool in &tools {
            storage.upsert_tool(tool).unwrap();
        }

        let results = storage.search_fts("containers", 10).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].tool.id, "docker-ps");
    }
}
