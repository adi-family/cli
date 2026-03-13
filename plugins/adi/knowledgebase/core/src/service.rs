include!(concat!(env!("OUT_DIR"), "/knowledgebase_adi_service.rs"));

use crate::embedder::Embedder;
use crate::models::{
    ApprovalStatus, AuditAction, AuditEntry, ConflictPair, DeleteResult, Edge, EdgeType, Node,
    NodeStats, NodeType, SearchResult, Subgraph,
};
use crate::search::{SearchConfig, SearchEngine};
use crate::storage::Storage;
use chrono::Utc;
use std::sync::Arc;
use uuid::Uuid;

pub struct KnowledgebaseService {
    storage: Storage,
    embedder: Arc<dyn Embedder>,
}

impl KnowledgebaseService {
    pub fn new(storage: Storage, embedder: Arc<dyn Embedder>) -> Self {
        Self { storage, embedder }
    }

    fn embed_text(&self, text: &str) -> Result<Vec<f32>, AdiServiceError> {
        let results = self
            .embedder
            .embed(&[text])
            .map_err(|e| AdiServiceError::internal(e.to_string()))?;
        results
            .into_iter()
            .next()
            .ok_or_else(|| AdiServiceError::internal("No embedding returned"))
    }

    fn record_audit(
        &self,
        node_id: Uuid,
        action: AuditAction,
        ctx: &AdiCallerContext,
        details: Option<serde_json::Value>,
    ) {
        let entry = AuditEntry {
            id: Uuid::new_v4(),
            node_id,
            action,
            actor_source: ctx
                .user_id
                .clone()
                .unwrap_or_else(|| "system".to_string()),
            actor_id: ctx.device_id.clone(),
            details,
            created_at: Utc::now(),
        };
        let _ = self.storage.insert_audit(&entry);
    }
}

#[async_trait]
impl KnowledgebaseServiceHandler for KnowledgebaseService {
    async fn create_node(
        &self,
        ctx: &AdiCallerContext,
        title: String,
        content: String,
        node_type: NodeType,
        source: String,
        metadata: Option<std::collections::HashMap<String, serde_json::Value>>,
    ) -> Result<Node, AdiServiceError> {
        let now = Utc::now();
        let node = Node {
            id: Uuid::new_v4(),
            node_type,
            title,
            content,
            source,
            approval_status: ApprovalStatus::Pending,
            metadata: metadata
                .map(|m| serde_json::to_value(m).unwrap_or_default())
                .unwrap_or_else(|| serde_json::json!({})),
            created_at: now,
            updated_at: now,
        };

        let embedding = self.embed_text(&node.embedding_content())?;
        self.storage
            .store_node(&node, &embedding)
            .map_err(|e| AdiServiceError::internal(e.to_string()))?;

        self.record_audit(node.id, AuditAction::Create, ctx, None);
        Ok(node)
    }

    async fn get_node(
        &self,
        _ctx: &AdiCallerContext,
        id: Uuid,
    ) -> Result<Node, AdiServiceError> {
        self.storage
            .get_node(id)
            .map_err(|e| AdiServiceError::internal(e.to_string()))?
            .ok_or_else(|| AdiServiceError::not_found("Node not found"))
    }

    async fn update_node(
        &self,
        ctx: &AdiCallerContext,
        id: Uuid,
        title: Option<String>,
        content: Option<String>,
        node_type: Option<NodeType>,
        metadata: Option<std::collections::HashMap<String, serde_json::Value>>,
    ) -> Result<Node, AdiServiceError> {
        let metadata_value = metadata.map(|m| serde_json::to_value(m).unwrap_or_default());

        let node = self
            .storage
            .update_node(
                id,
                title.as_deref(),
                content.as_deref(),
                node_type,
                metadata_value.as_ref(),
            )
            .map_err(|e| AdiServiceError::internal(e.to_string()))?
            .ok_or_else(|| AdiServiceError::not_found("Node not found"))?;

        // Re-embed if content changed
        if content.is_some() || title.is_some() {
            let embedding = self.embed_text(&node.embedding_content())?;
            self.storage
                .store_node(&node, &embedding)
                .map_err(|e| AdiServiceError::internal(e.to_string()))?;
        }

        self.record_audit(
            id,
            AuditAction::Update,
            ctx,
            Some(serde_json::json!({"fields_changed": true})),
        );
        Ok(node)
    }

