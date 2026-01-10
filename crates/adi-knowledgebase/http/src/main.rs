use adi_knowledgebase_core::{default_data_dir, EdgeType, Knowledgebase, NodeType};
use anyhow::Result;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post},
    Json, Router,
};
use serde::Deserialize;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing::info;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};
use uuid::Uuid;

struct AppState {
    kb: RwLock<Option<Knowledgebase>>,
    data_dir: PathBuf,
}

#[derive(Deserialize)]
struct QueryParams {
    q: String,
    #[serde(default = "default_limit")]
    limit: usize,
}

fn default_limit() -> usize {
    5
}

#[derive(Deserialize)]
struct AddRequest {
    user_said: String,
    derived_knowledge: String,
    #[serde(default)]
    node_type: Option<String>,
}

#[derive(Deserialize)]
struct LinkRequest {
    from_id: Uuid,
    to_id: Uuid,
    #[serde(default)]
    edge_type: Option<String>,
    #[serde(default)]
    weight: Option<f32>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let data_dir = if args.len() > 1 {
        PathBuf::from(&args[1])
    } else {
        default_data_dir()
    };

    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3001);

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(filter)
        .init();

    info!("Starting ADI Knowledgebase HTTP server");
    info!("Data directory: {}", data_dir.display());

    let kb = match Knowledgebase::open(&data_dir).await {
        Ok(kb) => Some(kb),
        Err(e) => {
            tracing::warn!("Failed to initialize Knowledgebase: {}", e);
            None
        }
    };

    let state = Arc::new(AppState {
        kb: RwLock::new(kb),
        data_dir,
    });

    let app = Router::new()
        .route("/", get(health))
        .route("/health", get(health))
        .route("/status", get(status))
        .route("/nodes", post(add_node))
        .route("/nodes/:id", get(get_node))
        .route("/nodes/:id", delete(delete_node))
        .route("/nodes/:id/approve", post(approve_node))
        .route("/query", get(query))
        .route("/subgraph", get(subgraph))
        .route("/conflicts", get(conflicts))
        .route("/orphans", get(orphans))
        .route("/edges", post(add_edge))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    info!("Listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn health() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "ok",
        "service": "adi-knowledgebase-http",
        "version": env!("CARGO_PKG_VERSION")
    }))
}

async fn status(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let kb = state.kb.read().await;

    match kb.as_ref() {
        Some(kb) => {
            let embedding_count = kb.storage().embedding.count();
            (
                StatusCode::OK,
                Json(serde_json::json!({
                    "initialized": true,
                    "data_dir": state.data_dir.display().to_string(),
                    "embeddings": embedding_count
                })),
            )
        }
        None => (
            StatusCode::OK,
            Json(serde_json::json!({
                "initialized": false,
                "data_dir": state.data_dir.display().to_string()
            })),
        ),
    }
}

async fn add_node(
    State(state): State<Arc<AppState>>,
    Json(req): Json<AddRequest>,
) -> impl IntoResponse {
    let kb = state.kb.read().await;

    let Some(kb) = kb.as_ref() else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({ "error": "Knowledgebase not initialized" })),
        );
    };

    let node_type = req
        .node_type
        .as_deref()
        .map(|s| match s {
            "decision" => NodeType::Decision,
            "fact" => NodeType::Fact,
            "error" => NodeType::Error,
            "guide" => NodeType::Guide,
            "glossary" => NodeType::Glossary,
            "context" => NodeType::Context,
            "assumption" => NodeType::Assumption,
            _ => NodeType::Fact,
        })
        .unwrap_or(NodeType::Fact);

    match kb
        .add_from_user(&req.user_said, &req.derived_knowledge, node_type)
        .await
    {
        Ok(node) => (
            StatusCode::CREATED,
            Json(serde_json::to_value(node).unwrap()),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        ),
    }
}

