use crate::error::Result;
use crate::models::{
    ApprovalStatus, AuditAction, AuditEntry, ConflictPair, Edge, EdgeType, Node, NodeStats,
    NodeType, Subgraph, TagInfo,
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

            CREATE TABLE IF NOT EXISTS node_tags (
                node_id TEXT NOT NULL,
                tag TEXT NOT NULL,
                PRIMARY KEY (node_id, tag),
                FOREIGN KEY (node_id) REFERENCES nodes(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS audit_log (
                id TEXT PRIMARY KEY,
                node_id TEXT NOT NULL,
                action TEXT NOT NULL,
                actor_source TEXT NOT NULL,
                actor_id TEXT,
                details TEXT,
                created_at TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_nodes_type ON nodes(node_type);
            CREATE INDEX IF NOT EXISTS idx_nodes_status ON nodes(approval_status);
            CREATE INDEX IF NOT EXISTS idx_nodes_source ON nodes(source);
            CREATE INDEX IF NOT EXISTS idx_node_tags_tag ON node_tags(tag);
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
        tags: Option<&[String]>,
        limit: i32,
        offset: i32,
    ) -> Result<Vec<Node>> {
        let conn = self.conn.lock().unwrap();

        let mut sql = "SELECT DISTINCT n.* FROM nodes n".to_string();
        let mut param_values: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        let has_tags = tags.is_some_and(|t| !t.is_empty());
        if has_tags {
            let tag_list = tags.unwrap();
            let placeholders: Vec<_> = tag_list
                .iter()
                .enumerate()
                .map(|(i, _)| format!("?{}", param_values.len() + i + 1))
                .collect();
            sql.push_str(&format!(
                " INNER JOIN node_tags nt ON nt.node_id = n.id AND nt.tag IN ({})",
                placeholders.join(", ")
            ));
            for tag in tag_list {
                param_values.push(Box::new(tag.clone()));
            }
        }

        sql.push_str(" WHERE 1=1");

        if let Some(nt) = node_type {
            param_values.push(Box::new(nt.as_str().to_string()));
            sql.push_str(&format!(" AND n.node_type = ?{}", param_values.len()));
        }
        if let Some(status) = approval_status {
            param_values.push(Box::new(status.as_str().to_string()));
            sql.push_str(&format!(" AND n.approval_status = ?{}", param_values.len()));
        }
        if let Some(src) = source {
            param_values.push(Box::new(src.to_string()));
            sql.push_str(&format!(" AND n.source = ?{}", param_values.len()));
        }

        sql.push_str(" ORDER BY n.created_at DESC");

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
                    if types.contains(&edge.edge_type) && edge.from_id == id {
                        subgraph.add_edge(edge.clone());
                        if !visited.contains(&edge.to_id) {
                            next_frontier.push(edge.to_id);
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

    // ── Tags ──

    pub fn set_tags(&self, node_id: Uuid, tags: &[String]) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let id_str = node_id.to_string();
        conn.execute("DELETE FROM node_tags WHERE node_id = ?1", params![id_str])?;
        for tag in tags {
            conn.execute(
                "INSERT OR IGNORE INTO node_tags (node_id, tag) VALUES (?1, ?2)",
                params![id_str, tag],
            )?;
        }
        Ok(())
    }

    pub fn get_tags(&self, node_id: Uuid) -> Result<Vec<String>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT tag FROM node_tags WHERE node_id = ?1 ORDER BY tag")?;
        let tags = stmt
            .query_map(params![node_id.to_string()], |row| row.get(0))?
            .collect::<std::result::Result<Vec<String>, _>>()?;
        Ok(tags)
    }

    pub fn get_tags_for_nodes(&self, node_ids: &[Uuid]) -> Result<HashMap<Uuid, Vec<String>>> {
        if node_ids.is_empty() {
            return Ok(HashMap::new());
        }
        let conn = self.conn.lock().unwrap();
        let placeholders: Vec<_> = node_ids.iter().map(|_| "?").collect();
        let sql = format!(
            "SELECT node_id, tag FROM node_tags WHERE node_id IN ({}) ORDER BY tag",
            placeholders.join(", ")
        );
        let mut stmt = conn.prepare(&sql)?;
        let id_strings: Vec<_> = node_ids.iter().map(|id| id.to_string()).collect();
        let params: Vec<&dyn rusqlite::ToSql> = id_strings
            .iter()
            .map(|s| s as &dyn rusqlite::ToSql)
            .collect();

        let mut result: HashMap<Uuid, Vec<String>> = HashMap::new();
        let rows = stmt.query_map(params.as_slice(), |row| {
            let node_id_str: String = row.get(0)?;
            let tag: String = row.get(1)?;
            Ok((node_id_str, tag))
        })?;
        for row in rows {
            let (node_id_str, tag) = row?;
            let node_id = Uuid::parse_str(&node_id_str).unwrap();
            result.entry(node_id).or_default().push(tag);
        }
        Ok(result)
    }

    pub fn list_tags(&self, limit: i32) -> Result<Vec<TagInfo>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT tag, COUNT(*) as cnt FROM node_tags GROUP BY tag ORDER BY cnt DESC, tag ASC LIMIT ?1",
        )?;
        let tags = stmt
            .query_map(params![limit], |row| {
                Ok(TagInfo {
                    tag: row.get(0)?,
                    count: row.get(1)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(tags)
    }

    /// Populate tags on a single node from the node_tags table.
    pub fn fill_tags(&self, node: &mut Node) -> Result<()> {
        node.tags = self.get_tags(node.id)?;
        Ok(())
    }

    /// Populate tags on a list of nodes in a single batch query.
    pub fn fill_tags_batch(&self, nodes: &mut [Node]) -> Result<()> {
        let ids: Vec<Uuid> = nodes.iter().map(|n| n.id).collect();
        let mut tags_map = self.get_tags_for_nodes(&ids)?;
        for node in nodes.iter_mut() {
            if let Some(tags) = tags_map.remove(&node.id) {
                node.tags = tags;
            }
        }
        Ok(())
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
        tags: Vec::new(),
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

    let action = match action_str.as_str() {
        "create" => AuditAction::Create,
        "update" => AuditAction::Update,
        "delete" => AuditAction::Delete,
        "approve" => AuditAction::Approve,
        "reject" => AuditAction::Reject,
        other => {
            return Err(rusqlite::Error::FromSqlConversionFailure(
                2,
                rusqlite::types::Type::Text,
                format!("unknown audit action: {other}").into(),
            ));
        }
    };

    Ok(AuditEntry {
        id: Uuid::parse_str(&id_str).unwrap(),
        node_id: Uuid::parse_str(&node_id_str).unwrap(),
        action,
        actor_source: row.get("actor_source")?,
        actor_id: row.get("actor_id")?,
        details: details_str.and_then(|s| serde_json::from_str(&s).ok()),
        created_at: DateTime::parse_from_rfc3339(&created_at_str)
            .unwrap()
            .with_timezone(&Utc),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn test_storage() -> (GraphStorage, TempDir) {
        let dir = TempDir::new().unwrap();
        let storage = GraphStorage::open(&dir.path().join("test.db")).unwrap();
        (storage, dir)
    }

    fn make_node(node_type: NodeType, source: &str) -> Node {
        let now = Utc::now();
        Node {
            id: Uuid::new_v4(),
            node_type,
            title: format!("Test {}", node_type.as_str()),
            content: "Test content".into(),
            source: source.into(),
            approval_status: ApprovalStatus::Pending,
            metadata: serde_json::json!({}),
            tags: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }

    fn make_edge(from: Uuid, to: Uuid, edge_type: EdgeType) -> Edge {
        Edge {
            id: Uuid::new_v4(),
            from_id: from,
            to_id: to,
            edge_type,
            weight: 1.0,
            metadata: serde_json::json!({}),
            created_at: Utc::now(),
        }
    }

    #[test]
    fn insert_and_get_node() {
        let (storage, _dir) = test_storage();
        let node = make_node(NodeType::Decision, "human");
        storage.insert_node(&node).unwrap();

        let fetched = storage.get_node(node.id).unwrap().unwrap();
        assert_eq!(fetched.id, node.id);
        assert_eq!(fetched.title, node.title);
        assert_eq!(fetched.source, "human");
        assert_eq!(fetched.node_type, NodeType::Decision);
        assert_eq!(fetched.approval_status, ApprovalStatus::Pending);
    }

    #[test]
    fn get_nonexistent_node_returns_none() {
        let (storage, _dir) = test_storage();
        assert!(storage.get_node(Uuid::new_v4()).unwrap().is_none());
    }

    #[test]
    fn update_node_partial() {
        let (storage, _dir) = test_storage();
        let node = make_node(NodeType::Fact, "ai:gpt-4");
        storage.insert_node(&node).unwrap();

        let updated = storage
            .update_node(node.id, Some("New title"), None, None, None)
            .unwrap()
            .unwrap();
        assert_eq!(updated.title, "New title");
        assert_eq!(updated.content, node.content);
    }

    #[test]
    fn update_nonexistent_returns_none() {
        let (storage, _dir) = test_storage();
        let result = storage
            .update_node(Uuid::new_v4(), Some("x"), None, None, None)
            .unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn delete_node() {
        let (storage, _dir) = test_storage();
        let node = make_node(NodeType::Fact, "human");
        storage.insert_node(&node).unwrap();

        assert!(storage.delete_node(node.id).unwrap());
        assert!(storage.get_node(node.id).unwrap().is_none());
    }

    #[test]
    fn delete_nonexistent_returns_false() {
        let (storage, _dir) = test_storage();
        assert!(!storage.delete_node(Uuid::new_v4()).unwrap());
    }

    #[test]
    fn list_nodes_with_filters() {
        let (storage, _dir) = test_storage();
        let n1 = make_node(NodeType::Decision, "human");
        let n2 = make_node(NodeType::Fact, "ai:gpt-4");
        let n3 = make_node(NodeType::Decision, "ai:gpt-4");
        storage.insert_node(&n1).unwrap();
        storage.insert_node(&n2).unwrap();
        storage.insert_node(&n3).unwrap();

        let all = storage.list_nodes(None, None, None, None, 50, 0).unwrap();
        assert_eq!(all.len(), 3);

        let decisions = storage
            .list_nodes(Some(NodeType::Decision), None, None, None, 50, 0)
            .unwrap();
        assert_eq!(decisions.len(), 2);

        let ai_nodes = storage
            .list_nodes(None, None, Some("ai:gpt-4"), None, 50, 0)
            .unwrap();
        assert_eq!(ai_nodes.len(), 2);

        let limited = storage.list_nodes(None, None, None, None, 1, 0).unwrap();
        assert_eq!(limited.len(), 1);
    }

    #[test]
    fn approval_workflow() {
        let (storage, _dir) = test_storage();
        let node = make_node(NodeType::Assumption, "ai");
        storage.insert_node(&node).unwrap();
        assert_eq!(
            storage.get_node(node.id).unwrap().unwrap().approval_status,
            ApprovalStatus::Pending
        );

        let approved = storage
            .update_approval_status(node.id, ApprovalStatus::Approved)
            .unwrap()
            .unwrap();
        assert_eq!(approved.approval_status, ApprovalStatus::Approved);

        let pending = storage
            .list_nodes(None, Some(ApprovalStatus::Pending), None, None, 50, 0)
            .unwrap();
        assert!(pending.is_empty());
    }

    #[test]
    fn edge_crud() {
        let (storage, _dir) = test_storage();
        let n1 = make_node(NodeType::Decision, "human");
        let n2 = make_node(NodeType::Decision, "human");
        storage.insert_node(&n1).unwrap();
        storage.insert_node(&n2).unwrap();

        let edge = make_edge(n1.id, n2.id, EdgeType::DerivedFrom);
        storage.insert_edge(&edge).unwrap();

        let edges = storage.get_edges(n1.id).unwrap();
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].edge_type, EdgeType::DerivedFrom);

        let edges_from_n2 = storage.get_edges(n2.id).unwrap();
        assert_eq!(edges_from_n2.len(), 1);

        assert!(storage.delete_edge(edge.id).unwrap());
        assert!(storage.get_edges(n1.id).unwrap().is_empty());
    }

    #[test]
    fn cascade_delete_removes_edges() {
        let (storage, _dir) = test_storage();
        let n1 = make_node(NodeType::Fact, "human");
        let n2 = make_node(NodeType::Fact, "human");
        storage.insert_node(&n1).unwrap();
        storage.insert_node(&n2).unwrap();
        storage
            .insert_edge(&make_edge(n1.id, n2.id, EdgeType::Requires))
            .unwrap();

        storage.delete_node(n1.id).unwrap();
        assert!(storage.get_edges(n2.id).unwrap().is_empty());
    }

    #[test]
    fn find_orphans() {
        let (storage, _dir) = test_storage();
        let n1 = make_node(NodeType::Fact, "human");
        let n2 = make_node(NodeType::Fact, "human");
        let n3 = make_node(NodeType::Fact, "human");
        storage.insert_node(&n1).unwrap();
        storage.insert_node(&n2).unwrap();
        storage.insert_node(&n3).unwrap();
        storage
            .insert_edge(&make_edge(n1.id, n2.id, EdgeType::RelatedTo))
            .unwrap();

        let orphans = storage.find_orphans().unwrap();
        assert_eq!(orphans.len(), 1);
        assert_eq!(orphans[0].id, n3.id);
    }

    #[test]
    fn find_conflicts() {
        let (storage, _dir) = test_storage();
        let n1 = make_node(NodeType::Fact, "human");
        let n2 = make_node(NodeType::Fact, "ai");
        storage.insert_node(&n1).unwrap();
        storage.insert_node(&n2).unwrap();
        storage
            .insert_edge(&make_edge(n1.id, n2.id, EdgeType::Contradicts))
            .unwrap();

        let conflicts = storage.find_conflicts().unwrap();
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].node_a.id, n1.id);
        assert_eq!(conflicts[0].node_b.id, n2.id);
    }

    #[test]
    fn get_neighbors_traversal() {
        let (storage, _dir) = test_storage();
        let n1 = make_node(NodeType::Decision, "human");
        let n2 = make_node(NodeType::Decision, "human");
        let n3 = make_node(NodeType::Decision, "human");
        storage.insert_node(&n1).unwrap();
        storage.insert_node(&n2).unwrap();
        storage.insert_node(&n3).unwrap();
        storage
            .insert_edge(&make_edge(n1.id, n2.id, EdgeType::Requires))
            .unwrap();
        storage
            .insert_edge(&make_edge(n2.id, n3.id, EdgeType::Requires))
            .unwrap();

        let sub_1hop = storage.get_neighbors(n1.id, 1).unwrap();
        assert_eq!(sub_1hop.nodes.len(), 2);

        let sub_2hop = storage.get_neighbors(n1.id, 2).unwrap();
        assert_eq!(sub_2hop.nodes.len(), 3);
    }

    #[test]
    fn get_impact_follows_dependency_edges() {
        let (storage, _dir) = test_storage();
        let root = make_node(NodeType::Decision, "human");
        let child = make_node(NodeType::Decision, "human");
        let grandchild = make_node(NodeType::Decision, "human");
        let unrelated = make_node(NodeType::Decision, "human");
        storage.insert_node(&root).unwrap();
        storage.insert_node(&child).unwrap();
        storage.insert_node(&grandchild).unwrap();
        storage.insert_node(&unrelated).unwrap();

        storage
            .insert_edge(&make_edge(root.id, child.id, EdgeType::DerivedFrom))
            .unwrap();
        storage
            .insert_edge(&make_edge(child.id, grandchild.id, EdgeType::Requires))
            .unwrap();
        storage
            .insert_edge(&make_edge(root.id, unrelated.id, EdgeType::RelatedTo))
            .unwrap();

        let impact = storage.get_impact(root.id, &[]).unwrap();
        assert_eq!(impact.nodes.len(), 3);
        assert!(!impact.nodes.iter().any(|n| n.id == unrelated.id));
    }

    #[test]
    fn get_stats() {
        let (storage, _dir) = test_storage();
        let n1 = make_node(NodeType::Decision, "human");
        let n2 = make_node(NodeType::Fact, "ai");
        let n3 = make_node(NodeType::Decision, "human");
        storage.insert_node(&n1).unwrap();
        storage.insert_node(&n2).unwrap();
        storage.insert_node(&n3).unwrap();
        storage
            .insert_edge(&make_edge(n1.id, n2.id, EdgeType::Contradicts))
            .unwrap();

        let stats = storage.get_stats().unwrap();
        assert_eq!(stats.total_nodes, 3);
        assert_eq!(stats.total_edges, 1);
        assert_eq!(stats.by_type["decision"], 2);
        assert_eq!(stats.by_type["fact"], 1);
        assert_eq!(stats.by_status["pending"], 3);
        assert_eq!(stats.orphan_count, 1);
        assert_eq!(stats.conflict_count, 1);
    }

    #[test]
    fn audit_trail() {
        let (storage, _dir) = test_storage();
        let node = make_node(NodeType::Fact, "human");
        storage.insert_node(&node).unwrap();

        let entry = AuditEntry {
            id: Uuid::new_v4(),
            node_id: node.id,
            action: AuditAction::Create,
            actor_source: "user:alice".into(),
            actor_id: Some("device-1".into()),
            details: Some(serde_json::json!({"method": "api"})),
            created_at: Utc::now(),
        };
        storage.insert_audit(&entry).unwrap();

        let log = storage.get_audit_log(node.id, 10).unwrap();
        assert_eq!(log.len(), 1);
        assert_eq!(log[0].action, AuditAction::Create);
        assert_eq!(log[0].actor_source, "user:alice");
        assert_eq!(log[0].actor_id.as_deref(), Some("device-1"));
        assert!(log[0].details.is_some());
    }

    #[test]
    fn audit_preserved_on_node_delete() {
        let (storage, _dir) = test_storage();
        let node = make_node(NodeType::Fact, "human");
        storage.insert_node(&node).unwrap();

        let entry = AuditEntry {
            id: Uuid::new_v4(),
            node_id: node.id,
            action: AuditAction::Create,
            actor_source: "system".into(),
            actor_id: None,
            details: None,
            created_at: Utc::now(),
        };
        storage.insert_audit(&entry).unwrap();
        storage.delete_node(node.id).unwrap();

        let log = storage.get_audit_log(node.id, 10).unwrap();
        assert_eq!(log.len(), 1, "audit entries must be preserved after node deletion");
    }

    /// FIX CHECK: get_impact must only traverse FORWARD (outbound) edges.
    /// Given A --DerivedFrom--> B --DerivedFrom--> C,
    /// get_impact(B) should return {B, C} — only downstream nodes.
    /// A is upstream of B and should NOT be included in B's impact.
    #[test]
    fn get_impact_only_traverses_forward() {
        let (storage, _dir) = test_storage();
        let a = make_node(NodeType::Decision, "human");
        let b = make_node(NodeType::Decision, "human");
        let c = make_node(NodeType::Decision, "human");
        storage.insert_node(&a).unwrap();
        storage.insert_node(&b).unwrap();
        storage.insert_node(&c).unwrap();

        // A --DerivedFrom--> B --DerivedFrom--> C
        storage.insert_edge(&make_edge(a.id, b.id, EdgeType::DerivedFrom)).unwrap();
        storage.insert_edge(&make_edge(b.id, c.id, EdgeType::DerivedFrom)).unwrap();

        let impact = storage.get_impact(b.id, &[EdgeType::DerivedFrom]).unwrap();

        // Must NOT include A (upstream node)
        let has_a = impact.nodes.iter().any(|n| n.id == a.id);
        assert!(
            !has_a,
            "get_impact must not include upstream node A — only forward dependencies"
        );
        // Must include B (self) and C (downstream)
        assert_eq!(
            impact.nodes.len(), 2,
            "get_impact(B) should return exactly {{B, C}}, got {} nodes", impact.nodes.len()
        );
    }

    /// FIX CHECK: unknown AuditAction strings must return an error, not silently
    /// become AuditAction::Create. Silent fallback corrupts audit trail semantics.
    #[test]
    fn unknown_audit_action_returns_error() {
        let (storage, _dir) = test_storage();
        let node = make_node(NodeType::Fact, "human");
        storage.insert_node(&node).unwrap();

        // Insert an audit entry with an unknown action string directly via SQL
        let conn = storage.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO audit_log (id, node_id, action, actor_source, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                Uuid::new_v4().to_string(),
                node.id.to_string(),
                "archive",  // Unknown action from a newer schema version
                "system",
                Utc::now().to_rfc3339(),
            ],
        ).unwrap();
        drop(conn);

        // Reading an unknown action should return an error, not silently map to Create
        let result = storage.get_audit_log(node.id, 10);
        assert!(
            result.is_err(),
            "unknown audit action 'archive' should return an error, not silently become Create"
        );
    }

    /// FIX CHECK: audit entries must be preserved when a node is deleted.
    /// The audit trail is critical for compliance — cascade delete must not
    /// destroy audit records. Either remove the CASCADE or move audit to a
    /// separate table without foreign key constraints.
    #[test]
    fn audit_entries_preserved_after_node_delete() {
        let (storage, _dir) = test_storage();
        let node = make_node(NodeType::Fact, "human");
        storage.insert_node(&node).unwrap();

        let create_audit = AuditEntry {
            id: Uuid::new_v4(),
            node_id: node.id,
            action: AuditAction::Create,
            actor_source: "user".into(),
            actor_id: None,
            details: None,
            created_at: Utc::now(),
        };
        let delete_audit = AuditEntry {
            id: Uuid::new_v4(),
            node_id: node.id,
            action: AuditAction::Delete,
            actor_source: "user".into(),
            actor_id: None,
            details: None,
            created_at: Utc::now(),
        };
        storage.insert_audit(&create_audit).unwrap();
        storage.insert_audit(&delete_audit).unwrap();

        // Delete the node
        storage.delete_node(node.id).unwrap();

        // Audit entries must survive node deletion
        let log_after = storage.get_audit_log(node.id, 10).unwrap();
        assert_eq!(
            log_after.len(), 2,
            "audit entries must be preserved after node deletion for compliance"
        );
    }

    #[test]
    fn set_and_get_tags() {
        let (storage, _dir) = test_storage();
        let node = make_node(NodeType::Fact, "human");
        storage.insert_node(&node).unwrap();

        storage.set_tags(node.id, &["rust".into(), "database".into()]).unwrap();
        let tags = storage.get_tags(node.id).unwrap();
        assert_eq!(tags, vec!["database", "rust"]); // sorted
    }

    #[test]
    fn set_tags_replaces_existing() {
        let (storage, _dir) = test_storage();
        let node = make_node(NodeType::Fact, "human");
        storage.insert_node(&node).unwrap();

        storage.set_tags(node.id, &["old_tag".into()]).unwrap();
        storage.set_tags(node.id, &["new_tag".into()]).unwrap();
        let tags = storage.get_tags(node.id).unwrap();
        assert_eq!(tags, vec!["new_tag"]);
    }

    #[test]
    fn tags_cascade_on_node_delete() {
        let (storage, _dir) = test_storage();
        let node = make_node(NodeType::Fact, "human");
        storage.insert_node(&node).unwrap();
        storage.set_tags(node.id, &["important".into()]).unwrap();

        storage.delete_node(node.id).unwrap();
        let tags = storage.get_tags(node.id).unwrap();
        assert!(tags.is_empty());
    }

    #[test]
    fn list_tags_with_counts() {
        let (storage, _dir) = test_storage();
        let n1 = make_node(NodeType::Fact, "human");
        let n2 = make_node(NodeType::Decision, "human");
        let n3 = make_node(NodeType::Fact, "human");
        storage.insert_node(&n1).unwrap();
        storage.insert_node(&n2).unwrap();
        storage.insert_node(&n3).unwrap();

        storage.set_tags(n1.id, &["rust".into(), "backend".into()]).unwrap();
        storage.set_tags(n2.id, &["rust".into()]).unwrap();
        storage.set_tags(n3.id, &["backend".into(), "database".into()]).unwrap();

        let tags = storage.list_tags(10).unwrap();
        assert_eq!(tags.len(), 3);
        // sorted by count DESC
        assert_eq!(tags[0].tag, "backend");
        assert_eq!(tags[0].count, 2);
        assert_eq!(tags[1].tag, "rust");
        assert_eq!(tags[1].count, 2);
        assert_eq!(tags[2].tag, "database");
        assert_eq!(tags[2].count, 1);
    }

    #[test]
    fn list_nodes_filter_by_tags() {
        let (storage, _dir) = test_storage();
        let n1 = make_node(NodeType::Fact, "human");
        let n2 = make_node(NodeType::Fact, "human");
        let n3 = make_node(NodeType::Fact, "human");
        storage.insert_node(&n1).unwrap();
        storage.insert_node(&n2).unwrap();
        storage.insert_node(&n3).unwrap();

        storage.set_tags(n1.id, &["rust".into(), "backend".into()]).unwrap();
        storage.set_tags(n2.id, &["python".into()]).unwrap();
        storage.set_tags(n3.id, &["rust".into()]).unwrap();

        let rust_nodes = storage
            .list_nodes(None, None, None, Some(&["rust".into()]), 50, 0)
            .unwrap();
        assert_eq!(rust_nodes.len(), 2);

        let python_nodes = storage
            .list_nodes(None, None, None, Some(&["python".into()]), 50, 0)
            .unwrap();
        assert_eq!(python_nodes.len(), 1);

        let no_match = storage
            .list_nodes(None, None, None, Some(&["nonexistent".into()]), 50, 0)
            .unwrap();
        assert!(no_match.is_empty());
    }

    #[test]
    fn fill_tags_batch() {
        let (storage, _dir) = test_storage();
        let n1 = make_node(NodeType::Fact, "human");
        let n2 = make_node(NodeType::Fact, "human");
        storage.insert_node(&n1).unwrap();
        storage.insert_node(&n2).unwrap();

        storage.set_tags(n1.id, &["a".into(), "b".into()]).unwrap();
        storage.set_tags(n2.id, &["c".into()]).unwrap();

        let mut nodes = vec![n1.clone(), n2.clone()];
        storage.fill_tags_batch(&mut nodes).unwrap();
        assert_eq!(nodes[0].tags, vec!["a", "b"]);
        assert_eq!(nodes[1].tags, vec!["c"]);
    }

    #[test]
    fn source_string_preserved_exactly() {
        let (storage, _dir) = test_storage();
        let sources = ["human", "ai:gpt-4", "import:jira", "ai:agent-loop", "system"];
        for src in sources {
            let node = make_node(NodeType::Fact, src);
            storage.insert_node(&node).unwrap();
            let fetched = storage.get_node(node.id).unwrap().unwrap();
            assert_eq!(fetched.source, src);
        }
    }
}
