use crate::error::Result;
use crate::types::{Confidence, Edge, EdgeType, KnowledgeSource, Node, NodeType, Subgraph};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use std::collections::HashSet;
use std::path::Path;
use std::sync::Mutex;
use uuid::Uuid;

/// Graph database storage using SQLite
pub struct GraphStorage {
    conn: Mutex<Connection>,
}

impl GraphStorage {
    pub fn open(path: &Path) -> Result<Self> {
        let conn = Connection::open(path)?;
        let storage = Self {
            conn: Mutex::new(conn),
        };
        storage.init_schema()?;
        Ok(storage)
    }

    fn init_schema(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS nodes (
                id TEXT PRIMARY KEY,
                node_type TEXT NOT NULL,
                title TEXT NOT NULL,
                content TEXT NOT NULL,
                source_type TEXT NOT NULL,
                source_data TEXT NOT NULL,
                confidence REAL NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                last_accessed_at TEXT NOT NULL,
                metadata TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS edges (
                id TEXT PRIMARY KEY,
                from_id TEXT NOT NULL,
                to_id TEXT NOT NULL,
                edge_type TEXT NOT NULL,
                weight REAL NOT NULL,
                created_at TEXT NOT NULL,
                metadata TEXT NOT NULL,
                FOREIGN KEY (from_id) REFERENCES nodes(id) ON DELETE CASCADE,
                FOREIGN KEY (to_id) REFERENCES nodes(id) ON DELETE CASCADE
            );

            CREATE INDEX IF NOT EXISTS idx_edges_from ON edges(from_id);
            CREATE INDEX IF NOT EXISTS idx_edges_to ON edges(to_id);
            CREATE INDEX IF NOT EXISTS idx_edges_type ON edges(edge_type);
            CREATE INDEX IF NOT EXISTS idx_nodes_type ON nodes(node_type);
            "#,
        )?;
        Ok(())
    }

    pub fn insert_node(&self, node: &Node) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let (source_type, source_data) = match &node.source {
            KnowledgeSource::User { statement } => {
                ("user", serde_json::json!({ "statement": statement }))
            }
            KnowledgeSource::Derived {
                interpretation,
                source_id,
            } => (
                "derived",
                serde_json::json!({ "interpretation": interpretation, "source_id": source_id }),
            ),
        };