    async fn delete_node(
        &self,
        ctx: &AdiCallerContext,
        id: Uuid,
    ) -> Result<DeleteResult, AdiServiceError> {
        self.record_audit(id, AuditAction::Delete, ctx, None);

        let deleted = self
            .storage
            .delete_node(id)
            .map_err(|e| AdiServiceError::internal(e.to_string()))?;

        if !deleted {
            return Err(AdiServiceError::not_found("Node not found"));
        }

        Ok(DeleteResult { deleted: true })
    }

    async fn list_nodes(
        &self,
        _ctx: &AdiCallerContext,
        node_type: Option<NodeType>,
        approval_status: Option<ApprovalStatus>,
        source: Option<String>,
        limit: Option<i32>,
        offset: Option<i32>,
    ) -> Result<Vec<Node>, AdiServiceError> {
        self.storage
            .list_nodes(
                node_type,
                approval_status,
                source.as_deref(),
                limit.unwrap_or(50),
                offset.unwrap_or(0),
            )
            .map_err(|e| AdiServiceError::internal(e.to_string()))
    }

    async fn approve_node(
        &self,
        ctx: &AdiCallerContext,
        id: Uuid,
    ) -> Result<Node, AdiServiceError> {
        let node = self
            .storage
            .update_approval_status(id, ApprovalStatus::Approved)
            .map_err(|e| AdiServiceError::internal(e.to_string()))?
            .ok_or_else(|| AdiServiceError::not_found("Node not found"))?;

        self.record_audit(id, AuditAction::Approve, ctx, None);
        Ok(node)
    }

    async fn reject_node(
        &self,
        ctx: &AdiCallerContext,
        id: Uuid,
        reason: Option<String>,
    ) -> Result<Node, AdiServiceError> {
        let node = self
            .storage
            .update_approval_status(id, ApprovalStatus::Rejected)
            .map_err(|e| AdiServiceError::internal(e.to_string()))?
            .ok_or_else(|| AdiServiceError::not_found("Node not found"))?;

        self.record_audit(
            id,
            AuditAction::Reject,
            ctx,
            reason.map(|r| serde_json::json!({"reason": r})),
        );
        Ok(node)
    }

    async fn list_pending(
        &self,
        _ctx: &AdiCallerContext,
        limit: Option<i32>,
    ) -> Result<Vec<Node>, AdiServiceError> {
        self.storage
            .list_nodes(None, Some(ApprovalStatus::Pending), None, limit.unwrap_or(50), 0)
            .map_err(|e| AdiServiceError::internal(e.to_string()))
    }

    async fn create_edge(
        &self,
        _ctx: &AdiCallerContext,
        from_id: Uuid,
        to_id: Uuid,
        edge_type: EdgeType,
        weight: Option<f32>,
        metadata: Option<std::collections::HashMap<String, serde_json::Value>>,
    ) -> Result<Edge, AdiServiceError> {
        // Verify both nodes exist
        self.storage
            .get_node(from_id)
            .map_err(|e| AdiServiceError::internal(e.to_string()))?
            .ok_or_else(|| AdiServiceError::not_found("Source node not found"))?;
        self.storage
            .get_node(to_id)
            .map_err(|e| AdiServiceError::internal(e.to_string()))?
            .ok_or_else(|| AdiServiceError::not_found("Target node not found"))?;

        let edge = Edge {
            id: Uuid::new_v4(),
            from_id,
            to_id,
            edge_type,
            weight: weight.unwrap_or(1.0).clamp(0.0, 1.0),
            metadata: metadata
                .map(|m| serde_json::to_value(m).unwrap_or_default())
                .unwrap_or_else(|| serde_json::json!({})),
            created_at: Utc::now(),
        };

        self.storage
            .insert_edge(&edge)
            .map_err(|e| AdiServiceError::internal(e.to_string()))?;

        Ok(edge)
    }

    async fn delete_edge(
        &self,
        _ctx: &AdiCallerContext,
        id: Uuid,
    ) -> Result<DeleteResult, AdiServiceError> {
        let deleted = self
            .storage
            .delete_edge(id)
            .map_err(|e| AdiServiceError::internal(e.to_string()))?;

        if !deleted {
            return Err(AdiServiceError::not_found("Edge not found"));
        }

        Ok(DeleteResult { deleted: true })
    }

