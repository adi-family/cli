mod generated;

#[cfg(feature = "mcp")]
mod mcp;

use anyhow::Result;
use async_trait::async_trait;
use axum::{routing::get, Json, Router};
use generated::models::*;
use generated::server::*;
use knowledgebase_core::{default_data_dir, Knowledgebase};
use lib_http_common::version_header_layer;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing::info;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};
use uuid::Uuid;
use lib_env_parse::{env_vars, env_opt};

env_vars! {
    Port => "PORT",
}

struct AppState {
    kb: Arc<RwLock<Option<Knowledgebase>>>,
    data_dir: PathBuf,
}

fn internal_error(e: impl std::fmt::Display) -> ApiError {
    ApiError {
        status: 500,
        code: "internal_error".to_string(),
        message: e.to_string(),
    }
}

fn unavailable() -> ApiError {
    ApiError {
        status: 503,
        code: "unavailable".to_string(),
        message: "Knowledgebase not initialized".to_string(),
    }
}

fn parse_node_type(s: Option<&str>) -> knowledgebase_core::NodeType {
    match s {
        Some("decision") => knowledgebase_core::NodeType::Decision,
        Some("fact") => knowledgebase_core::NodeType::Fact,
        Some("error") => knowledgebase_core::NodeType::Error,
        Some("guide") => knowledgebase_core::NodeType::Guide,
        Some("glossary") => knowledgebase_core::NodeType::Glossary,
        Some("context") => knowledgebase_core::NodeType::Context,
        Some("assumption") => knowledgebase_core::NodeType::Assumption,
        _ => knowledgebase_core::NodeType::Fact,
    }
}

fn parse_edge_type(s: Option<&str>) -> knowledgebase_core::EdgeType {
    match s {
        Some("supersedes") => knowledgebase_core::EdgeType::Supersedes,
        Some("contradicts") => knowledgebase_core::EdgeType::Contradicts,
        Some("requires") => knowledgebase_core::EdgeType::Requires,
        Some("related_to") => knowledgebase_core::EdgeType::RelatedTo,
        Some("derived_from") => knowledgebase_core::EdgeType::DerivedFrom,
        Some("answers") => knowledgebase_core::EdgeType::Answers,
        _ => knowledgebase_core::EdgeType::RelatedTo,
    }
}

/// Convert core types to generated models via serde Value (core and generated both derive Serialize/Deserialize)
fn json_convert<T: serde::Serialize, U: serde::de::DeserializeOwned>(
    val: &T,
) -> Result<U, ApiError> {
    serde_json::to_value(val)
        .and_then(|v| serde_json::from_value(v))
        .map_err(internal_error)
}

#[async_trait]
impl KnowledgebaseServiceHandler for AppState {
    async fn get_status(&self) -> Result<StatusResponse, ApiError> {
        let kb = self.kb.read().await;
        match kb.as_ref() {
            Some(kb) => {
                let embedding_count = kb.storage().embedding.count();
                Ok(StatusResponse {
                    initialized: true,
                    data_dir: self.data_dir.display().to_string(),
                    embeddings: Some(embedding_count as i32),
                })
            }
            None => Ok(StatusResponse {
                initialized: false,
                data_dir: self.data_dir.display().to_string(),
                embeddings: None,
            }),
        }
    }

    async fn add_node(&self, body: AddRequest) -> Result<Node, ApiError> {
        let kb = self.kb.read().await;
        let kb = kb.as_ref().ok_or_else(unavailable)?;
        let node_type = parse_node_type(body.node_type.as_deref());
        let node = kb
            .add_from_user(&body.user_said, &body.derived_knowledge, node_type)
            .await
            .map_err(internal_error)?;
        json_convert(&node)
    }

    async fn get_node(&self, id: Uuid) -> Result<Node, ApiError> {
        let kb = self.kb.read().await;
        let kb = kb.as_ref().ok_or_else(unavailable)?;
        match kb.get_node(id).map_err(internal_error)? {
            Some(node) => json_convert(&node),
            None => Err(ApiError {
                status: 404,
                code: "not_found".to_string(),
                message: "Node not found".to_string(),
            }),
        }
    }

    async fn delete_node(&self, id: Uuid) -> Result<DeletedResponse, ApiError> {
        let kb = self.kb.read().await;
        let kb = kb.as_ref().ok_or_else(unavailable)?;
        kb.delete_node(id).map_err(internal_error)?;
        Ok(DeletedResponse { deleted: id })
    }

