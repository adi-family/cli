mod generated;

use anyhow::Result;
use async_trait::async_trait;
use axum::{routing::get, Json, Router};
use generated::enums::{SymbolKind, Visibility};
use generated::models::*;
use generated::server::*;
use lib_http_common::version_header_layer;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing::info;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};
use lib_env_parse::{env_vars, env_opt};

env_vars! {
    Port => "PORT",
}

struct AppState {
    adi: RwLock<Option<indexer_core::Adi>>,
    project_path: PathBuf,
}

fn internal_error(e: impl std::fmt::Display) -> ApiError {
    ApiError {
        status: 500,
        code: "internal_error".to_string(),
        message: e.to_string(),
    }
}

fn unavailable(msg: &str) -> ApiError {
    ApiError {
        status: 503,
        code: "unavailable".to_string(),
        message: msg.to_string(),
    }
}

fn not_found(e: impl std::fmt::Display) -> ApiError {
    ApiError {
        status: 404,
        code: "not_found".to_string(),
        message: e.to_string(),
    }
}

fn convert_symbol_kind(k: indexer_core::SymbolKind) -> SymbolKind {
    match k {
        indexer_core::SymbolKind::Function => SymbolKind::Function,
        indexer_core::SymbolKind::Method => SymbolKind::Method,
        indexer_core::SymbolKind::Class => SymbolKind::Class,
        indexer_core::SymbolKind::Struct => SymbolKind::Struct,
        indexer_core::SymbolKind::Enum => SymbolKind::Enum,
        indexer_core::SymbolKind::Interface => SymbolKind::Interface,
        indexer_core::SymbolKind::Trait => SymbolKind::Trait,
        indexer_core::SymbolKind::Module => SymbolKind::Module,
        indexer_core::SymbolKind::Constant => SymbolKind::Constant,
        indexer_core::SymbolKind::Variable => SymbolKind::Variable,
        indexer_core::SymbolKind::Type => SymbolKind::Type,
        indexer_core::SymbolKind::Property => SymbolKind::Property,
        indexer_core::SymbolKind::Field => SymbolKind::Field,
        indexer_core::SymbolKind::Constructor => SymbolKind::Constructor,
        indexer_core::SymbolKind::Destructor => SymbolKind::Destructor,
        indexer_core::SymbolKind::Operator => SymbolKind::Operator,
        indexer_core::SymbolKind::Macro => SymbolKind::Macro,
        indexer_core::SymbolKind::Namespace => SymbolKind::Namespace,
        indexer_core::SymbolKind::Package => SymbolKind::Package,
        _ => SymbolKind::Unknown,
    }
}

fn convert_visibility(v: indexer_core::Visibility) -> Visibility {
    match v {
        indexer_core::Visibility::Public => Visibility::Public,
        indexer_core::Visibility::PublicCrate => Visibility::PublicCrate,
        indexer_core::Visibility::PublicSuper => Visibility::PublicSuper,
        indexer_core::Visibility::Protected => Visibility::Protected,
        indexer_core::Visibility::Private => Visibility::Private,
        indexer_core::Visibility::Internal => Visibility::Internal,
        _ => Visibility::Unknown,
    }
}

fn convert_symbol(s: &indexer_core::Symbol) -> Symbol {
    Symbol {
        id: s.id.0,
        name: s.name.clone(),
        kind: convert_symbol_kind(s.kind),
        file_id: s.file_id.0,
        file_path: s.file_path.display().to_string(),
        location: Location {
            start_line: s.location.start_line,
            start_col: s.location.start_col,
            end_line: s.location.end_line,
            end_col: s.location.end_col,
            start_byte: s.location.start_byte,
            end_byte: s.location.end_byte,
        },
        parent_id: s.parent_id.map(|id| id.0),
        signature: s.signature.clone(),
        description: s.description.clone(),
        doc_comment: s.doc_comment.clone(),
        visibility: convert_visibility(s.visibility),
        is_entry_point: s.is_entry_point,
    }
}

fn convert_search_result(r: &indexer_core::SearchResult) -> SearchResult {
    SearchResult {
        symbol: convert_symbol(&r.symbol),
        score: r.score,
        context: r.context.clone(),
    }
}

#[async_trait]
impl IndexerServiceHandler for AppState {
    async fn get_status(&self) -> Result<Status, ApiError> {
        let adi = self.adi.read().await;
        let adi = adi.as_ref().ok_or_else(|| unavailable("ADI not initialized"))?;
        let status = adi.status().map_err(internal_error)?;
        // Convert via serde since Status fields match
        let v = serde_json::to_value(status).map_err(internal_error)?;
        serde_json::from_value(v).map_err(internal_error)
    }

    async fn index_project(&self) -> Result<IndexProgress, ApiError> {
        #[allow(deprecated)] // standalone HTTP binary uses fastembed, not plugin manager
        let adi = match indexer_core::Adi::open(&self.project_path).await {
            Ok(adi) => adi,
            Err(e) => return Err(internal_error(e)),
        };

        let progress = adi.index().await.map_err(internal_error)?;
        *self.adi.write().await = Some(adi);

        Ok(IndexProgress {
            files_processed: progress.files_processed,
            files_total: progress.files_total,
            symbols_indexed: progress.symbols_indexed,
            errors: progress.errors,
        })
    }

    async fn search(&self, query: IndexerServiceSearchQuery) -> Result<Vec<SearchResult>, ApiError> {
        let adi = self.adi.read().await;
        let adi = adi.as_ref().ok_or_else(|| unavailable("ADI not initialized"))?;
        let limit = query.limit.map(|l| l as usize).unwrap_or(10);
        let results = adi.search(&query.q, limit).await.map_err(internal_error)?;
        Ok(results.iter().map(convert_search_result).collect())
    }