    async fn get_edges(
        &self,
        _ctx: &AdiCallerContext,
        node_id: Uuid,
    ) -> Result<Vec<Edge>, AdiServiceError> {
        self.storage
            .get_edges(node_id)
            .map_err(|e| AdiServiceError::internal(e.to_string()))
    }

    async fn search(
        &self,
        _ctx: &AdiCallerContext,
        query: String,
        limit: Option<i32>,
        min_score: Option<f32>,
    ) -> Result<Vec<SearchResult>, AdiServiceError> {
        let embedding = self.embed_text(&query)?;
        let config = SearchConfig {
            top_k: limit.unwrap_or(10) as usize,
            min_score: min_score.unwrap_or(0.3),
            ..Default::default()
        };
        let engine = SearchEngine::new(&self.storage, config);
        engine
            .search(&embedding)
            .map_err(|e| AdiServiceError::internal(e.to_string()))
    }

    async fn get_subgraph(
        &self,
        _ctx: &AdiCallerContext,
        query: String,
        hops: Option<i32>,
        limit: Option<i32>,
    ) -> Result<Subgraph, AdiServiceError> {
        let embedding = self.embed_text(&query)?;
        let config = SearchConfig {
            top_k: limit.unwrap_or(10) as usize,
            context_hops: hops.unwrap_or(2) as usize,
            ..Default::default()
        };
        let engine = SearchEngine::new(&self.storage, config);
        engine
            .get_subgraph(&embedding)
            .map_err(|e| AdiServiceError::internal(e.to_string()))
    }

    async fn get_neighbors(
        &self,
        _ctx: &AdiCallerContext,
        node_id: Uuid,
        hops: Option<i32>,
    ) -> Result<Subgraph, AdiServiceError> {
        self.storage
            .get_neighbors(node_id, hops.unwrap_or(2) as usize)
            .map_err(|e| AdiServiceError::internal(e.to_string()))
    }

    async fn get_impact(
        &self,
        _ctx: &AdiCallerContext,
        node_id: Uuid,
        edge_types: Option<Vec<EdgeType>>,
    ) -> Result<Subgraph, AdiServiceError> {
        let types = edge_types.unwrap_or_default();
        self.storage
            .get_impact(node_id, &types)
            .map_err(|e| AdiServiceError::internal(e.to_string()))
    }

    async fn get_conflicts(
        &self,
        _ctx: &AdiCallerContext,
    ) -> Result<Vec<ConflictPair>, AdiServiceError> {
        self.storage
            .find_conflicts()
            .map_err(|e| AdiServiceError::internal(e.to_string()))
    }

    async fn get_orphans(
        &self,
        _ctx: &AdiCallerContext,
    ) -> Result<Vec<Node>, AdiServiceError> {
        self.storage
            .find_orphans()
            .map_err(|e| AdiServiceError::internal(e.to_string()))
    }

    async fn find_duplicates(
        &self,
        _ctx: &AdiCallerContext,
        content: String,
        threshold: Option<f32>,
    ) -> Result<Vec<SearchResult>, AdiServiceError> {
        let embedding = self.embed_text(&content)?;
        let threshold = threshold.unwrap_or(0.85);
        let candidates = self
            .storage
            .find_similar(&embedding, 10)
            .map_err(|e| AdiServiceError::internal(e.to_string()))?;

        let mut results = Vec::new();
        for (uuid, score) in candidates {
            if score >= threshold {
                if let Some(node) = self
                    .storage
                    .get_node(uuid)
                    .map_err(|e| AdiServiceError::internal(e.to_string()))?
                {
                    let edges = self
                        .storage
                        .get_edges(uuid)
                        .map_err(|e| AdiServiceError::internal(e.to_string()))?;
                    results.push(SearchResult { node, score, edges });
                }
            }
        }
        Ok(results)
    }

    async fn get_audit_log(
        &self,
        _ctx: &AdiCallerContext,
        node_id: Uuid,
        limit: Option<i32>,
    ) -> Result<Vec<AuditEntry>, AdiServiceError> {
        self.storage
            .get_audit_log(node_id, limit.unwrap_or(100))
            .map_err(|e| AdiServiceError::internal(e.to_string()))
    }

    async fn get_stats(
        &self,
        _ctx: &AdiCallerContext,
    ) -> Result<NodeStats, AdiServiceError> {
        self.storage
            .get_stats()
            .map_err(|e| AdiServiceError::internal(e.to_string()))
    }
}
