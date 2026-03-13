use crate::error::Result;
use crate::models::{
    ApprovalStatus, AuditAction, AuditEntry, ConflictPair, Edge, EdgeType, Node, NodeStats,
    NodeType, Subgraph,
};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::Mutex;
use uuid::Uuid;

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
            PRAGMA foreign_keys = ON;

            CREATE TABLE IF NOT EXISTS nodes (
                id TEXT PRIMARY KEY,
                node_type TEXT NOT NULL,
                title TEXT NOT NULL,
                content TEXT NOT NULL,
                source TEXT NOT NULL,
                approval_status TEXT NOT NULL DEFAULT 'pending',
                metadata TEXT NOT NULL DEFAULT '{}',
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS edges (
                id TEXT PRIMARY KEY,
                from_id TEXT NOT NULL,
                to_id TEXT NOT NULL,
                edge_type TEXT NOT NULL,
                weight REAL NOT NULL DEFAULT 1.0,
                metadata TEXT NOT NULL DEFAULT '{}',
                created_at TEXT NOT NULL,
                FOREIGN KEY (from_id) REFERENCES nodes(id) ON DELETE CASCADE,
                FOREIGN KEY (to_id) REFERENCES nodes(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS audit_log (
                id TEXT PRIMARY KEY,
                node_id TEXT NOT NULL,
                action TEXT NOT NULL,
                actor_source TEXT NOT NULL,
                actor_id TEXT,
                details TEXT,
                created_at TEXT NOT NULL,
                FOREIGN KEY (node_id) REFERENCES nodes(id) ON DELETE CASCADE
            );

            CREATE INDEX IF NOT EXISTS idx_nodes_type ON nodes(node_type);
            CREATE INDEX IF NOT EXISTS idx_nodes_status ON nodes(approval_status);
            CREATE INDEX IF NOT EXISTS idx_nodes_source ON nodes(source);
            CREATE INDEX IF NOT EXISTS idx_edges_from ON edges(from_id);
            CREATE INDEX IF NOT EXISTS idx_edges_to ON edges(to_id);
            CREATE INDEX IF NOT EXISTS idx_edges_type ON edges(edge_type);
            CREATE INDEX IF NOT EXISTS idx_audit_node ON audit_log(node_id);
            "#,
        )?;
        Ok(())
    }

    pub fn insert_node(&self, node: &Node) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            r#"INSERT OR REPLACE INTO nodes
               (id, node_type, title, content, source, approval_status, metadata, created_at, updated_at)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)"#,
            params![
                node.id.to_string(),
                node.node_type.as_str(),
                node.title,
                node.content,
                node.source,
                node.approval_status.as_str(),
                node.metadata.to_string(),
                node.created_at.to_rfc3339(),
                node.updated_at.to_rfc3339(),
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
                row_to_node,
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
            .query_map(params.as_slice(), row_to_node)?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(nodes)
    }

    pub fn update_node(
        &self,
        id: Uuid,
        title: Option<&str>,
        content: Option<&str>,
        node_type: Option<NodeType>,
        metadata: Option<&serde_json::Value>,
    ) -> Result<Option<Node>> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().to_rfc3339();

        let existing = conn
            .query_row(
                "SELECT * FROM nodes WHERE id = ?1",
                params![id.to_string()],
                row_to_node,
            )
            .optional()?;

        let Some(existing) = existing else {
            return Ok(None);
        };

        let final_title = title.unwrap_or(&existing.title);
        let final_content = content.unwrap_or(&existing.content);
        let final_node_type = node_type.unwrap_or(existing.node_type);
        let final_metadata = metadata.unwrap_or(&existing.metadata);

        conn.execute(
            r#"UPDATE nodes SET title = ?1, content = ?2, node_type = ?3, metadata = ?4, updated_at = ?5
               WHERE id = ?6"#,
            params![
                final_title,
                final_content,
                final_node_type.as_str(),
                final_metadata.to_string(),
                now,
                id.to_string(),
            ],
        )?;

        drop(conn);
        self.get_node(id)
    }

    pub fn delete_node(&self, id: Uuid) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        let affected = conn.execute("DELETE FROM nodes WHERE id = ?1", params![id.to_string()])?;
        Ok(affected > 0)
    }

    pub fn list_nodes(
        &self,
        node_type: Option<NodeType>,
        approval_status: Option<ApprovalStatus>,
        source: Option<&str>,
        limit: i32,
        offset: i32,
    ) -> Result<Vec<Node>> {
        let conn = self.conn.lock().unwrap();

        let mut sql = "SELECT * FROM nodes WHERE 1=1".to_string();
        let mut param_values: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if let Some(nt) = node_type {
            param_values.push(Box::new(nt.as_str().to_string()));
            sql.push_str(&format!(" AND node_type = ?{}", param_values.len()));
        }
        if let Some(status) = approval_status {
            param_values.push(Box::new(status.as_str().to_string()));
            sql.push_str(&format!(" AND approval_status = ?{}", param_values.len()));
        }
        if let Some(src) = source {
            param_values.push(Box::new(src.to_string()));
            sql.push_str(&format!(" AND source = ?{}", param_values.len()));
        }

        sql.push_str(" ORDER BY created_at DESC");

        param_values.push(Box::new(limit));
        sql.push_str(&format!(" LIMIT ?{}", param_values.len()));

        param_values.push(Box::new(offset));
        sql.push_str(&format!(" OFFSET ?{}", param_values.len()));

        let mut stmt = conn.prepare(&sql)?;
        let params: Vec<&dyn rusqlite::ToSql> =
            param_values.iter().map(|p| p.as_ref()).collect();
        let nodes = stmt
            .query_map(params.as_slice(), row_to_node)?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(nodes)
    }

    pub fn update_approval_status(&self, id: Uuid, status: ApprovalStatus) -> Result<Option<Node>> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().to_rfc3339();
        let affected = conn.execute(
            "UPDATE nodes SET approval_status = ?1, updated_at = ?2 WHERE id = ?3",
            params![status.as_str(), now, id.to_string()],
        )?;
        drop(conn);

        if affected == 0 {
            return Ok(None);
        }
        self.get_node(id)
    }

    pub fn insert_edge(&self, edge: &Edge) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            r#"INSERT OR REPLACE INTO edges
               (id, from_id, to_id, edge_type, weight, metadata, created_at)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)"#,
            params![
                edge.id.to_string(),
                edge.from_id.to_string(),
                edge.to_id.to_string(),
                edge.edge_type.as_str(),
                edge.weight,
                edge.metadata.to_string(),
                edge.created_at.to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    pub fn delete_edge(&self, id: Uuid) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        let affected = conn.execute("DELETE FROM edges WHERE id = ?1", params![id.to_string()])?;
        Ok(affected > 0)
    }

    pub fn get_edges(&self, node_id: Uuid) -> Result<Vec<Edge>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT * FROM edges WHERE from_id = ?1 OR to_id = ?1")?;
        let edges = stmt
            .query_map(params![node_id.to_string()], row_to_edge)?
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

    /// Traverse transitive dependencies from a node via specified edge types.
    pub fn get_impact(&self, node_id: Uuid, edge_types: &[EdgeType]) -> Result<Subgraph> {
        let default_types = [EdgeType::DerivedFrom, EdgeType::Requires];
        let types = if edge_types.is_empty() {
            &default_types[..]
        } else {
            edge_types
        };

        let mut subgraph = Subgraph::new();
        let mut visited: HashSet<Uuid> = HashSet::new();
        let mut frontier: Vec<Uuid> = vec![node_id];

        while !frontier.is_empty() {
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
                for edge in &edges {
                    if types.contains(&edge.edge_type) {
                        subgraph.add_edge(edge.clone());
                        // Follow edges where this node is the source
                        if edge.from_id == id && !visited.contains(&edge.to_id) {
                            next_frontier.push(edge.to_id);
                        }
                        if edge.to_id == id && !visited.contains(&edge.from_id) {
                            next_frontier.push(edge.from_id);
                        }
                    }
                }
            }
            frontier = next_frontier;
        }

        Ok(subgraph)
    }

    pub fn find_orphans(&self) -> Result<Vec<Node>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            r#"SELECT n.* FROM nodes n
               WHERE NOT EXISTS (SELECT 1 FROM edges e WHERE e.from_id = n.id OR e.to_id = n.id)"#,
        )?;
        let orphans = stmt
            .query_map([], row_to_node)?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(orphans)
    }

    pub fn find_conflicts(&self) -> Result<Vec<ConflictPair>> {
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
                conflicts.push(ConflictPair { node_a: a, node_b: b });
            }
        }
        Ok(conflicts)
    }

    pub fn get_stats(&self) -> Result<NodeStats> {
        let conn = self.conn.lock().unwrap();

        let total_nodes: i32 =
            conn.query_row("SELECT COUNT(*) FROM nodes", [], |row| row.get(0))?;
        let total_edges: i32 =
            conn.query_row("SELECT COUNT(*) FROM edges", [], |row| row.get(0))?;

        let mut by_type = HashMap::new();
        let mut stmt = conn.prepare("SELECT node_type, COUNT(*) FROM nodes GROUP BY node_type")?;
        let type_rows = stmt.query_map([], |row| {
            let t: String = row.get(0)?;
            let c: i32 = row.get(1)?;
            Ok((t, c))
        })?;
        for row in type_rows {
            let (t, c) = row?;
            by_type.insert(t, c);
        }

        let mut by_status = HashMap::new();
        let mut stmt =
            conn.prepare("SELECT approval_status, COUNT(*) FROM nodes GROUP BY approval_status")?;
        let status_rows = stmt.query_map([], |row| {
            let s: String = row.get(0)?;
            let c: i32 = row.get(1)?;
            Ok((s, c))
        })?;
        for row in status_rows {
            let (s, c) = row?;
            by_status.insert(s, c);
        }

        let orphan_count: i32 = conn.query_row(
            "SELECT COUNT(*) FROM nodes n WHERE NOT EXISTS (SELECT 1 FROM edges e WHERE e.from_id = n.id OR e.to_id = n.id)",
            [],
            |row| row.get(0),
        )?;

        let conflict_count: i32 = conn.query_row(
            "SELECT COUNT(*) FROM edges WHERE edge_type = 'contradicts'",
            [],
            |row| row.get(0),
        )?;

        Ok(NodeStats {
            total_nodes,
            total_edges,
            by_type,
            by_status,
            orphan_count,
            conflict_count,
        })
    }

    // ── Audit ──

    pub fn insert_audit(&self, entry: &AuditEntry) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            r#"INSERT INTO audit_log (id, node_id, action, actor_source, actor_id, details, created_at)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)"#,
            params![
                entry.id.to_string(),
                entry.node_id.to_string(),
                entry.action.as_str(),
                entry.actor_source,
                entry.actor_id,
                entry.details.as_ref().map(|d| d.to_string()),
                entry.created_at.to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    pub fn get_audit_log(&self, node_id: Uuid, limit: i32) -> Result<Vec<AuditEntry>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT * FROM audit_log WHERE node_id = ?1 ORDER BY created_at DESC LIMIT ?2",
        )?;
        let entries = stmt
            .query_map(params![node_id.to_string(), limit], row_to_audit)?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(entries)
    }
}