async fn get_node(State(state): State<Arc<AppState>>, Path(id): Path<Uuid>) -> impl IntoResponse {
    let kb = state.kb.read().await;

    let Some(kb) = kb.as_ref() else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({ "error": "Knowledgebase not initialized" })),
        );
    };

    match kb.get_node(id) {
        Ok(Some(node)) => (StatusCode::OK, Json(serde_json::to_value(node).unwrap())),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "Node not found" })),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        ),
    }
}

async fn delete_node(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    let kb = state.kb.read().await;

    let Some(kb) = kb.as_ref() else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({ "error": "Knowledgebase not initialized" })),
        );
    };

    match kb.delete_node(id) {
        Ok(()) => (
            StatusCode::OK,
            Json(serde_json::json!({ "deleted": id.to_string() })),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        ),
    }
}

async fn approve_node(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    let kb = state.kb.read().await;

    let Some(kb) = kb.as_ref() else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({ "error": "Knowledgebase not initialized" })),
        );
    };

    match kb.approve(id) {
        Ok(()) => (
            StatusCode::OK,
            Json(serde_json::json!({ "approved": id.to_string() })),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        ),
    }
}

async fn query(
    State(state): State<Arc<AppState>>,
    Query(params): Query<QueryParams>,
) -> impl IntoResponse {
    let kb = state.kb.read().await;

    let Some(kb) = kb.as_ref() else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({ "error": "Knowledgebase not initialized" })),
        );
    };

    match kb.query(&params.q).await {
        Ok(results) => {
            let results: Vec<_> = results.into_iter().take(params.limit).collect();
            (StatusCode::OK, Json(serde_json::to_value(results).unwrap()))
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        ),
    }
}

async fn subgraph(
    State(state): State<Arc<AppState>>,
    Query(params): Query<QueryParams>,
) -> impl IntoResponse {
    let kb = state.kb.read().await;

    let Some(kb) = kb.as_ref() else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({ "error": "Knowledgebase not initialized" })),
        );
    };

    match kb.query_subgraph(&params.q).await {
        Ok(subgraph) => (
            StatusCode::OK,
            Json(serde_json::to_value(subgraph).unwrap()),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        ),
    }
}

async fn conflicts(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let kb = state.kb.read().await;

    let Some(kb) = kb.as_ref() else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({ "error": "Knowledgebase not initialized" })),
        );
    };

    match kb.get_conflicts() {
        Ok(conflicts) => {
            let result: Vec<_> = conflicts
                .into_iter()
                .map(|(a, b)| {
                    serde_json::json!({
                        "node_a": a,
                        "node_b": b
                    })
                })
                .collect();
            (StatusCode::OK, Json(serde_json::to_value(result).unwrap()))
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        ),
    }
}

async fn orphans(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let kb = state.kb.read().await;

    let Some(kb) = kb.as_ref() else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({ "error": "Knowledgebase not initialized" })),
        );
    };

    match kb.get_orphans() {
        Ok(orphans) => (StatusCode::OK, Json(serde_json::to_value(orphans).unwrap())),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        ),
    }
}

async fn add_edge(
    State(state): State<Arc<AppState>>,
    Json(req): Json<LinkRequest>,
) -> impl IntoResponse {
    let kb = state.kb.read().await;

    let Some(kb) = kb.as_ref() else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({ "error": "Knowledgebase not initialized" })),
        );
    };

    let edge_type = req
        .edge_type
        .as_deref()
        .map(|s| match s {
            "supersedes" => EdgeType::Supersedes,
            "contradicts" => EdgeType::Contradicts,
            "requires" => EdgeType::Requires,
            "related_to" => EdgeType::RelatedTo,
            "derived_from" => EdgeType::DerivedFrom,
            "answers" => EdgeType::Answers,
            _ => EdgeType::RelatedTo,
        })
        .unwrap_or(EdgeType::RelatedTo);

    let weight = req.weight.unwrap_or(0.5);

    match kb.add_edge(req.from_id, req.to_id, edge_type, weight) {
        Ok(edge) => (
            StatusCode::CREATED,
            Json(serde_json::to_value(edge).unwrap()),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        ),
    }
}
