mod embedding;
mod graph;

pub use embedding::EmbeddingStorage;
pub use graph::GraphStorage;

use crate::error::Result;
use crate::types::{Edge, Node, Subgraph};
use uuid::Uuid;

/// Combined storage for knowledgebase
pub struct Storage {
    pub graph: GraphStorage,
    pub embedding: EmbeddingStorage,
}

impl Storage {
    pub fn open(data_dir: &std::path::Path) -> Result<Self> {
        let graph = GraphStorage::open(&data_dir.join("graph.db"))?;
        let embedding = EmbeddingStorage::open(&data_dir.join("embeddings"))?;
        Ok(Self { graph, embedding })
    }

    /// Store a node with its embedding
    pub async fn store_node(&self, node: &Node, embedding: &[f32]) -> Result<()> {
        self.graph.insert_node(node)?;
        self.embedding.insert(node.id, embedding)?;
        Ok(())
    }

    /// Delete a node and its embedding
    pub fn delete_node(&self, id: Uuid) -> Result<()> {
        self.graph.delete_node(id)?;
        self.embedding.delete(id)?;
        Ok(())
    }

    /// Find similar nodes by embedding
    pub fn find_similar(&self, embedding: &[f32], limit: usize) -> Result<Vec<(Uuid, f32)>> {
        self.embedding.search(embedding, limit)
    }

    /// Get node by ID
    pub fn get_node(&self, id: Uuid) -> Result<Option<Node>> {
        self.graph.get_node(id)
    }

    /// Get nodes by IDs
    pub fn get_nodes(&self, ids: &[Uuid]) -> Result<Vec<Node>> {
        self.graph.get_nodes(ids)
    }

    /// Get edges for a node
    pub fn get_edges(&self, node_id: Uuid) -> Result<Vec<Edge>> {
        self.graph.get_edges(node_id)
    }

    /// Insert an edge
    pub fn insert_edge(&self, edge: &Edge) -> Result<()> {
        self.graph.insert_edge(edge)
    }

    /// Get N-hop neighbors
    pub fn get_neighbors(&self, node_id: Uuid, hops: usize) -> Result<Subgraph> {
        self.graph.get_neighbors(node_id, hops)
    }

    /// Update node access time
    pub fn touch_node(&self, id: Uuid) -> Result<()> {
        self.graph.touch_node(id)
    }

    /// Find orphan nodes (no edges)
    pub fn find_orphans(&self) -> Result<Vec<Uuid>> {
        self.graph.find_orphans()
    }

    /// Find nodes with contradicts edges
    pub fn find_conflicts(&self) -> Result<Vec<(Node, Node)>> {
        self.graph.find_conflicts()
    }
}
