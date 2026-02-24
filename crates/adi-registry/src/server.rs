use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::{header, StatusCode},
    routing::{get, post},
    Json, Router,
};
use plugin_registry_core::{ArtifactKind, RegistryStorage};
use serde::Deserialize;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs::File;
use tokio_util::io::ReaderStream;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

type AppState = Arc<RegistryStorage>;

fn data_dir() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("~/.local/share"))
        .join("adi")
        .join("registry")
}

// ---------------------------------------------------------------------------
// Error helpers
// ---------------------------------------------------------------------------

#[derive(Debug, serde::Serialize)]
struct ApiError {
    status: u16,
    code: String,
    message: String,
}

impl axum::response::IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        let status = StatusCode::from_u16(self.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        (status, Json(self)).into_response()
    }
}

fn internal_error(e: impl std::fmt::Display) -> ApiError {
    ApiError {
        status: 500,
        code: "internal_error".to_string(),
        message: e.to_string(),
    }
}

fn not_found(msg: &str) -> ApiError {
    ApiError {
        status: 404,
        code: "not_found".to_string(),
        message: msg.to_string(),
    }
}

fn bad_request(msg: &str) -> ApiError {
    ApiError {
        status: 400,
        code: "bad_request".to_string(),
        message: msg.to_string(),
    }
}

// ---------------------------------------------------------------------------
// File streaming helpers
// ---------------------------------------------------------------------------

async fn serve_file(path: PathBuf, content_type: &str) -> Result<axum::response::Response, ApiError> {
    let file = File::open(&path).await.map_err(internal_error)?;
    let stream = ReaderStream::new(file);
    let body = Body::from_stream(stream);

    let filename = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("download");

    axum::response::Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, content_type)
        .header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{filename}\""),
        )
        .body(body)
        .map_err(internal_error)
}

async fn serve_artifact(
    storage: &RegistryStorage,
    kind: ArtifactKind,
    id: &str,
    version: &str,
    platform: &str,
) -> Result<axum::response::Response, ApiError> {
    let platform = platform.trim_end_matches(".tar.gz");
    let path = storage.artifact_path(kind, id, version, platform);

    if !path.exists() {
        return Err(not_found("Artifact not found"));
    }

    let root = storage.root().to_path_buf();
    let id_owned = id.to_string();
    tokio::spawn(async move {
        let s = RegistryStorage::new(root);
        let _ = s.increment_downloads(kind, &id_owned).await;
    });

    serve_file(path, "application/gzip").await
}

fn matches_query(id: &str, name: &str, description: &str, tags: &[String], q: &str) -> bool {
    let q = q.to_lowercase();
    id.to_lowercase().contains(&q)
        || name.to_lowercase().contains(&q)
        || description.to_lowercase().contains(&q)
        || tags.iter().any(|t| t.to_lowercase().contains(&q))
}

// ---------------------------------------------------------------------------
// Handlers — health / index / search
// ---------------------------------------------------------------------------

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ok",
        "service": "adi-registry-plugin",
        "version": env!("CARGO_PKG_VERSION")
    }))
}

async fn get_index(State(st): State<AppState>) -> Result<Json<serde_json::Value>, ApiError> {
    let index = st.load_index().await.map_err(internal_error)?;
    serde_json::to_value(&index)
        .map(Json)
        .map_err(internal_error)
}

#[derive(Deserialize)]
struct SearchQuery {
    q: String,
    kind: Option<String>,
}

async fn search(
    State(st): State<AppState>,
    Query(query): Query<SearchQuery>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let index = st.load_index().await.map_err(internal_error)?;
    let kind = query.kind.as_deref().unwrap_or("all");

    let packages: Vec<_> = if kind == "all" || kind == "package" {
        index
            .packages
            .iter()
            .filter(|p| matches_query(&p.id, &p.name, &p.description, &p.tags, &query.q))
            .collect()
    } else {
        vec![]
    };

    let plugins: Vec<_> = if kind == "all" || kind == "plugin" {
        index
            .plugins
            .iter()
            .filter(|p| matches_query(&p.id, &p.name, &p.description, &p.tags, &query.q))
            .collect()
    } else {
        vec![]
    };

    serde_json::to_value(&serde_json::json!({ "packages": packages, "plugins": plugins }))
        .map(Json)
        .map_err(internal_error)
}

// ---------------------------------------------------------------------------
// Handlers — plugins
// ---------------------------------------------------------------------------

async fn plugin_latest(
    State(st): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let info = st
        .get_plugin_latest(&id)
        .await
        .map_err(|_| not_found("Plugin not found"))?;
    serde_json::to_value(&info).map(Json).map_err(internal_error)
}

