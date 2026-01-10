// Copyright (c) 2024-2025 Ihor
// SPDX-License-Identifier: BSL-1.1
// See LICENSE file for details

use anyhow::Result;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing::info;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

struct AppState {
    adi: RwLock<Option<adi_indexer_core::Adi>>,
    project_path: PathBuf,
}

#[derive(Deserialize)]
struct SearchQuery {
    q: String,
    #[serde(default = "default_limit")]
    limit: usize,
}

fn default_limit() -> usize {
    10
}

#[derive(Deserialize)]
struct DeadCodeQuery {
    #[serde(default = "default_mode")]
    mode: String,
    #[serde(default = "default_true")]
    exclude_tests: bool,
    #[serde(default = "default_true")]
    exclude_traits: bool,
    #[serde(default = "default_true")]
    exclude_ffi: bool,
}

fn default_mode() -> String {
    "strict".to_string()
}

fn default_true() -> bool {
    true
}

#[derive(Serialize)]
#[allow(dead_code)]
struct ErrorResponse {
    error: String,
}

#[derive(Serialize)]
#[allow(dead_code)]
struct SuccessResponse<T> {
    data: T,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Parse args
    let args: Vec<String> = std::env::args().collect();
    let project_path = if args.len() > 1 {
        PathBuf::from(&args[1])
    } else {
        std::env::current_dir()?
    };

    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3000);

    // Setup logging
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(filter)
        .init();

    info!("Starting ADI HTTP server");
    info!("Project path: {}", project_path.display());

    // Initialize ADI
    let adi = match adi_indexer_core::Adi::open(&project_path).await {
        Ok(adi) => Some(adi),
        Err(e) => {
            tracing::warn!("Failed to initialize ADI: {}. Run /index first.", e);
            None
        }
    };

    let state = Arc::new(AppState {
        adi: RwLock::new(adi),
        project_path: project_path.canonicalize()?,
    });

    let app = Router::new()
        .route("/", get(health))
        .route("/health", get(health))
        .route("/status", get(status))
        .route("/index", post(index_project))
        .route("/search", get(search))
        .route("/symbols", get(search_symbols))
        .route("/symbols/:id", get(get_symbol))
        .route("/symbols/:id/reachability", get(get_symbol_reachability))
        .route("/files", get(search_files))
        .route("/files/*path", get(get_file))
        .route("/tree", get(get_tree))
        .route("/dead-code", get(find_dead_code))
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
        "service": "adi-http",
        "version": env!("CARGO_PKG_VERSION")
    }))
}

async fn status(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let adi = state.adi.read().await;

    match adi.as_ref() {
        Some(adi) => match adi.status() {
            Ok(status) => (StatusCode::OK, Json(serde_json::to_value(status).unwrap())),
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e.to_string() })),
            ),
        },
        None => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({ "error": "ADI not initialized. POST /index first." })),
        ),
    }
}

async fn index_project(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    // Re-initialize ADI
    let adi = match adi_indexer_core::Adi::open(&state.project_path).await {
        Ok(adi) => adi,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e.to_string() })),
            );
        }
    };

    // Index
    let progress = match adi.index().await {
        Ok(p) => p,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e.to_string() })),
            );
        }
    };

    // Store new ADI instance
    *state.adi.write().await = Some(adi);

    (
        StatusCode::OK,
        Json(serde_json::to_value(progress).unwrap()),
    )
}

async fn search(
    State(state): State<Arc<AppState>>,
    Query(query): Query<SearchQuery>,
) -> impl IntoResponse {
    let adi = state.adi.read().await;

    match adi.as_ref() {
        Some(adi) => match adi.search(&query.q, query.limit).await {
            Ok(results) => (StatusCode::OK, Json(serde_json::to_value(results).unwrap())),
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e.to_string() })),
            ),
        },
        None => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({ "error": "ADI not initialized" })),
        ),
    }
}

async fn search_symbols(
    State(state): State<Arc<AppState>>,
    Query(query): Query<SearchQuery>,
) -> impl IntoResponse {
    let adi = state.adi.read().await;

    match adi.as_ref() {
        Some(adi) => match adi.search_symbols(&query.q, query.limit).await {
            Ok(results) => (StatusCode::OK, Json(serde_json::to_value(results).unwrap())),
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e.to_string() })),
            ),
        },
        None => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({ "error": "ADI not initialized" })),
        ),
    }
}

async fn get_symbol(State(state): State<Arc<AppState>>, Path(id): Path<i64>) -> impl IntoResponse {
    let adi = state.adi.read().await;

    match adi.as_ref() {
        Some(adi) => match adi.get_symbol(adi_indexer_core::SymbolId(id)) {
            Ok(symbol) => (StatusCode::OK, Json(serde_json::to_value(symbol).unwrap())),
            Err(e) => (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({ "error": e.to_string() })),
            ),
        },
        None => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({ "error": "ADI not initialized" })),
        ),
    }
}