    async fn search_symbols(&self, query: IndexerServiceSearchSymbolsQuery) -> Result<Vec<SearchResult>, ApiError> {
        let adi = self.adi.read().await;
        let adi = adi.as_ref().ok_or_else(|| unavailable("ADI not initialized"))?;
        let limit = query.limit.map(|l| l as usize).unwrap_or(10);
        let symbols = adi.search_symbols(&query.q, limit).await.map_err(internal_error)?;
        Ok(symbols
            .iter()
            .map(|s| SearchResult {
                symbol: convert_symbol(s),
                score: 1.0,
                context: None,
            })
            .collect())
    }

    async fn get_symbol(&self, id: i64) -> Result<Symbol, ApiError> {
        let adi = self.adi.read().await;
        let adi = adi.as_ref().ok_or_else(|| unavailable("ADI not initialized"))?;
        let symbol = adi.get_symbol(indexer_core::SymbolId(id)).map_err(not_found)?;
        Ok(convert_symbol(&symbol))
    }

    async fn get_symbol_reachability(&self, id: i64) -> Result<ReachabilityResponse, ApiError> {
        use indexer_core::analyzer::{EntryPointDetector, ReachabilityAnalyzer};

        let adi = self.adi.read().await;
        let adi = adi.as_ref().ok_or_else(|| unavailable("ADI not initialized"))?;

        let storage = indexer_core::SqliteStorage::open(
            &self.project_path.join(".adi/tree/index.sqlite"),
        )
        .map_err(internal_error)?;
        let storage_arc = Arc::new(storage);

        let entry_detector = EntryPointDetector::new(storage_arc.clone());
        let entry_points = entry_detector.detect_entry_points().map_err(internal_error)?;

        let reachability_analyzer = ReachabilityAnalyzer::new(storage_arc);
        let is_reachable = reachability_analyzer
            .is_reachable(indexer_core::SymbolId(id), &entry_points)
            .map_err(internal_error)?;

        let symbol = adi.get_symbol(indexer_core::SymbolId(id)).map_err(not_found)?;

        Ok(ReachabilityResponse {
            symbol_id: id,
            symbol_name: symbol.name,
            is_reachable,
            status: if is_reachable {
                "reachable".to_string()
            } else {
                "dead_code".to_string()
            },
        })
    }

    async fn search_files(&self, query: IndexerServiceSearchFilesQuery) -> Result<Vec<File>, ApiError> {
        let adi = self.adi.read().await;
        let adi = adi.as_ref().ok_or_else(|| unavailable("ADI not initialized"))?;
        let limit = query.limit.map(|l| l as usize).unwrap_or(10);
        let results = adi.search_files(&query.q, limit).await.map_err(internal_error)?;
        let v = serde_json::to_value(results).map_err(internal_error)?;
        serde_json::from_value(v).map_err(internal_error)
    }

    async fn get_file(&self, path: String) -> Result<FileInfo, ApiError> {
        let adi = self.adi.read().await;
        let adi = adi.as_ref().ok_or_else(|| unavailable("ADI not initialized"))?;
        let file_info = adi.get_file(std::path::Path::new(&path)).map_err(not_found)?;
        let v = serde_json::to_value(file_info).map_err(internal_error)?;
        serde_json::from_value(v).map_err(internal_error)
    }

    async fn get_tree(&self) -> Result<Tree, ApiError> {
        let adi = self.adi.read().await;
        let adi = adi.as_ref().ok_or_else(|| unavailable("ADI not initialized"))?;
        let tree = adi.get_tree().map_err(internal_error)?;
        let v = serde_json::to_value(tree).map_err(internal_error)?;
        serde_json::from_value(v).map_err(internal_error)
    }

    async fn find_dead_code(&self, query: IndexerServiceFindDeadCodeQuery) -> Result<DeadCodeReport, ApiError> {
        use indexer_core::analyzer::{AnalysisConfig, AnalysisMode, DeadCodeAnalyzer};

        let mode = match query.mode.as_deref() {
            Some("library") => AnalysisMode::Library,
            Some("application") => AnalysisMode::Application,
            _ => AnalysisMode::Strict,
        };

        let config = AnalysisConfig {
            mode,
            exclude_tests: query.exclude_tests.unwrap_or(true),
            exclude_traits: query.exclude_traits.unwrap_or(true),
            exclude_ffi: query.exclude_ffi.unwrap_or(true),
            exclude_patterns: vec![],
        };

        let storage = indexer_core::SqliteStorage::open(
            &self.project_path.join(".adi/tree/index.sqlite"),
        )
        .map_err(internal_error)?;

        let analyzer = DeadCodeAnalyzer::new(Arc::new(storage), config);
        let report = analyzer.analyze().map_err(internal_error)?;
        let v = serde_json::to_value(report).map_err(internal_error)?;
        serde_json::from_value(v).map_err(internal_error)
    }
}

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ok",
        "service": "adi-http",
        "version": env!("CARGO_PKG_VERSION")
    }))
}

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let project_path = if args.len() > 1 {
        PathBuf::from(&args[1])
    } else {
        std::env::current_dir()?
    };

    let port: u16 = env_opt(EnvVar::Port.as_str())
        .and_then(|p| p.parse().ok())
        .unwrap_or(3000);

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(filter)
        .init();

    info!("Starting ADI HTTP server");
    info!("Project path: {}", project_path.display());

    #[allow(deprecated)] // standalone HTTP binary uses fastembed, not plugin manager
    let adi = match indexer_core::Adi::open(&project_path).await {
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
        .merge(generated::server::create_router::<AppState>())
        .layer(version_header_layer(
            env!("CARGO_PKG_NAME"),
            env!("CARGO_PKG_VERSION"),
        ))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], port));
    info!("Listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
