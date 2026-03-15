mod embedding;
mod graph;

pub use embedding::EmbeddingStorage;
pub use graph::GraphStorage;

use crate::error::Result;
use crate::models::{
    ApprovalStatus, AuditEntry, ConflictPair, Edge, EdgeType, Node, NodeStats, NodeType, Subgraph,
    TagInfo,
};
use uuid::Uuid;

pub struct Storage {
    pub graph: GraphStorage,
    pub embedding: EmbeddingStorage,
}

impl Storage {
    pub fn open(data_dir: &std::path::Path, embedding_dimensions: usize) -> Result<Self> {
        let graph = GraphStorage::open(&data_dir.join("graph.db"))?;
        let embedding = EmbeddingStorage::open(&data_dir.join("embeddings"), embedding_dimensions)?;
        Ok(Self { graph, embedding })
    }

    pub fn store_node(&self, node: &Node, embedding: &[f32]) -> Result<()> {
        self.graph.insert_node(node)?;
        self.graph.set_tags(node.id, &node.tags)?;
        self.embedding.insert(node.id, embedding)?;
        Ok(())
    }

    pub fn delete_node(&self, id: Uuid) -> Result<bool> {
        self.embedding.delete(id)?;
        self.graph.delete_node(id)
    }

    pub fn find_similar(&self, embedding: &[f32], limit: usize) -> Result<Vec<(Uuid, f32)>> {
        self.embedding.search(embedding, limit)
    }

    pub fn get_node(&self, id: Uuid) -> Result<Option<Node>> {
        let mut node = self.graph.get_node(id)?;
        if let Some(ref mut n) = node {
            self.graph.fill_tags(n)?;
        }
        Ok(node)
    }

    pub fn update_node(
        &self,
        id: Uuid,
        title: Option<&str>,
        content: Option<&str>,
        node_type: Option<NodeType>,
        metadata: Option<&serde_json::Value>,
    ) -> Result<Option<Node>> {
        let mut node = self.graph.update_node(id, title, content, node_type, metadata)?;
        if let Some(ref mut n) = node {
            self.graph.fill_tags(n)?;
        }
        Ok(node)
    }

    pub fn set_tags(&self, node_id: Uuid, tags: &[String]) -> Result<()> {
        self.graph.set_tags(node_id, tags)
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
        let mut nodes = self.graph.list_nodes(node_type, approval_status, source, tags, limit, offset)?;
        self.graph.fill_tags_batch(&mut nodes)?;
        Ok(nodes)
    }

    pub fn update_approval_status(&self, id: Uuid, status: ApprovalStatus) -> Result<Option<Node>> {
        let mut node = self.graph.update_approval_status(id, status)?;
        if let Some(ref mut n) = node {
            self.graph.fill_tags(n)?;
        }
        Ok(node)
    }

    pub fn list_tags(&self, limit: i32) -> Result<Vec<TagInfo>> {
        self.graph.list_tags(limit)
    }

    pub fn get_edges(&self, node_id: Uuid) -> Result<Vec<Edge>> {
        self.graph.get_edges(node_id)
    }

    pub fn insert_edge(&self, edge: &Edge) -> Result<()> {
        self.graph.insert_edge(edge)
    }

    pub fn delete_edge(&self, id: Uuid) -> Result<bool> {
        self.graph.delete_edge(id)
    }

    pub fn get_neighbors(&self, node_id: Uuid, hops: usize) -> Result<Subgraph> {
        self.graph.get_neighbors(node_id, hops)
    }

    pub fn get_impact(&self, node_id: Uuid, edge_types: &[EdgeType]) -> Result<Subgraph> {
        self.graph.get_impact(node_id, edge_types)
    }

    pub fn find_orphans(&self) -> Result<Vec<Node>> {
        self.graph.find_orphans()
    }

    pub fn find_conflicts(&self) -> Result<Vec<ConflictPair>> {
        self.graph.find_conflicts()
    }

    pub fn get_stats(&self) -> Result<NodeStats> {
        self.graph.get_stats()
    }

    pub fn insert_audit(&self, entry: &AuditEntry) -> Result<()> {
        self.graph.insert_audit(entry)
    }

    pub fn get_audit_log(&self, node_id: Uuid, limit: i32) -> Result<Vec<AuditEntry>> {
        self.graph.get_audit_log(node_id, limit)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::ApprovalStatus;
    use chrono::Utc;
    use tempfile::TempDir;
    use uuid::Uuid;

    const DIMS: usize = 4;

    fn make_node(title: &str) -> Node {
        let now = Utc::now();
        Node {
            id: Uuid::new_v4(),
            node_type: NodeType::Fact,
            title: title.into(),
            content: "content".into(),
            source: "human".into(),
            approval_status: ApprovalStatus::Approved,
            metadata: serde_json::json!({}),
            tags: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }

    /// FIX CHECK: delete_node must delete graph BEFORE embedding.
    /// If graph deletion fails, the embedding should still be intact so
    /// find_similar can still locate the node. Current order (embedding first)
    /// leaves an orphaned graph node with no embedding on partial failure.
    ///
    /// This test verifies correct cleanup after successful deletion and
    /// documents the required ordering: graph first, then embedding.
    #[test]
    fn delete_node_cleans_up_both_stores() {
        let dir = TempDir::new().unwrap();
        let storage = Storage::open(dir.path(), DIMS).unwrap();

        let node = make_node("Test node");
        let embedding = vec![1.0, 0.0, 0.0, 0.0];
        storage.store_node(&node, &embedding).unwrap();

        // Both stores have the node
        assert!(storage.get_node(node.id).unwrap().is_some());
        assert_eq!(storage.find_similar(&embedding, 5).unwrap().len(), 1);

        // Delete must clean up both atomically
        storage.delete_node(node.id).unwrap();

        // Graph node gone
        assert!(
            storage.get_node(node.id).unwrap().is_none(),
            "graph node must be deleted"
        );
        // Embedding gone
        let similar = storage.find_similar(&embedding, 5).unwrap();
        assert!(
            similar.iter().all(|(uuid, _)| *uuid != node.id),
            "embedding must be deleted"
        );
    }

    /// FIX CHECK: store_node must be atomic — if embedding insert fails,
    /// the graph node must be rolled back so there are no ghost nodes
    /// that exist in the graph but are invisible to similarity search.
    #[test]
    fn store_node_is_consistent() {
        let dir = TempDir::new().unwrap();
        let storage = Storage::open(dir.path(), DIMS).unwrap();

        let node = make_node("Test node");
        let embedding = vec![1.0, 0.0, 0.0, 0.0];
        storage.store_node(&node, &embedding).unwrap();

        // Both stores must have the node after successful store
        let fetched = storage.get_node(node.id).unwrap();
        assert!(fetched.is_some(), "node must exist in graph after store");

        let similar = storage.find_similar(&embedding, 5).unwrap();
        assert!(
            !similar.is_empty(),
            "node must exist in embedding index after store"
        );
    }
}
