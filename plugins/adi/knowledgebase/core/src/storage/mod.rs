mod embedding;
mod graph;

pub use embedding::EmbeddingStorage;
pub use graph::GraphStorage;

use crate::error::Result;
use crate::models::{
    ApprovalStatus, AuditEntry, ConflictPair, Edge, EdgeType, Node, NodeStats, NodeType, Subgraph,
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
        self.graph.get_node(id)
    }

    pub fn update_node(
        &self,
        id: Uuid,
        title: Option<&str>,
        content: Option<&str>,
        node_type: Option<NodeType>,
        metadata: Option<&serde_json::Value>,
    ) -> Result<Option<Node>> {
        self.graph.update_node(id, title, content, node_type, metadata)
    }

    pub fn list_nodes(
        &self,
        node_type: Option<NodeType>,
        approval_status: Option<ApprovalStatus>,
        source: Option<&str>,
        limit: i32,
        offset: i32,
    ) -> Result<Vec<Node>> {
        self.graph.list_nodes(node_type, approval_status, source, limit, offset)
    }

    pub fn update_approval_status(&self, id: Uuid, status: ApprovalStatus) -> Result<Option<Node>> {
        self.graph.update_approval_status(id, status)
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
