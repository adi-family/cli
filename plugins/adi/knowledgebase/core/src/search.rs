use crate::error::Result;
use crate::models::{Edge, EdgeType, Node, SearchResult, Subgraph};
use crate::storage::Storage;

pub struct SearchConfig {
    pub top_k: usize,
    pub context_hops: usize,
    pub min_score: f32,
    pub similarity_weight: f32,
    pub edge_weight: f32,
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

pub struct SearchEngine<'a> {
    storage: &'a Storage,
    config: SearchConfig,
}

impl<'a> SearchEngine<'a> {
    pub fn new(storage: &'a Storage, config: SearchConfig) -> Self {
        Self { storage, config }
    }

    pub fn search(&self, query_embedding: &[f32]) -> Result<Vec<SearchResult>> {
        let candidates = self
            .storage
            .find_similar(query_embedding, self.config.top_k)?;

        let mut results = Vec::new();
        for (uuid, similarity_score) in candidates {
            if similarity_score < self.config.min_score {
                continue;
            }

            if let Some(node) = self.storage.get_node(uuid)? {
                let edges = self.storage.get_edges(uuid)?;
                let edge_score = calculate_edge_score(&edges);
                let recency_score = calculate_recency_score(&node);

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

        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        Ok(results)
    }

    pub fn get_subgraph(&self, query_embedding: &[f32]) -> Result<Subgraph> {
        let results = self.search(query_embedding)?;
        let mut subgraph = Subgraph::new();

        for result in results {
            let node_id = result.node.id;
            subgraph.add_node(result.node);

            for edge in result.edges {
                subgraph.add_edge(edge);
            }

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
}

fn calculate_edge_score(edges: &[Edge]) -> f32 {
    if edges.is_empty() {
        return 0.0;
    }

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

fn calculate_recency_score(node: &Node) -> f32 {
    let now = chrono::Utc::now();
    let age = now.signed_duration_since(node.updated_at);
    let hours = age.num_hours() as f32;
    (0.5_f32).powf(hours / 24.0)
}