        conn.execute(
            r#"INSERT OR REPLACE INTO nodes
               (id, node_type, title, content, source_type, source_data, confidence,
                created_at, updated_at, last_accessed_at, metadata)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)"#,
            params![
                node.id.to_string(),
                node.node_type.as_str(),
                node.title,
                node.content,
                source_type,
                source_data.to_string(),
                node.confidence.0,
                node.created_at.to_rfc3339(),
                node.updated_at.to_rfc3339(),
                node.last_accessed_at.to_rfc3339(),
                node.metadata.to_string(),
            ],
        )?;
        Ok(())
    }

    pub fn get_node(&self, id: Uuid) -> Result<Option<Node>> {
        let conn = self.conn.lock().unwrap();
        let result = conn
            .query_row(
                "SELECT * FROM nodes WHERE id = ?1",
                params![id.to_string()],
                Self::row_to_node,
            )
            .optional()?;
        Ok(result)
    }

    pub fn get_nodes(&self, ids: &[Uuid]) -> Result<Vec<Node>> {
        if ids.is_empty() {
            return Ok(vec![]);
        }
        let conn = self.conn.lock().unwrap();
        let placeholders: Vec<_> = ids.iter().map(|_| "?").collect();
        let sql = format!(
            "SELECT * FROM nodes WHERE id IN ({})",
            placeholders.join(", ")
        );
        let mut stmt = conn.prepare(&sql)?;
        let id_strings: Vec<_> = ids.iter().map(|id| id.to_string()).collect();
        let params: Vec<&dyn rusqlite::ToSql> = id_strings
            .iter()
            .map(|s| s as &dyn rusqlite::ToSql)
            .collect();
        let nodes = stmt
            .query_map(params.as_slice(), Self::row_to_node)?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(nodes)
    }

    pub fn delete_node(&self, id: Uuid) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "DELETE FROM edges WHERE from_id = ?1 OR to_id = ?1",
            params![id.to_string()],
        )?;
        conn.execute("DELETE FROM nodes WHERE id = ?1", params![id.to_string()])?;
        Ok(())
    }

    pub fn insert_edge(&self, edge: &Edge) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            r#"INSERT OR REPLACE INTO edges
               (id, from_id, to_id, edge_type, weight, created_at, metadata)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)"#,
            params![
                edge.id.to_string(),
                edge.from_id.to_string(),
                edge.to_id.to_string(),
                edge.edge_type.as_str(),
                edge.weight,
                edge.created_at.to_rfc3339(),
                edge.metadata.to_string(),
            ],
        )?;
        Ok(())
    }

    pub fn get_edges(&self, node_id: Uuid) -> Result<Vec<Edge>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT * FROM edges WHERE from_id = ?1 OR to_id = ?1")?;
        let edges = stmt
            .query_map(params![node_id.to_string()], Self::row_to_edge)?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(edges)
    }

    pub fn get_neighbors(&self, node_id: Uuid, hops: usize) -> Result<Subgraph> {
        let mut subgraph = Subgraph::new();
        let mut visited: HashSet<Uuid> = HashSet::new();
        let mut frontier: Vec<Uuid> = vec![node_id];

        for _ in 0..=hops {
            let mut next_frontier = Vec::new();
            for id in frontier {
                if visited.contains(&id) {
                    continue;
                }
                visited.insert(id);

                if let Some(node) = self.get_node(id)? {
                    subgraph.add_node(node);
                }

                let edges = self.get_edges(id)?;
                for edge in edges {
                    subgraph.add_edge(edge.clone());
                    if !visited.contains(&edge.from_id) {
                        next_frontier.push(edge.from_id);
                    }
                    if !visited.contains(&edge.to_id) {
                        next_frontier.push(edge.to_id);
                    }
                }
            }
            frontier = next_frontier;
        }

        Ok(subgraph)
    }

    pub fn touch_node(&self, id: Uuid) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE nodes SET last_accessed_at = ?1 WHERE id = ?2",
            params![now, id.to_string()],
        )?;
        Ok(())
    }

    pub fn find_orphans(&self) -> Result<Vec<Uuid>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            r#"SELECT n.id FROM nodes n
               WHERE NOT EXISTS (SELECT 1 FROM edges e WHERE e.from_id = n.id OR e.to_id = n.id)"#,
        )?;
        let orphans = stmt
            .query_map([], |row| {
                let id_str: String = row.get(0)?;
                Ok(Uuid::parse_str(&id_str).unwrap())
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(orphans)
    }

    pub fn find_conflicts(&self) -> Result<Vec<(Node, Node)>> {
        let pairs: Vec<(String, String)> = {
            let conn = self.conn.lock().unwrap();
            let mut stmt = conn.prepare(
                r#"SELECT e.from_id, e.to_id FROM edges e WHERE e.edge_type = 'contradicts'"#,
            )?;
            let result = stmt
                .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
                .collect::<std::result::Result<Vec<_>, _>>()?;
            result
        };

        let mut conflicts = Vec::new();
        for (from_str, to_str) in pairs {
            let from_id = Uuid::parse_str(&from_str).unwrap();
            let to_id = Uuid::parse_str(&to_str).unwrap();
            if let (Some(a), Some(b)) = (self.get_node(from_id)?, self.get_node(to_id)?) {
                conflicts.push((a, b));
            }
        }
        Ok(conflicts)
    }

    pub fn update_confidence(&self, id: Uuid, confidence: Confidence) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE nodes SET confidence = ?1, updated_at = ?2 WHERE id = ?3",
            params![confidence.0, now, id.to_string()],
        )?;
        Ok(())
    }

    fn row_to_node(row: &rusqlite::Row) -> rusqlite::Result<Node> {
        let id_str: String = row.get("id")?;
        let node_type_str: String = row.get("node_type")?;
        let source_type: String = row.get("source_type")?;
        let source_data_str: String = row.get("source_data")?;
        let created_at_str: String = row.get("created_at")?;
        let updated_at_str: String = row.get("updated_at")?;
        let last_accessed_at_str: String = row.get("last_accessed_at")?;
        let metadata_str: String = row.get("metadata")?;

        let node_type = match node_type_str.as_str() {
            "decision" => NodeType::Decision,
            "fact" => NodeType::Fact,
            "error" => NodeType::Error,
            "guide" => NodeType::Guide,
            "glossary" => NodeType::Glossary,
            "context" => NodeType::Context,
            "assumption" => NodeType::Assumption,
            _ => NodeType::Fact,
        };

        let source_data: serde_json::Value =
            serde_json::from_str(&source_data_str).unwrap_or_default();
        let source = match source_type.as_str() {
            "user" => KnowledgeSource::User {
                statement: source_data["statement"].as_str().unwrap_or("").to_string(),
            },
            _ => KnowledgeSource::Derived {
                interpretation: source_data["interpretation"]
                    .as_str()
                    .unwrap_or("")
                    .to_string(),
                source_id: source_data["source_id"]
                    .as_str()
                    .and_then(|s| Uuid::parse_str(s).ok()),
            },
        };

        Ok(Node {
            id: Uuid::parse_str(&id_str).unwrap(),
            node_type,
            title: row.get("title")?,
            content: row.get("content")?,
            source,
            confidence: Confidence::new(row.get("confidence")?),
            created_at: DateTime::parse_from_rfc3339(&created_at_str)
                .unwrap()
                .with_timezone(&Utc),
            updated_at: DateTime::parse_from_rfc3339(&updated_at_str)
                .unwrap()
                .with_timezone(&Utc),
            last_accessed_at: DateTime::parse_from_rfc3339(&last_accessed_at_str)
                .unwrap()
                .with_timezone(&Utc),
            metadata: serde_json::from_str(&metadata_str).unwrap_or_default(),
        })
    }

    fn row_to_edge(row: &rusqlite::Row) -> rusqlite::Result<Edge> {
        let id_str: String = row.get("id")?;
        let from_id_str: String = row.get("from_id")?;
        let to_id_str: String = row.get("to_id")?;
        let edge_type_str: String = row.get("edge_type")?;
        let created_at_str: String = row.get("created_at")?;
        let metadata_str: String = row.get("metadata")?;

        let edge_type = match edge_type_str.as_str() {
            "supersedes" => EdgeType::Supersedes,
            "contradicts" => EdgeType::Contradicts,
            "requires" => EdgeType::Requires,
            "related_to" => EdgeType::RelatedTo,
            "derived_from" => EdgeType::DerivedFrom,
            "answers" => EdgeType::Answers,
            _ => EdgeType::RelatedTo,
        };

        Ok(Edge {
            id: Uuid::parse_str(&id_str).unwrap(),
            from_id: Uuid::parse_str(&from_id_str).unwrap(),
            to_id: Uuid::parse_str(&to_id_str).unwrap(),
            edge_type,
            weight: row.get("weight")?,
            created_at: DateTime::parse_from_rfc3339(&created_at_str)
                .unwrap()
                .with_timezone(&Utc),
            metadata: serde_json::from_str(&metadata_str).unwrap_or_default(),
        })
    }
}