async fn search_files(
    State(state): State<Arc<AppState>>,
    Query(query): Query<SearchQuery>,
) -> impl IntoResponse {
    let adi = state.adi.read().await;

    match adi.as_ref() {
        Some(adi) => match adi.search_files(&query.q, query.limit).await {
            Ok(results) => (StatusCode::OK, Json(serde_json::to_value(results).unwrap())),
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e.to_string() })),
            ),
        },
        None => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({ "error": "ADI not initialized" })),
        ),
    }
}

async fn get_file(
    State(state): State<Arc<AppState>>,
    Path(path): Path<String>,
) -> impl IntoResponse {
    let adi = state.adi.read().await;

    match adi.as_ref() {
        Some(adi) => match adi.get_file(std::path::Path::new(&path)) {
            Ok(file_info) => (
                StatusCode::OK,
                Json(serde_json::to_value(file_info).unwrap()),
            ),
            Err(e) => (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({ "error": e.to_string() })),
            ),
        },
        None => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({ "error": "ADI not initialized" })),
        ),
    }
}

async fn get_tree(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let adi = state.adi.read().await;

    match adi.as_ref() {
        Some(adi) => match adi.get_tree() {
            Ok(tree) => (StatusCode::OK, Json(serde_json::to_value(tree).unwrap())),
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e.to_string() })),
            ),
        },
        None => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({ "error": "ADI not initialized" })),
        ),
    }
}

async fn find_dead_code(
    State(state): State<Arc<AppState>>,
    Query(query): Query<DeadCodeQuery>,
) -> impl IntoResponse {
    use adi_indexer_core::analyzer::{AnalysisConfig, AnalysisMode, DeadCodeAnalyzer};

    let mode = match query.mode.as_str() {
        "library" => AnalysisMode::Library,
        "application" => AnalysisMode::Application,
        _ => AnalysisMode::Strict,
    };

    let config = AnalysisConfig {
        mode,
        exclude_tests: query.exclude_tests,
        exclude_traits: query.exclude_traits,
        exclude_ffi: query.exclude_ffi,
        exclude_patterns: vec![],
    };

    match adi_indexer_core::SqliteStorage::open(&state.project_path.join(".adi/tree/index.sqlite"))
    {
        Ok(storage) => {
            let analyzer = DeadCodeAnalyzer::new(Arc::new(storage), config);
            match analyzer.analyze() {
                Ok(report) => (StatusCode::OK, Json(serde_json::to_value(report).unwrap())),
                Err(e) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({ "error": e.to_string() })),
                ),
            }
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        ),
    }
}

async fn get_symbol_reachability(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> impl IntoResponse {
    use adi_indexer_core::analyzer::{EntryPointDetector, ReachabilityAnalyzer};

    let adi = state.adi.read().await;

    match adi.as_ref() {
        Some(adi) => {
            match adi_indexer_core::SqliteStorage::open(
                &state.project_path.join(".adi/tree/index.sqlite"),
            ) {
                Ok(storage) => {
                    let storage_arc = Arc::new(storage);

                    let entry_detector = EntryPointDetector::new(storage_arc.clone());
                    let entry_points = match entry_detector.detect_entry_points() {
                        Ok(ep) => ep,
                        Err(e) => {
                            return (
                                StatusCode::INTERNAL_SERVER_ERROR,
                                Json(serde_json::json!({ "error": e.to_string() })),
                            );
                        }
                    };

                    let reachability_analyzer = ReachabilityAnalyzer::new(storage_arc);
                    let is_reachable = match reachability_analyzer
                        .is_reachable(adi_indexer_core::SymbolId(id), &entry_points)
                    {
                        Ok(r) => r,
                        Err(e) => {
                            return (
                                StatusCode::INTERNAL_SERVER_ERROR,
                                Json(serde_json::json!({ "error": e.to_string() })),
                            );
                        }
                    };

                    match adi.get_symbol(adi_indexer_core::SymbolId(id)) {
                        Ok(symbol) => (
                            StatusCode::OK,
                            Json(serde_json::json!({
                                "symbol_id": id,
                                "symbol_name": symbol.name,
                                "is_reachable": is_reachable,
                                "status": if is_reachable { "reachable" } else { "dead_code" }
                            })),
                        ),
                        Err(e) => (
                            StatusCode::NOT_FOUND,
                            Json(serde_json::json!({ "error": e.to_string() })),
                        ),
                    }
                }
                Err(e) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({ "error": e.to_string() })),
                ),
            }
        }
        None => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({ "error": "ADI not initialized" })),
        ),
    }
}
