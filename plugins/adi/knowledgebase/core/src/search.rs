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

pub fn sanitize_weight(weight: Option<f32>) -> f32 {
    let w = weight.unwrap_or(1.0);
    if w.is_finite() { w.clamp(0.0, 1.0) } else { 0.0 }
}

fn calculate_recency_score(node: &Node) -> f32 {
    let now = chrono::Utc::now();
    let age = now.signed_duration_since(node.updated_at);
    let hours = age.num_hours().max(0) as f32;
    (0.5_f32).powf(hours / 24.0).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{ApprovalStatus, NodeType};
    use crate::storage::Storage;
    use chrono::Utc;
    use tempfile::TempDir;
    use uuid::Uuid;

    const DIMS: usize = 4;

    fn test_storage() -> (Storage, TempDir) {
        let dir = TempDir::new().unwrap();
        let storage = Storage::open(dir.path(), DIMS).unwrap();
        (storage, dir)
    }

    fn make_node(title: &str, content: &str) -> Node {
        let now = Utc::now();
        Node {
            id: Uuid::new_v4(),
            node_type: NodeType::Fact,
            title: title.into(),
            content: content.into(),
            source: "human".into(),
            approval_status: ApprovalStatus::Approved,
            metadata: serde_json::json!({}),
            tags: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }

    #[test]
    fn search_returns_results_above_min_score() {
        let (storage, _dir) = test_storage();
        let node = make_node("Postgres", "Use PostgreSQL for storage");
        let embedding = vec![1.0, 0.0, 0.0, 0.0];
        storage.store_node(&node, &embedding).unwrap();

        let config = SearchConfig {
            top_k: 10,
            min_score: 0.0,
            ..Default::default()
        };
        let engine = SearchEngine::new(&storage, config);
        let results = engine.search(&[1.0, 0.0, 0.0, 0.0]).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].node.id, node.id);
        assert!(results[0].score > 0.0);
    }

    #[test]
    fn search_filters_below_min_score() {
        let (storage, _dir) = test_storage();
        let node = make_node("Postgres", "DB");
        storage.store_node(&node, &[1.0, 0.0, 0.0, 0.0]).unwrap();

        let config = SearchConfig {
            top_k: 10,
            min_score: 0.99,
            ..Default::default()
        };
        let engine = SearchEngine::new(&storage, config);
        // Orthogonal query vector — low similarity
        let results = engine.search(&[0.0, 1.0, 0.0, 0.0]).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn search_sorted_by_score_descending() {
        let (storage, _dir) = test_storage();
        let n1 = make_node("Close", "A");
        let n2 = make_node("Far", "B");
        storage.store_node(&n1, &[0.9, 0.1, 0.0, 0.0]).unwrap();
        storage.store_node(&n2, &[0.0, 0.0, 1.0, 0.0]).unwrap();

        let config = SearchConfig {
            min_score: 0.0,
            ..Default::default()
        };
        let engine = SearchEngine::new(&storage, config);
        let results = engine.search(&[1.0, 0.0, 0.0, 0.0]).unwrap();
        if results.len() >= 2 {
            assert!(results[0].score >= results[1].score);
        }
    }

    #[test]
    fn get_subgraph_includes_neighbors() {
        let (storage, _dir) = test_storage();
        let n1 = make_node("Root", "Root node");
        let n2 = make_node("Neighbor", "Neighbor node");
        storage.store_node(&n1, &[1.0, 0.0, 0.0, 0.0]).unwrap();
        storage.store_node(&n2, &[0.0, 1.0, 0.0, 0.0]).unwrap();
        storage
            .insert_edge(&Edge {
                id: Uuid::new_v4(),
                from_id: n1.id,
                to_id: n2.id,
                edge_type: EdgeType::RelatedTo,
                weight: 1.0,
                metadata: serde_json::json!({}),
                created_at: Utc::now(),
            })
            .unwrap();

        let config = SearchConfig {
            min_score: 0.0,
            context_hops: 1,
            ..Default::default()
        };
        let engine = SearchEngine::new(&storage, config);
        let subgraph = engine.get_subgraph(&[1.0, 0.0, 0.0, 0.0]).unwrap();
        assert!(subgraph.nodes.len() >= 2);
    }

    #[test]
    fn edge_score_calculation() {
        let edges_high = vec![Edge {
            id: Uuid::new_v4(),
            from_id: Uuid::new_v4(),
            to_id: Uuid::new_v4(),
            edge_type: EdgeType::Requires,
            weight: 1.0,
            metadata: serde_json::json!({}),
            created_at: Utc::now(),
        }];
        let edges_low = vec![Edge {
            id: Uuid::new_v4(),
            from_id: Uuid::new_v4(),
            to_id: Uuid::new_v4(),
            edge_type: EdgeType::Contradicts,
            weight: 1.0,
            metadata: serde_json::json!({}),
            created_at: Utc::now(),
        }];

        assert!(calculate_edge_score(&edges_high) > calculate_edge_score(&edges_low));
        assert_eq!(calculate_edge_score(&[]), 0.0);
    }

    /// FIX CHECK: recency_score must be clamped to [0, 1] even for future timestamps.
    /// Future `updated_at` (clock skew, data corruption) must not produce scores > 1.0
    /// because that corrupts search ranking relative to other score components.
    #[test]
    fn recency_score_clamped_for_future_timestamps() {
        let now = Utc::now();
        let future_node = Node {
            id: Uuid::new_v4(),
            node_type: NodeType::Fact,
            title: "Future".into(),
            content: "x".into(),
            source: "human".into(),
            approval_status: ApprovalStatus::Approved,
            metadata: serde_json::json!({}),
            tags: Vec::new(),
            created_at: now,
            updated_at: now + chrono::Duration::days(7),
        };
        let score = calculate_recency_score(&future_node);
        assert!(
            score <= 1.0,
            "recency score for future node should be clamped to [0, 1], got {score}"
        );
        assert!(
            score >= 0.0,
            "recency score should never be negative, got {score}"
        );
    }

    /// FIX CHECK: edge weight must be validated as finite before use.
    /// NaN weight passes through clamp(0.0, 1.0) unchanged because NaN comparisons
    /// are always false. This propagates NaN into search scores, crashing sort_by.
    /// Fix: validate weight.is_finite() before storing, or sanitize in edge_score.
    #[test]
    fn edge_weight_must_be_finite() {
        // Simulate what create_edge does: sanitize_weight
        let weight: Option<f32> = Some(f32::NAN);
        let stored_weight = sanitize_weight(weight);
        assert!(
            stored_weight.is_finite(),
            "edge weight must be finite after validation, got NaN. \
             Fix: check is_finite() before clamp, or default NaN to 0.0"
        );
    }

    #[test]
    fn recency_score_recent_is_higher() {
        let now = Utc::now();
        let recent = Node {
            id: Uuid::new_v4(),
            node_type: NodeType::Fact,
            title: "Recent".into(),
            content: "x".into(),
            source: "human".into(),
            approval_status: ApprovalStatus::Approved,
            metadata: serde_json::json!({}),
            tags: Vec::new(),
            created_at: now,
            updated_at: now,
        };
        let old = Node {
            id: Uuid::new_v4(),
            node_type: NodeType::Fact,
            title: "Old".into(),
            content: "x".into(),
            source: "human".into(),
            approval_status: ApprovalStatus::Approved,
            metadata: serde_json::json!({}),
            tags: Vec::new(),
            created_at: now - chrono::Duration::days(30),
            updated_at: now - chrono::Duration::days(30),
        };

        assert!(calculate_recency_score(&recent) > calculate_recency_score(&old));
    }
}
