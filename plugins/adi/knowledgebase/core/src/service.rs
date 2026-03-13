include!(concat!(env!("OUT_DIR"), "/knowledgebase_adi_service.rs"));

use crate::embedder::Embedder;
use crate::models::{
    ApprovalStatus, AuditAction, AuditEntry, ConflictPair, DeleteResult, Edge, EdgeType, Node,
    NodeStats, NodeType, SearchResult, Subgraph,
};
use crate::search::{sanitize_weight, SearchConfig, SearchEngine};
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
                .embedding
                .delete(id)
                .map_err(|e| AdiServiceError::internal(e.to_string()))?;
            self.storage
                .embedding
                .insert(id, &embedding)
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
            weight: sanitize_weight(weight),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::embedder::DummyEmbedder;
    use crate::storage::Storage;
    use lib_adi_service::AdiCallerContext;
    use tempfile::TempDir;

    const DIMS: u32 = 32;

    fn test_service() -> (KnowledgebaseService, TempDir) {
        let dir = TempDir::new().unwrap();
        let embedder = Arc::new(DummyEmbedder::new(DIMS));
        let storage = Storage::open(dir.path(), DIMS as usize).unwrap();
        let service = KnowledgebaseService::new(storage, embedder);
        (service, dir)
    }

    fn ctx() -> AdiCallerContext {
        AdiCallerContext {
            user_id: Some("test-user".into()),
            device_id: Some("test-device".into()),
        }
    }

    #[tokio::test]
    async fn create_and_get_node() {
        let (svc, _dir) = test_service();
        let node = svc
            .create_node(
                &ctx(),
                "Use Postgres".into(),
                "We chose Postgres for storage".into(),
                NodeType::Decision,
                "human".into(),
                None,
            )
            .await
            .unwrap();

        assert_eq!(node.title, "Use Postgres");
        assert_eq!(node.source, "human");
        assert_eq!(node.approval_status, ApprovalStatus::Pending);

        let fetched = svc.get_node(&ctx(), node.id).await.unwrap();
        assert_eq!(fetched.id, node.id);
    }

    #[tokio::test]
    async fn get_nonexistent_returns_not_found() {
        let (svc, _dir) = test_service();
        let err = svc.get_node(&ctx(), Uuid::new_v4()).await.unwrap_err();
        assert_eq!(err.code, "not_found");
    }

    #[tokio::test]
    async fn update_node() {
        let (svc, _dir) = test_service();
        let node = svc
            .create_node(&ctx(), "Title".into(), "Content".into(), NodeType::Fact, "ai".into(), None)
            .await
            .unwrap();

        let updated = svc
            .update_node(&ctx(), node.id, Some("New Title".into()), None, None, None)
            .await
            .unwrap();
        assert_eq!(updated.title, "New Title");
        assert_eq!(updated.content, "Content");
    }

    #[tokio::test]
    async fn delete_node() {
        let (svc, _dir) = test_service();
        let node = svc
            .create_node(&ctx(), "T".into(), "C".into(), NodeType::Fact, "human".into(), None)
            .await
            .unwrap();

        let result = svc.delete_node(&ctx(), node.id).await.unwrap();
        assert!(result.deleted);

        let err = svc.get_node(&ctx(), node.id).await.unwrap_err();
        assert_eq!(err.code, "not_found");
    }

    #[tokio::test]
    async fn list_and_filter_nodes() {
        let (svc, _dir) = test_service();
        svc.create_node(&ctx(), "A".into(), "C".into(), NodeType::Decision, "human".into(), None)
            .await
            .unwrap();
        svc.create_node(&ctx(), "B".into(), "C".into(), NodeType::Fact, "ai:gpt-4".into(), None)
            .await
            .unwrap();

        let all = svc.list_nodes(&ctx(), None, None, None, None, None).await.unwrap();
        assert_eq!(all.len(), 2);

        let decisions = svc
            .list_nodes(&ctx(), Some(NodeType::Decision), None, None, None, None)
            .await
            .unwrap();
        assert_eq!(decisions.len(), 1);
    }

    #[tokio::test]
    async fn approval_workflow() {
        let (svc, _dir) = test_service();
        let node = svc
            .create_node(&ctx(), "T".into(), "C".into(), NodeType::Assumption, "ai".into(), None)
            .await
            .unwrap();
        assert_eq!(node.approval_status, ApprovalStatus::Pending);

        let pending = svc.list_pending(&ctx(), None).await.unwrap();
        assert_eq!(pending.len(), 1);

        let approved = svc.approve_node(&ctx(), node.id).await.unwrap();
        assert_eq!(approved.approval_status, ApprovalStatus::Approved);

        let pending = svc.list_pending(&ctx(), None).await.unwrap();
        assert!(pending.is_empty());
    }

    #[tokio::test]
    async fn reject_with_reason() {
        let (svc, _dir) = test_service();
        let node = svc
            .create_node(&ctx(), "T".into(), "C".into(), NodeType::Fact, "ai".into(), None)
            .await
            .unwrap();

        let rejected = svc
            .reject_node(&ctx(), node.id, Some("Inaccurate".into()))
            .await
            .unwrap();
        assert_eq!(rejected.approval_status, ApprovalStatus::Rejected);

        let log = svc.get_audit_log(&ctx(), node.id, None).await.unwrap();
        let reject_entry = log.iter().find(|e| e.action == AuditAction::Reject);
        assert!(reject_entry.is_some());
        let details = reject_entry.unwrap().details.as_ref().unwrap();
        assert_eq!(details["reason"], "Inaccurate");
    }

    #[tokio::test]
    async fn edge_crud() {
        let (svc, _dir) = test_service();
        let n1 = svc
            .create_node(&ctx(), "A".into(), "C".into(), NodeType::Decision, "human".into(), None)
            .await
            .unwrap();
        let n2 = svc
            .create_node(&ctx(), "B".into(), "C".into(), NodeType::Decision, "human".into(), None)
            .await
            .unwrap();

        let edge = svc
            .create_edge(&ctx(), n1.id, n2.id, EdgeType::DerivedFrom, None, None)
            .await
            .unwrap();
        assert_eq!(edge.edge_type, EdgeType::DerivedFrom);
        assert!((edge.weight - 1.0).abs() < f32::EPSILON);

        let edges = svc.get_edges(&ctx(), n1.id).await.unwrap();
        assert_eq!(edges.len(), 1);

        let del = svc.delete_edge(&ctx(), edge.id).await.unwrap();
        assert!(del.deleted);
    }

    #[tokio::test]
    async fn search_finds_created_nodes() {
        let (svc, _dir) = test_service();
        svc.create_node(
            &ctx(),
            "PostgreSQL decision".into(),
            "We chose Postgres for the main database".into(),
            NodeType::Decision,
            "human".into(),
            None,
        )
        .await
        .unwrap();

        let results = svc
            .search(&ctx(), "PostgreSQL decision".into(), Some(10), Some(0.0))
            .await
            .unwrap();
        assert!(!results.is_empty());
    }

    #[tokio::test]
    async fn find_duplicates() {
        let (svc, _dir) = test_service();
        svc.create_node(&ctx(), "Exact text".into(), "Exact content".into(), NodeType::Fact, "human".into(), None)
            .await
            .unwrap();

        let dupes = svc
            .find_duplicates(&ctx(), "Exact text Exact content".into(), Some(0.0))
            .await
            .unwrap();
        assert!(!dupes.is_empty());
    }

    #[tokio::test]
    async fn get_stats() {
        let (svc, _dir) = test_service();
        svc.create_node(&ctx(), "A".into(), "C".into(), NodeType::Decision, "human".into(), None)
            .await
            .unwrap();
        svc.create_node(&ctx(), "B".into(), "C".into(), NodeType::Fact, "ai".into(), None)
            .await
            .unwrap();

        let stats = svc.get_stats(&ctx()).await.unwrap();
        assert_eq!(stats.total_nodes, 2);
        assert_eq!(stats.orphan_count, 2);
    }

    #[tokio::test]
    async fn audit_trail_records_mutations() {
        let (svc, _dir) = test_service();
        let node = svc
            .create_node(&ctx(), "T".into(), "C".into(), NodeType::Fact, "human".into(), None)
            .await
            .unwrap();
        svc.update_node(&ctx(), node.id, Some("T2".into()), None, None, None)
            .await
            .unwrap();
        svc.approve_node(&ctx(), node.id).await.unwrap();

        let log = svc.get_audit_log(&ctx(), node.id, None).await.unwrap();
        assert_eq!(log.len(), 3); // create + update + approve

        let actions: Vec<_> = log.iter().map(|e| e.action).collect();
        assert!(actions.contains(&AuditAction::Create));
        assert!(actions.contains(&AuditAction::Update));
        assert!(actions.contains(&AuditAction::Approve));
    }

    #[tokio::test]
    async fn get_orphans() {
        let (svc, _dir) = test_service();
        let n1 = svc
            .create_node(&ctx(), "A".into(), "C".into(), NodeType::Fact, "human".into(), None)
            .await
            .unwrap();
        let n2 = svc
            .create_node(&ctx(), "B".into(), "C".into(), NodeType::Fact, "human".into(), None)
            .await
            .unwrap();
        svc.create_edge(&ctx(), n1.id, n2.id, EdgeType::RelatedTo, None, None)
            .await
            .unwrap();
        let orphan = svc
            .create_node(&ctx(), "Orphan".into(), "C".into(), NodeType::Fact, "human".into(), None)
            .await
            .unwrap();

        let orphans = svc.get_orphans(&ctx()).await.unwrap();
        assert_eq!(orphans.len(), 1);
        assert_eq!(orphans[0].id, orphan.id);
    }

    #[tokio::test]
    async fn get_conflicts() {
        let (svc, _dir) = test_service();
        let n1 = svc
            .create_node(&ctx(), "A".into(), "C".into(), NodeType::Fact, "human".into(), None)
            .await
            .unwrap();
        let n2 = svc
            .create_node(&ctx(), "B".into(), "C".into(), NodeType::Fact, "ai".into(), None)
            .await
            .unwrap();
        svc.create_edge(&ctx(), n1.id, n2.id, EdgeType::Contradicts, None, None)
            .await
            .unwrap();

        let conflicts = svc.get_conflicts(&ctx()).await.unwrap();
        assert_eq!(conflicts.len(), 1);
    }

    #[tokio::test]
    async fn get_impact() {
        let (svc, _dir) = test_service();
        let root = svc
            .create_node(&ctx(), "Root".into(), "C".into(), NodeType::Decision, "human".into(), None)
            .await
            .unwrap();
        let child = svc
            .create_node(&ctx(), "Child".into(), "C".into(), NodeType::Decision, "human".into(), None)
            .await
            .unwrap();
        svc.create_edge(&ctx(), root.id, child.id, EdgeType::DerivedFrom, None, None)
            .await
            .unwrap();

        let impact = svc.get_impact(&ctx(), root.id, None).await.unwrap();
        assert_eq!(impact.nodes.len(), 2);
    }
}
