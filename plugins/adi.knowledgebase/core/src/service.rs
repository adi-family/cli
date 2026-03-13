use lib_adi_service::{
    AdiCallerContext, AdiHandleResult, AdiMethodInfo, AdiPluginCapabilities,
    AdiService, AdiServiceError, SubscriptionEvent, SubscriptionEventInfo,
};
use async_trait::async_trait;
use bytes::Bytes;
use serde_json::{json, Value as JsonValue};
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex};
use uuid::Uuid;

use crate::{EdgeType, Knowledgebase, NodeType};

pub struct KnowledgebaseService {
    kb: Arc<Mutex<Knowledgebase>>,
    event_tx: broadcast::Sender<SubscriptionEvent>,
}

impl KnowledgebaseService {
    pub fn new(kb: Knowledgebase) -> Self {
        let (event_tx, _) = broadcast::channel(256);
        Self {
            kb: Arc::new(Mutex::new(kb)),
            event_tx,
        }
    }

    #[cfg(feature = "fastembed")]
    pub async fn open_default() -> Result<Self, String> {
        let data_dir = crate::default_data_dir();
        #[allow(deprecated)]
        let kb = Knowledgebase::open(&data_dir)
            .await
            .map_err(|e| format!("Failed to open knowledgebase: {e}"))?;
        Ok(Self::new(kb))
    }

    fn broadcast_event(&self, event: &str, data: JsonValue) {
        let _ = self.event_tx.send(SubscriptionEvent {
            event: event.to_string(),
            data,
        });
    }

    fn parse_node_type(s: Option<&str>) -> NodeType {
        match s {
            Some("decision") => NodeType::Decision,
            Some("fact") => NodeType::Fact,
            Some("error") => NodeType::Error,
            Some("guide") => NodeType::Guide,
            Some("glossary") => NodeType::Glossary,
            Some("context") => NodeType::Context,
            Some("assumption") => NodeType::Assumption,
            _ => NodeType::Fact,
        }
    }

    fn parse_edge_type(s: Option<&str>) -> EdgeType {
        match s {
            Some("supersedes") => EdgeType::Supersedes,
            Some("contradicts") => EdgeType::Contradicts,
            Some("requires") => EdgeType::Requires,
            Some("related_to") => EdgeType::RelatedTo,
            Some("derived_from") => EdgeType::DerivedFrom,
            Some("answers") => EdgeType::Answers,
            _ => EdgeType::RelatedTo,
        }
    }
}

fn json_to_bytes(value: JsonValue) -> Bytes {
    Bytes::from(serde_json::to_vec(&value).unwrap())
}

fn node_to_json(node: &crate::Node) -> JsonValue {
    json!({
        "id": node.id,
        "node_type": format!("{:?}", node.node_type).to_lowercase(),
        "title": node.title,
        "content": node.content,
        "confidence": node.confidence.0,
        "created_at": node.created_at.to_rfc3339(),
        "updated_at": node.updated_at.to_rfc3339(),
        "metadata": node.metadata,
    })
}

fn edge_to_json(edge: &crate::Edge) -> JsonValue {
    json!({
        "id": edge.id,
        "from_id": edge.from_id,
        "to_id": edge.to_id,
        "edge_type": format!("{:?}", edge.edge_type).to_lowercase(),
        "weight": edge.weight,
        "created_at": edge.created_at.to_rfc3339(),
    })
}

#[async_trait]
impl AdiService for KnowledgebaseService {
    fn plugin_id(&self) -> &str { "adi.knowledgebase" }
    fn name(&self) -> &str { "Knowledgebase" }
    fn version(&self) -> &str { env!("CARGO_PKG_VERSION") }

    fn capabilities(&self) -> AdiPluginCapabilities {
        AdiPluginCapabilities { subscriptions: true, notifications: true, streaming: false }
    }

    fn methods(&self) -> Vec<AdiMethodInfo> {
        vec![
            AdiMethodInfo { name: "get_status".into(), description: "Get knowledgebase status".into(), ..Default::default() },
            AdiMethodInfo { name: "add".into(), description: "Add knowledge node".into(), ..Default::default() },
            AdiMethodInfo { name: "get".into(), description: "Get node by ID".into(), ..Default::default() },
            AdiMethodInfo { name: "delete".into(), description: "Delete node".into(), ..Default::default() },
            AdiMethodInfo { name: "approve".into(), description: "Approve node".into(), ..Default::default() },
            AdiMethodInfo { name: "query".into(), description: "Semantic search".into(), ..Default::default() },
            AdiMethodInfo { name: "subgraph".into(), description: "Get subgraph for query".into(), ..Default::default() },
            AdiMethodInfo { name: "conflicts".into(), description: "List conflicting nodes".into(), ..Default::default() },
            AdiMethodInfo { name: "orphans".into(), description: "List orphan nodes".into(), ..Default::default() },
            AdiMethodInfo { name: "link".into(), description: "Add edge between nodes".into(), ..Default::default() },
        ]
    }

