use crate::error::Result;
use crate::storage::Storage;
use crate::types::{Edge, EdgeType, Node, SearchResult, Subgraph};
use uuid::Uuid;

/// Search configuration
pub struct SearchConfig {
    /// Number of top results from embedding search
    pub top_k: usize,
    /// Number of hops for context expansion
    pub context_hops: usize,
    /// Minimum similarity score threshold
    pub min_score: f32,
    /// Weight for embedding similarity in final ranking
    pub similarity_weight: f32,
    /// Weight for edge strength in final ranking
    pub edge_weight: f32,
    /// Weight for recency in final ranking
    pub recency_weight: f32,
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            top_k: 10,
            context_hops: 2,
            min_score: 0.3,
            similarity_weight: 0.5,
            edge_weight: 0.3,
            recency_weight: 0.2,
        }
    }
}

/// Hybrid search engine combining embedding similarity and graph traversal
pub struct SearchEngine<'a> {
    storage: &'a Storage,
    config: SearchConfig,
}

impl<'a> SearchEngine<'a> {
    pub fn new(storage: &'a Storage, config: SearchConfig) -> Self {
        Self { storage, config }
    }

    /// Perform hybrid search: embedding similarity + graph context
    pub async fn search(&self, query_embedding: &[f32]) -> Result<Vec<SearchResult>> {
        // Step 1: Find similar nodes by embedding
        let candidates = self
            .storage
            .find_similar(query_embedding, self.config.top_k)?;

        // Step 2: Load candidate nodes and expand context
        let mut results = Vec::new();
        for (uuid, similarity_score) in candidates {
            if similarity_score < self.config.min_score {
                continue;
            }

            if let Some(node) = self.storage.get_node(uuid)? {
                // Update access time
                self.storage.touch_node(uuid)?;

                // Get edges for context
                let edges = self.storage.get_edges(uuid)?;

                // Calculate final score
                let edge_score = self.calculate_edge_score(&edges);
                let recency_score = self.calculate_recency_score(&node);

                let final_score = similarity_score * self.config.similarity_weight
                    + edge_score * self.config.edge_weight
                    + recency_score * self.config.recency_weight;

                results.push(SearchResult {
                    node,
                    score: final_score,
                    edges,
                });
            }
        }

        // Sort by final score
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        Ok(results)
    }

    /// Get subgraph for agent consumption
    pub async fn get_subgraph(&self, query_embedding: &[f32]) -> Result<Subgraph> {
        let results = self.search(query_embedding).await?;
        let mut subgraph = Subgraph::new();

        for result in results {
            // Add main node
            let node_id = result.node.id;
            subgraph.add_node(result.node);

            // Add edges
            for edge in result.edges {
                subgraph.add_edge(edge);
            }

            // Expand context with N-hop neighbors
            let neighbors = self
                .storage
                .get_neighbors(node_id, self.config.context_hops)?;
            for node in neighbors.nodes {
                subgraph.add_node(node);
            }
            for edge in neighbors.edges {
                subgraph.add_edge(edge);
            }
        }

        Ok(subgraph)
    }

    fn calculate_edge_score(&self, edges: &[Edge]) -> f32 {
        if edges.is_empty() {
            return 0.0;
        }

        // Weight different edge types
        let mut score = 0.0;
        for edge in edges {
            let type_weight = match edge.edge_type {
                EdgeType::Requires => 0.9,
                EdgeType::Answers => 0.8,
                EdgeType::DerivedFrom => 0.7,
                EdgeType::RelatedTo => 0.5,
                EdgeType::Supersedes => 0.3,
                EdgeType::Contradicts => 0.1,
            };
            score += edge.weight * type_weight;
        }
        (score / edges.len() as f32).min(1.0)
    }

    fn calculate_recency_score(&self, node: &Node) -> f32 {
        let now = chrono::Utc::now();
        let age = now.signed_duration_since(node.last_accessed_at);
        let hours = age.num_hours() as f32;

        // Decay over time (half-life of 24 hours)
        (0.5_f32).powf(hours / 24.0)
    }
}

/// Conflict detection
pub struct ConflictDetector<'a> {
    storage: &'a Storage,
}

impl<'a> ConflictDetector<'a> {
    pub fn new(storage: &'a Storage) -> Self {
        Self { storage }
    }

    /// Check if a new node conflicts with existing nodes
    pub async fn detect_conflicts(&self, embedding: &[f32], threshold: f32) -> Result<Vec<Uuid>> {
        let similar = self.storage.find_similar(embedding, 10)?;
        let conflicts: Vec<Uuid> = similar
            .into_iter()
            .filter(|(_, score)| *score > threshold)
            .map(|(uuid, _)| uuid)
            .collect();
        Ok(conflicts)
    }

    /// Get all existing conflicts
    pub fn get_conflicts(&self) -> Result<Vec<(Node, Node)>> {
        self.storage.find_conflicts()
    }
}

/// Duplicate detection
pub struct DuplicateDetector<'a> {
    storage: &'a Storage,
    threshold: f32,
}

impl<'a> DuplicateDetector<'a> {
    pub fn new(storage: &'a Storage, threshold: f32) -> Self {
        Self { storage, threshold }
    }

    /// Check if content is a duplicate of existing nodes
    pub async fn find_duplicates(&self, embedding: &[f32]) -> Result<Vec<(Uuid, f32)>> {
        let similar = self.storage.find_similar(embedding, 5)?;
        let duplicates: Vec<(Uuid, f32)> = similar
            .into_iter()
            .filter(|(_, score)| *score > self.threshold)
            .collect();
        Ok(duplicates)
    }
}
