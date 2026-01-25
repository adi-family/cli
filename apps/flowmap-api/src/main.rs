use axum::{
    Router,
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
};
use lib_flowmap_core::*;
use lib_flowmap_parser::{FlowParser, MultiLangParser};
use lib_http_common::version_header_layer;
use lib_logging_core::trace_layer;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use walkdir::WalkDir;

#[derive(Clone)]
struct AppState {
    indexes: Arc<RwLock<HashMap<String, FlowIndex>>>,
    block_indexes: Arc<RwLock<HashMap<String, FlowMapOutput>>>,
}

#[derive(Debug, Deserialize)]
struct ParseQuery {
    path: String,
}

#[derive(Debug, Serialize)]
struct ApiResponse<T> {
    success: bool,
    data: Option<T>,
    error: Option<String>,
}

impl<T> ApiResponse<T> {
    fn ok(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    fn err(msg: &str) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(msg.to_string()),
        }
    }
}

#[derive(Debug, Serialize)]
struct ParseResponse {
    root_path: String,
    flow_count: usize,
    flows: Vec<FlowSummary>,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "flowmap_api=info,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let state = AppState {
        indexes: Arc::new(RwLock::new(HashMap::new())),
        block_indexes: Arc::new(RwLock::new(HashMap::new())),
    };

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/", get(health))
        .route("/health", get(health))
        // Legacy v1 API (FlowGraph format)
        .route("/api/parse", get(parse_directory))
        .route("/api/flows", get(list_flows))
        .route("/api/flows/{id}", get(get_flow))
        .route("/api/flows/{id}/issues", get(get_flow_issues))
        .route("/api/source/{id}", get(get_source))
        // New v2 API (Block-based format)
        .route("/api/v2/parse", get(parse_blocks))
        .route("/api/v2/parse/file", post(parse_single_file))
        .route("/api/v2/blocks", get(get_blocks))
        .route("/api/v2/blocks/{id}", get(get_block))
        .route("/api/v2/languages", get(get_supported_languages))
        .layer(version_header_layer(
            env!("CARGO_PKG_NAME"),
            env!("CARGO_PKG_VERSION"),
        ))
        .layer(trace_layer())
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let port = std::env::var("PORT").unwrap_or_else(|_| "8092".to_string());
    let addr = format!("0.0.0.0:{}", port);

    tracing::info!("FlowMap API starting on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn health() -> &'static str {
    "FlowMap API OK"
}