    async fn handle(&self, _ctx: &AdiCallerContext, method: &str, payload: Bytes) -> Result<AdiHandleResult, AdiServiceError> {
        let params: JsonValue = if payload.is_empty() { json!({}) } else {
            serde_json::from_slice(&payload).map_err(|e| AdiServiceError::invalid_params(format!("Invalid JSON: {e}")))?
        };

        let result = match method {
            "get_status" => {
                let kb = self.kb.lock().await;
                json_to_bytes(json!({ "initialized": true, "data_dir": kb.data_dir().to_string_lossy(), "embeddings": kb.storage().embedding.count() }))
            }
            "add" => {
                let user_said = params["user_said"].as_str().ok_or_else(|| AdiServiceError::invalid_params("missing user_said"))?;
                let derived = params["derived_knowledge"].as_str().ok_or_else(|| AdiServiceError::invalid_params("missing derived_knowledge"))?;
                let node_type = Self::parse_node_type(params["node_type"].as_str());
                let kb = self.kb.lock().await;
                let node = kb.add_from_user(user_said, derived, node_type).await.map_err(|e| AdiServiceError::internal(e.to_string()))?;
                self.broadcast_event("node_added", node_to_json(&node));
                json_to_bytes(node_to_json(&node))
            }
            "get" => {
                let id: Uuid = serde_json::from_value(params["id"].clone()).map_err(|_| AdiServiceError::invalid_params("missing or invalid id"))?;
                let kb = self.kb.lock().await;
                let node = kb.get_node(id).map_err(|e| AdiServiceError::internal(e.to_string()))?.ok_or_else(|| AdiServiceError::not_found(format!("Node {id} not found")))?;
                json_to_bytes(node_to_json(&node))
            }
            "delete" => {
                let id: Uuid = serde_json::from_value(params["id"].clone()).map_err(|_| AdiServiceError::invalid_params("missing or invalid id"))?;
                let kb = self.kb.lock().await;
                kb.delete_node(id).map_err(|e| AdiServiceError::internal(e.to_string()))?;
                self.broadcast_event("node_deleted", json!({ "id": id }));
                json_to_bytes(json!({ "deleted": true }))
            }
            "approve" => {
                let id: Uuid = serde_json::from_value(params["id"].clone()).map_err(|_| AdiServiceError::invalid_params("missing or invalid id"))?;
                let kb = self.kb.lock().await;
                kb.approve(id).map_err(|e| AdiServiceError::internal(e.to_string()))?;
                self.broadcast_event("node_approved", json!({ "id": id }));
                json_to_bytes(json!({ "approved": true }))
            }
            "query" => {
                let q = params["q"].as_str().ok_or_else(|| AdiServiceError::invalid_params("missing q"))?;
                let kb = self.kb.lock().await;
                let results = kb.query(q).await.map_err(|e| AdiServiceError::internal(e.to_string()))?;
                let arr: Vec<JsonValue> = results.iter().map(|r| json!({ "node": node_to_json(&r.node), "score": r.score, "edges": r.edges.iter().map(edge_to_json).collect::<Vec<_>>() })).collect();
                json_to_bytes(json!(arr))
            }
            "subgraph" => {
                let q = params["q"].as_str().ok_or_else(|| AdiServiceError::invalid_params("missing q"))?;
                let kb = self.kb.lock().await;
                let sg = kb.query_subgraph(q).await.map_err(|e| AdiServiceError::internal(e.to_string()))?;
                json_to_bytes(json!({ "nodes": sg.nodes.iter().map(node_to_json).collect::<Vec<_>>(), "edges": sg.edges.iter().map(edge_to_json).collect::<Vec<_>>() }))
            }
            "conflicts" => {
                let kb = self.kb.lock().await;
                let conflicts = kb.get_conflicts().map_err(|e| AdiServiceError::internal(e.to_string()))?;
                let pairs: Vec<JsonValue> = conflicts.into_iter().map(|(a, b)| json!({ "node_a": node_to_json(&a), "node_b": node_to_json(&b) })).collect();
                json_to_bytes(json!(pairs))
            }
            "orphans" => {
                let kb = self.kb.lock().await;
                let orphans = kb.get_orphans().map_err(|e| AdiServiceError::internal(e.to_string()))?;
                json_to_bytes(json!(orphans.iter().map(node_to_json).collect::<Vec<_>>()))
            }
            "link" => {
                let from_id: Uuid = serde_json::from_value(params["from_id"].clone()).map_err(|_| AdiServiceError::invalid_params("missing or invalid from_id"))?;
                let to_id: Uuid = serde_json::from_value(params["to_id"].clone()).map_err(|_| AdiServiceError::invalid_params("missing or invalid to_id"))?;
                let edge_type = Self::parse_edge_type(params["edge_type"].as_str());
                let weight = params["weight"].as_f64().unwrap_or(1.0) as f32;
                let kb = self.kb.lock().await;
                let edge = kb.add_edge(from_id, to_id, edge_type, weight).map_err(|e| AdiServiceError::internal(e.to_string()))?;
                self.broadcast_event("edge_added", edge_to_json(&edge));
                json_to_bytes(edge_to_json(&edge))
            }
            _ => return Err(AdiServiceError::method_not_found(method)),
        };
        Ok(AdiHandleResult::Success(result))
    }

    fn subscription_events(&self) -> Vec<SubscriptionEventInfo> {
        vec![
            SubscriptionEventInfo { name: "node_added".into(), description: "Node added".into(), data_schema: None },
            SubscriptionEventInfo { name: "node_deleted".into(), description: "Node deleted".into(), data_schema: None },
            SubscriptionEventInfo { name: "node_approved".into(), description: "Node approved".into(), data_schema: None },
            SubscriptionEventInfo { name: "edge_added".into(), description: "Edge added".into(), data_schema: None },
        ]
    }

    async fn subscribe(&self, _event: &str, _filter: Option<JsonValue>) -> Result<broadcast::Receiver<SubscriptionEvent>, AdiServiceError> {
        Ok(self.event_tx.subscribe())
    }
}