fn row_to_node(row: &rusqlite::Row) -> rusqlite::Result<Node> {
    let id_str: String = row.get("id")?;
    let node_type_str: String = row.get("node_type")?;
    let approval_str: String = row.get("approval_status")?;
    let created_at_str: String = row.get("created_at")?;
    let updated_at_str: String = row.get("updated_at")?;
    let metadata_str: String = row.get("metadata")?;

    Ok(Node {
        id: Uuid::parse_str(&id_str).unwrap(),
        node_type: NodeType::from_str(&node_type_str),
        title: row.get("title")?,
        content: row.get("content")?,
        source: row.get("source")?,
        approval_status: ApprovalStatus::from_str(&approval_str),
        metadata: serde_json::from_str(&metadata_str).unwrap_or_default(),
        created_at: DateTime::parse_from_rfc3339(&created_at_str)
            .unwrap()
            .with_timezone(&Utc),
        updated_at: DateTime::parse_from_rfc3339(&updated_at_str)
            .unwrap()
            .with_timezone(&Utc),
    })
}

fn row_to_edge(row: &rusqlite::Row) -> rusqlite::Result<Edge> {
    let id_str: String = row.get("id")?;
    let from_id_str: String = row.get("from_id")?;
    let to_id_str: String = row.get("to_id")?;
    let edge_type_str: String = row.get("edge_type")?;
    let created_at_str: String = row.get("created_at")?;
    let metadata_str: String = row.get("metadata")?;

    Ok(Edge {
        id: Uuid::parse_str(&id_str).unwrap(),
        from_id: Uuid::parse_str(&from_id_str).unwrap(),
        to_id: Uuid::parse_str(&to_id_str).unwrap(),
        edge_type: EdgeType::from_str(&edge_type_str),
        weight: row.get("weight")?,
        metadata: serde_json::from_str(&metadata_str).unwrap_or_default(),
        created_at: DateTime::parse_from_rfc3339(&created_at_str)
            .unwrap()
            .with_timezone(&Utc),
    })
}

fn row_to_audit(row: &rusqlite::Row) -> rusqlite::Result<AuditEntry> {
    let id_str: String = row.get("id")?;
    let node_id_str: String = row.get("node_id")?;
    let action_str: String = row.get("action")?;
    let created_at_str: String = row.get("created_at")?;
    let details_str: Option<String> = row.get("details")?;

    Ok(AuditEntry {
        id: Uuid::parse_str(&id_str).unwrap(),
        node_id: Uuid::parse_str(&node_id_str).unwrap(),
        action: match action_str.as_str() {
            "create" => AuditAction::Create,
            "update" => AuditAction::Update,
            "delete" => AuditAction::Delete,
            "approve" => AuditAction::Approve,
            "reject" => AuditAction::Reject,
            _ => AuditAction::Create,
        },
        actor_source: row.get("actor_source")?,
        actor_id: row.get("actor_id")?,
        details: details_str.and_then(|s| serde_json::from_str(&s).ok()),
        created_at: DateTime::parse_from_rfc3339(&created_at_str)
            .unwrap()
            .with_timezone(&Utc),
    })
}