    async fn approve_node(&self, id: Uuid) -> Result<ApprovedResponse, ApiError> {
        let kb = self.kb.read().await;
        let kb = kb.as_ref().ok_or_else(unavailable)?;
        kb.approve(id).map_err(internal_error)?;
        Ok(ApprovedResponse { approved: id })
    }

    async fn query(
        &self,
        query: KnowledgebaseServiceQueryQuery,
    ) -> Result<Vec<SearchResult>, ApiError> {
        let kb = self.kb.read().await;
        let kb = kb.as_ref().ok_or_else(unavailable)?;
        let limit = query.limit.map(|l| l as usize).unwrap_or(5);
        let results = kb.query(&query.q).await.map_err(internal_error)?;
        let results: Vec<_> = results.into_iter().take(limit).collect();
        json_convert(&results)
    }

    async fn subgraph(
        &self,
        query: KnowledgebaseServiceSubgraphQuery,
    ) -> Result<Subgraph, ApiError> {
        let kb = self.kb.read().await;
        let kb = kb.as_ref().ok_or_else(unavailable)?;
        let subgraph = kb.query_subgraph(&query.q).await.map_err(internal_error)?;
        json_convert(&subgraph)
    }

    async fn get_conflicts(&self) -> Result<Vec<ConflictPair>, ApiError> {
        let kb = self.kb.read().await;
        let kb = kb.as_ref().ok_or_else(unavailable)?;
        let conflicts = kb.get_conflicts().map_err(internal_error)?;
        Ok(conflicts
            .into_iter()
            .map(|(a, b)| ConflictPair {
                node_a: a.id,
                node_b: b.id,
            })
            .collect())
    }

    async fn get_orphans(&self) -> Result<Vec<Node>, ApiError> {
        let kb = self.kb.read().await;
        let kb = kb.as_ref().ok_or_else(unavailable)?;
        let orphans = kb.get_orphans().map_err(internal_error)?;
        json_convert(&orphans)
    }

    async fn add_edge(&self, body: LinkRequest) -> Result<Edge, ApiError> {
        let kb = self.kb.read().await;
        let kb = kb.as_ref().ok_or_else(unavailable)?;
        let edge_type = parse_edge_type(body.edge_type.as_deref());
        let weight = body.weight.unwrap_or(0.5);
        let edge = kb
            .add_edge(body.from_id, body.to_id, edge_type, weight)
            .map_err(internal_error)?;
        json_convert(&edge)
    }
}

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ok",
        "service": "adi-knowledgebase-http",
        "version": env!("CARGO_PKG_VERSION")
    }))
}

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let data_dir = if args.len() > 1 {
        PathBuf::from(&args[1])
    } else {
        default_data_dir()
    };

    let port: u16 = env_opt(EnvVar::Port.as_str())
        .and_then(|p| p.parse().ok())
        .unwrap_or(3001);

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(filter)
        .init();

    info!("Starting ADI Knowledgebase HTTP server");
    info!("Data directory: {}", data_dir.display());

    #[cfg(feature = "mcp")]
    info!("MCP support enabled at /mcp/sse (SSE) and /mcp/message (POST)");

    #[allow(deprecated)] // standalone HTTP binary uses fastembed, not plugin manager
    let kb = match Knowledgebase::open(&data_dir).await {
        Ok(kb) => Some(kb),
        Err(e) => {
            tracing::warn!("Failed to initialize Knowledgebase: {}", e);
            None
        }
    };

    let kb = RwLock::new(kb);
    let kb_arc = Arc::new(kb);

    let state = Arc::new(AppState {
        kb: kb_arc.clone(),
        data_dir,
    });

    // Build REST API router
    let rest_router = Router::new()
        .route("/", get(health))
        .route("/health", get(health))
        .merge(generated::server::create_router::<AppState>())
        .with_state(state);

    // Build MCP router if feature is enabled
    #[cfg(feature = "mcp")]
    let mcp_router = mcp::create_mcp_router(kb_arc);

    // Combine routers
    #[cfg(feature = "mcp")]
    let app = Router::new()
        .merge(rest_router)
        .nest("/mcp", mcp_router)
        .layer(version_header_layer(
            env!("CARGO_PKG_NAME"),
            env!("CARGO_PKG_VERSION"),
        ))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http());

    #[cfg(not(feature = "mcp"))]
    let app = rest_router
        .layer(version_header_layer(
            env!("CARGO_PKG_NAME"),
            env!("CARGO_PKG_VERSION"),
        ))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http());

    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], port));
    info!("Listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