async fn parse_directory(
    State(state): State<AppState>,
    Query(query): Query<ParseQuery>,
) -> Result<Json<ApiResponse<ParseResponse>>, StatusCode> {
    let path = PathBuf::from(&query.path);

    if !path.exists() {
        return Ok(Json(ApiResponse::err(&format!(
            "Path does not exist: {}",
            query.path
        ))));
    }

    if !path.is_dir() {
        return Ok(Json(ApiResponse::err(&format!(
            "Path is not a directory: {}",
            query.path
        ))));
    }

    let mut parser = FlowParser::new().map_err(|e| {
        tracing::error!("Failed to create parser: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let index = parser.parse_directory(&path).map_err(|e| {
        tracing::error!("Failed to parse directory: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let response = ParseResponse {
        root_path: index.root_path.clone(),
        flow_count: index.flows.len(),
        flows: index.summaries(),
    };

    // Store the index
    let key = query.path.clone();
    state.indexes.write().unwrap().insert(key, index);

    Ok(Json(ApiResponse::ok(response)))
}

async fn list_flows(
    State(state): State<AppState>,
    Query(query): Query<ParseQuery>,
) -> Json<ApiResponse<Vec<FlowSummary>>> {
    let indexes = state.indexes.read().unwrap();

    if let Some(index) = indexes.get(&query.path) {
        Json(ApiResponse::ok(index.summaries()))
    } else {
        Json(ApiResponse::err(
            "Path not parsed yet. Call /api/parse first.",
        ))
    }
}

async fn get_flow(
    State(state): State<AppState>,
    Path(id): Path<u64>,
    Query(query): Query<ParseQuery>,
) -> Json<ApiResponse<FlowGraph>> {
    let indexes = state.indexes.read().unwrap();

    if let Some(index) = indexes.get(&query.path) {
        if let Some(flow) = index.get_flow(FlowId(id)) {
            return Json(ApiResponse::ok(flow.clone()));
        }
        return Json(ApiResponse::err(&format!("Flow not found: {}", id)));
    }

    Json(ApiResponse::err(
        "Path not parsed yet. Call /api/parse first.",
    ))
}

async fn get_flow_issues(
    State(state): State<AppState>,
    Path(id): Path<u64>,
    Query(query): Query<ParseQuery>,
) -> Json<ApiResponse<Vec<FlowIssue>>> {
    let indexes = state.indexes.read().unwrap();

    if let Some(index) = indexes.get(&query.path) {
        if let Some(flow) = index.get_flow(FlowId(id)) {
            let issues = detect_issues(flow);
            return Json(ApiResponse::ok(issues));
        }
        return Json(ApiResponse::err(&format!("Flow not found: {}", id)));
    }

    Json(ApiResponse::err(
        "Path not parsed yet. Call /api/parse first.",
    ))
}

fn detect_issues(flow: &FlowGraph) -> Vec<FlowIssue> {
    let mut issues = Vec::new();

    // Check for unhandled error pins
    for node in flow.nodes.values() {
        for pin in &node.outputs {
            if pin.kind == PinKind::Error && !pin.connected {
                issues.push(FlowIssue {
                    kind: FlowIssueKind::UnhandledError,
                    node_id: node.id,
                    message: format!("Error from '{}' is not handled", node.label),
                    severity: IssueSeverity::Warning,
                });
            }
        }
    }

    issues
}

#[derive(Debug, Serialize)]
struct SourceResponse {
    file_path: String,
    start_line: u32,
    end_line: u32,
    content: Option<String>,
}

async fn get_source(
    State(state): State<AppState>,
    Path(node_id): Path<u64>,
    Query(query): Query<ParseQuery>,
) -> Json<ApiResponse<SourceResponse>> {
    let indexes = state.indexes.read().unwrap();

    if let Some(index) = indexes.get(&query.path) {
        for flow in &index.flows {
            if let Some(node) = flow.nodes.get(&NodeId(node_id)) {
                let full_path = PathBuf::from(&query.path).join(&node.location.file_path);
                let content = std::fs::read_to_string(&full_path).ok();

                return Json(ApiResponse::ok(SourceResponse {
                    file_path: node.location.file_path.clone(),
                    start_line: node.location.start_line,
                    end_line: node.location.end_line,
                    content,
                }));
            }
        }
        return Json(ApiResponse::err(&format!("Node not found: {}", node_id)));
    }

    Json(ApiResponse::err(
        "Path not parsed yet. Call /api/parse first.",
    ))
}

// ============================================================================
// V2 API - Block-based format
// ============================================================================

#[derive(Debug, Serialize)]
struct BlockParseResponse {
    root_path: String,
    block_count: usize,
    root_count: usize,
    language: Option<String>,
}

/// Parse a directory and return block-based output
async fn parse_blocks(
    State(state): State<AppState>,
    Query(query): Query<ParseQuery>,
) -> Result<Json<ApiResponse<BlockParseResponse>>, StatusCode> {
    let path = PathBuf::from(&query.path);

    if !path.exists() {
        return Ok(Json(ApiResponse::err(&format!(
            "Path does not exist: {}",
            query.path
        ))));
    }

    let mut parser = MultiLangParser::new().map_err(|e| {
        tracing::error!("Failed to create parser: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let mut combined_output = FlowMapOutput::new();

    // Walk the directory and parse all supported files
    for entry in WalkDir::new(&path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        let file_path = entry.path();
        let file_path_str = file_path.to_string_lossy().to_string();

        // Skip unsupported files
        if !MultiLangParser::is_supported(&file_path_str) {
            continue;
        }

        // Skip common non-source directories
        if file_path_str.contains("node_modules")
            || file_path_str.contains(".git")
            || file_path_str.contains("target")
            || file_path_str.contains("dist")
            || file_path_str.contains("build")
            || file_path_str.contains("__pycache__")
            || file_path_str.contains(".venv")
        {
            continue;
        }

        // Read and parse the file
        if let Ok(source) = std::fs::read_to_string(file_path) {
            // Use relative path for storage
            let rel_path = file_path
                .strip_prefix(&path)
                .unwrap_or(file_path)
                .to_string_lossy()
                .to_string();

            match parser.parse(&source, &rel_path) {
                Ok(output) => {
                    combined_output.merge(output);
                }
                Err(e) => {
                    tracing::warn!("Failed to parse {}: {}", rel_path, e);
                }
            }
        }
    }

    let response = BlockParseResponse {
        root_path: query.path.clone(),
        block_count: combined_output.block_count(),
        root_count: combined_output.root.len(),
        language: None, // Multi-language
    };

    // Store the index
    let key = query.path.clone();
    state
        .block_indexes
        .write()
        .unwrap()
        .insert(key, combined_output);

    Ok(Json(ApiResponse::ok(response)))
}

#[derive(Debug, Deserialize)]
struct SingleFileRequest {
    source: String,
    file_path: String,
}

/// Parse a single file from source code
async fn parse_single_file(
    Json(request): Json<SingleFileRequest>,
) -> Result<Json<ApiResponse<FlowMapOutput>>, StatusCode> {
    let mut parser = MultiLangParser::new().map_err(|e| {
        tracing::error!("Failed to create parser: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    match parser.parse(&request.source, &request.file_path) {
        Ok(output) => Ok(Json(ApiResponse::ok(output))),
        Err(e) => Ok(Json(ApiResponse::err(&e.to_string()))),
    }
}

/// Get the full block output for a parsed path
async fn get_blocks(
    State(state): State<AppState>,
    Query(query): Query<ParseQuery>,
) -> Json<ApiResponse<FlowMapOutput>> {
    let indexes = state.block_indexes.read().unwrap();

    if let Some(output) = indexes.get(&query.path) {
        Json(ApiResponse::ok(output.clone()))
    } else {
        Json(ApiResponse::err(
            "Path not parsed yet. Call /api/v2/parse first.",
        ))
    }
}

/// Get a specific block by ID
async fn get_block(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(query): Query<ParseQuery>,
) -> Json<ApiResponse<Block>> {
    let indexes = state.block_indexes.read().unwrap();

    if let Some(output) = indexes.get(&query.path) {
        let block_id = BlockId::new(id.clone());
        if let Some(block) = output.get_block(&block_id) {
            return Json(ApiResponse::ok(block.clone()));
        }
        return Json(ApiResponse::err(&format!("Block not found: {}", id)));
    }

    Json(ApiResponse::err(
        "Path not parsed yet. Call /api/v2/parse first.",
    ))
}

#[derive(Debug, Serialize)]
struct LanguagesResponse {
    languages: Vec<&'static str>,
    extensions: Vec<&'static str>,
}

/// Get list of supported languages and extensions
async fn get_supported_languages() -> Json<ApiResponse<LanguagesResponse>> {
    Json(ApiResponse::ok(LanguagesResponse {
        languages: vec![
            "typescript",
            "javascript",
            "tsx",
            "jsx",
            "python",
            "java",
            "rust",
        ],
        extensions: MultiLangParser::supported_extensions().to_vec(),
    }))
}