async fn plugin_version(
    State(st): State<AppState>,
    Path((id, version)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let version = &version;
    let info = st
        .get_plugin_info(&id, version)
        .await
        .map_err(|_| not_found("Plugin version not found"))?;
    serde_json::to_value(&info).map(Json).map_err(internal_error)
}

async fn plugin_download(
    State(st): State<AppState>,
    Path((id, version, platform)): Path<(String, String, String)>,
) -> Result<axum::response::Response, ApiError> {
    serve_artifact(&st, ArtifactKind::Plugin, &id, &version, &platform).await
}

async fn plugin_versions(
    State(st): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let versions = st
        .list_artifact_versions(ArtifactKind::Plugin, &id)
        .await
        .map_err(internal_error)?;
    Ok(Json(serde_json::json!({ "id": id, "versions": versions })))
}

async fn plugin_web_ui(
    State(st): State<AppState>,
    Path((id, version)): Path<(String, String)>,
) -> Result<axum::response::Response, ApiError> {
    let path = st.get_plugin_web_ui_path(&id, &version);
    if !path.exists() {
        return Err(not_found("Plugin web UI not found"));
    }

    let file = File::open(&path).await.map_err(internal_error)?;
    let stream = ReaderStream::new(file);
    let body = Body::from_stream(stream);

    axum::response::Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/javascript")
        .header(header::CACHE_CONTROL, "public, max-age=31536000, immutable")
        .header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
        .body(body)
        .map_err(internal_error)
}

// ---------------------------------------------------------------------------
// Handlers — packages
// ---------------------------------------------------------------------------

async fn package_latest(
    State(st): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let info = st
        .get_package_latest(&id)
        .await
        .map_err(|_| not_found("Package not found"))?;
    serde_json::to_value(&info).map(Json).map_err(internal_error)
}

async fn package_version(
    State(st): State<AppState>,
    Path((id, version)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let version = &version;
    let info = st
        .get_package_info(&id, version)
        .await
        .map_err(|_| not_found("Package version not found"))?;
    serde_json::to_value(&info).map(Json).map_err(internal_error)
}

async fn package_download(
    State(st): State<AppState>,
    Path((id, version, platform)): Path<(String, String, String)>,
) -> Result<axum::response::Response, ApiError> {
    serve_artifact(&st, ArtifactKind::Package, &id, &version, &platform).await
}

async fn package_versions(
    State(st): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let versions = st
        .list_artifact_versions(ArtifactKind::Package, &id)
        .await
        .map_err(internal_error)?;
    Ok(Json(serde_json::json!({ "id": id, "versions": versions })))
}

// ---------------------------------------------------------------------------
// Handlers — publish
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct PublishQuery {
    name: String,
    description: Option<String>,
    plugin_type: Option<String>,
    author: Option<String>,
}

async fn publish_plugin(
    State(st): State<AppState>,
    Path((id, version, platform)): Path<(String, String, String)>,
    Query(q): Query<PublishQuery>,
    body: axum::body::Bytes,
) -> Result<(StatusCode, Json<serde_json::Value>), ApiError> {
    if body.is_empty() {
        return Err(bad_request("No file uploaded"));
    }

    let plugin_types: Vec<String> = q
        .plugin_type
        .as_deref()
        .unwrap_or("extension")
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(String::from)
        .collect();

    st.publish_plugin(
        &id,
        &q.name,
        q.description.as_deref().unwrap_or(""),
        &plugin_types,
        &version,
        &platform,
        &body,
        q.author.as_deref().unwrap_or("unknown"),
        vec![],
    )
    .await
    .map_err(internal_error)?;

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({
            "status": "published",
            "id": id,
            "version": version,
            "platform": platform,
        })),
    ))
}

async fn publish_package(
    State(st): State<AppState>,
    Path((id, version, platform)): Path<(String, String, String)>,
    Query(q): Query<PublishQuery>,
    body: axum::body::Bytes,
) -> Result<(StatusCode, Json<serde_json::Value>), ApiError> {
    if body.is_empty() {
        return Err(bad_request("No file uploaded"));
    }

    st.publish_package(
        &id,
        &q.name,
        q.description.as_deref().unwrap_or(""),
        &version,
        &platform,
        &body,
        q.author.as_deref().unwrap_or("unknown"),
        vec![],
    )
    .await
    .map_err(internal_error)?;

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({
            "status": "published",
            "id": id,
            "version": version,
            "platform": platform,
        })),
    ))
}

async fn publish_plugin_web_ui(
    State(st): State<AppState>,
    Path((id, version)): Path<(String, String)>,
    body: axum::body::Bytes,
) -> Result<(StatusCode, Json<serde_json::Value>), ApiError> {
    if body.is_empty() {
        return Err(bad_request("Empty body — expected JavaScript content"));
    }

    st.publish_plugin_web_ui(&id, &version, &body)
        .await
        .map_err(internal_error)?;

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({
            "status": "published",
            "id": id,
            "version": version,
            "platform": "web",
        })),
    ))
}

// ---------------------------------------------------------------------------
// Router & entry point
// ---------------------------------------------------------------------------

fn build_router(state: AppState) -> Router {
    Router::new()
        // Health
        .route("/health", get(health))
        // Index & search
        .route("/v1/index", get(get_index))
        .route("/v1/search", get(search))
        // Plugins
        .route("/v1/plugins/:id/latest", get(plugin_latest))
        .route("/v1/plugins/:id/:version", get(plugin_version))
        .route(
            "/v1/plugins/:id/:version/{platform}.tar.gz",
            get(plugin_download),
        )
        .route("/v1/plugins/:id/versions", get(plugin_versions))
        .route("/v1/plugins/:id/:version/web.js", get(plugin_web_ui))
        // Packages
        .route("/v1/packages/:id/latest", get(package_latest))
        .route("/v1/packages/:id/:version", get(package_version))
        .route(
            "/v1/packages/:id/:version/{platform}.tar.gz",
            get(package_download),
        )
        .route("/v1/packages/:id/versions", get(package_versions))
        // Publish
        .route(
            "/v1/publish/plugins/:id/:version/:platform",
            post(publish_plugin),
        )
        .route(
            "/v1/publish/packages/:id/:version/:platform",
            post(publish_package),
        )
        .route(
            "/v1/publish/plugins/:id/:version/web",
            post(publish_plugin_web_ui),
        )
        // Limits & middleware
        .layer(axum::extract::DefaultBodyLimit::max(100 * 1024 * 1024))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

pub async fn run_server(port: u16) -> anyhow::Result<()> {
    let root = data_dir();
    tracing::info!(dir = %root.display(), "Registry data directory");

    let storage = RegistryStorage::new(root);
    storage.init().await?;

    let state = Arc::new(storage);
    let app = build_router(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    tracing::info!("Registry server listening on http://{addr}");
    println!("Registry server listening on http://{addr}");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
