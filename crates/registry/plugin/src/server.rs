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

struct AppState {
    storage: RegistryStorage,
    auth_token: Option<String>,
}

impl AppState {
    fn check_auth(&self, auth_header: Option<&str>) -> Result<(), ApiError> {
        let Some(expected) = &self.auth_token else {
            return Ok(());
        };
        let token = auth_header
            .and_then(|h| h.strip_prefix("Bearer "))
            .ok_or_else(unauthorized)?;
        if token != expected {
            return Err(unauthorized());
        }
        Ok(())
    }
}

fn data_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("REGISTRY_DATA_DIR") {
        return PathBuf::from(dir);
    }
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

fn unauthorized() -> ApiError {
    ApiError {
        status: 401,
        code: "unauthorized".to_string(),
        message: "Invalid or missing authorization token".to_string(),
    }
}

// ---------------------------------------------------------------------------
// Auth middleware
// ---------------------------------------------------------------------------

async fn check_publish_auth(
    State(state): State<Arc<AppState>>,
    req: axum::http::Request<Body>,
    next: axum::middleware::Next,
) -> Result<axum::response::Response, ApiError> {
    let auth_header = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());
    state.check_auth(auth_header.as_deref())?;
    Ok(next.run(req).await)
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

async fn get_index(State(st): State<Arc<AppState>>) -> Result<Json<serde_json::Value>, ApiError> {
    let index = st.storage.load_index().await.map_err(internal_error)?;
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
    State(st): State<Arc<AppState>>,
    Query(query): Query<SearchQuery>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let index = st.storage.load_index().await.map_err(internal_error)?;
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
    State(st): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let info = st
        .storage
        .get_plugin_latest(&id)
        .await
        .map_err(|_| not_found("Plugin not found"))?;
    serde_json::to_value(&info).map(Json).map_err(internal_error)
}

async fn plugin_version(
    State(st): State<Arc<AppState>>,
    Path((id, version)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let version = &version;
    let info = st
        .storage
        .get_plugin_info(&id, version)
        .await
        .map_err(|_| not_found("Plugin version not found"))?;
    serde_json::to_value(&info).map(Json).map_err(internal_error)
}

async fn plugin_download(
    State(st): State<Arc<AppState>>,
    Path((id, version, platform)): Path<(String, String, String)>,
) -> Result<axum::response::Response, ApiError> {
    serve_artifact(&st.storage, ArtifactKind::Plugin, &id, &version, &platform).await
}

async fn plugin_versions(
    State(st): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let versions = st
        .storage
        .list_artifact_versions(ArtifactKind::Plugin, &id)
        .await
        .map_err(internal_error)?;
    Ok(Json(serde_json::json!({ "id": id, "versions": versions })))
}

async fn plugin_web_ui(
    State(st): State<Arc<AppState>>,
    Path((id, version)): Path<(String, String)>,
) -> Result<axum::response::Response, ApiError> {
    let path = st.storage.get_plugin_web_ui_path(&id, &version);
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
    State(st): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let info = st
        .storage
        .get_package_latest(&id)
        .await
        .map_err(|_| not_found("Package not found"))?;
    serde_json::to_value(&info).map(Json).map_err(internal_error)
}

async fn package_version(
    State(st): State<Arc<AppState>>,
    Path((id, version)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let version = &version;
    let info = st
        .storage
        .get_package_info(&id, version)
        .await
        .map_err(|_| not_found("Package version not found"))?;
    serde_json::to_value(&info).map(Json).map_err(internal_error)
}

async fn package_download(
    State(st): State<Arc<AppState>>,
    Path((id, version, platform)): Path<(String, String, String)>,
) -> Result<axum::response::Response, ApiError> {
    serve_artifact(&st.storage, ArtifactKind::Package, &id, &version, &platform).await
}

async fn package_versions(
    State(st): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let versions = st
        .storage
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
    State(st): State<Arc<AppState>>,
    Path((id, version, platform)): Path<(String, String, String)>,
    Query(q): Query<PublishQuery>,
    headers: axum::http::HeaderMap,
    body: axum::body::Bytes,
) -> Result<(StatusCode, Json<serde_json::Value>), ApiError> {
    if body.is_empty() {
        return Err(bad_request("No file uploaded"));
    }

    let publisher_sig = headers
        .get("X-Publisher-Signature")
        .and_then(|v| v.to_str().ok())
        .map(String::from);
    let publisher_key = headers
        .get("X-Publisher-Public-Key")
        .and_then(|v| v.to_str().ok())
        .map(String::from);
    let publisher_id = headers
        .get("X-Publisher-Id")
        .and_then(|v| v.to_str().ok())
        .map(String::from);
    let publisher_cert = headers
        .get("X-Publisher-Certificate")
        .and_then(|v| v.to_str().ok())
        .map(String::from);

    let plugin_types: Vec<String> = q
        .plugin_type
        .as_deref()
        .unwrap_or("extension")
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(String::from)
        .collect();

    st.storage
        .publish_plugin(
            &id,
            &q.name,
            q.description.as_deref().unwrap_or(""),
            &plugin_types,
            &version,
            &platform,
            &body,
            q.author.as_deref().unwrap_or("unknown"),
            vec![],
            publisher_sig.as_deref(),
            publisher_key.as_deref(),
            publisher_id.as_deref(),
            publisher_cert.as_deref(),
        )
        .await
        .map_err(|e| {
            let msg = e.to_string();
            if msg.contains("signature") || msg.contains("certificate") || msg.contains("revoked") {
                bad_request(&msg)
            } else {
                internal_error(e)
            }
        })?;

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
    State(st): State<Arc<AppState>>,
    Path((id, version, platform)): Path<(String, String, String)>,
    Query(q): Query<PublishQuery>,
    headers: axum::http::HeaderMap,
    body: axum::body::Bytes,
) -> Result<(StatusCode, Json<serde_json::Value>), ApiError> {
    if body.is_empty() {
        return Err(bad_request("No file uploaded"));
    }

    let publisher_sig = headers
        .get("X-Publisher-Signature")
        .and_then(|v| v.to_str().ok())
        .map(String::from);
    let publisher_key = headers
        .get("X-Publisher-Public-Key")
        .and_then(|v| v.to_str().ok())
        .map(String::from);
    let publisher_id = headers
        .get("X-Publisher-Id")
        .and_then(|v| v.to_str().ok())
        .map(String::from);
    let publisher_cert = headers
        .get("X-Publisher-Certificate")
        .and_then(|v| v.to_str().ok())
        .map(String::from);

    st.storage
        .publish_package(
            &id,
            &q.name,
            q.description.as_deref().unwrap_or(""),
            &version,
            &platform,
            &body,
            q.author.as_deref().unwrap_or("unknown"),
            vec![],
            publisher_sig.as_deref(),
            publisher_key.as_deref(),
            publisher_id.as_deref(),
            publisher_cert.as_deref(),
        )
        .await
        .map_err(|e| {
            let msg = e.to_string();
            if msg.contains("signature") || msg.contains("certificate") || msg.contains("revoked") {
                bad_request(&msg)
            } else {
                internal_error(e)
            }
        })?;

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
    State(st): State<Arc<AppState>>,
    Path((id, version)): Path<(String, String)>,
    body: axum::body::Bytes,
) -> Result<(StatusCode, Json<serde_json::Value>), ApiError> {
    if body.is_empty() {
        return Err(bad_request("Empty body — expected JavaScript content"));
    }

    st.storage
        .publish_plugin_web_ui(&id, &version, &body)
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
// Handlers — publisher management
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct RegisterPublisherBody {
    publisher_id: String,
    public_key: String,
}

async fn register_publisher(
    State(st): State<Arc<AppState>>,
    Json(body): Json<RegisterPublisherBody>,
) -> Result<(StatusCode, Json<serde_json::Value>), ApiError> {
    let cert = st
        .storage
        .register_publisher(&body.publisher_id, &body.public_key)
        .await
        .map_err(|e| bad_request(&e.to_string()))?;
    serde_json::to_value(&cert)
        .map(|v| (StatusCode::CREATED, Json(v)))
        .map_err(internal_error)
}

async fn list_publishers(
    State(st): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let publishers = st.storage.list_publishers().await.map_err(internal_error)?;
    serde_json::to_value(&serde_json::json!({ "publishers": publishers }))
        .map(Json)
        .map_err(internal_error)
}

async fn revoke_publisher(
    State(st): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    st.storage
        .revoke_publisher(&id)
        .await
        .map_err(|e| bad_request(&e.to_string()))?;
    Ok(Json(serde_json::json!({ "status": "revoked", "publisher_id": id })))
}

// ---------------------------------------------------------------------------
// Handlers — registry public key
// ---------------------------------------------------------------------------

async fn registry_public_key(
    State(st): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let public_key = st.storage.load_public_key().await.map_err(internal_error)?;
    Ok(Json(serde_json::json!({
        "public_key": public_key,
        "algorithm": "Ed25519",
        "encoding": "base64",
    })))
}

// ---------------------------------------------------------------------------
// Router & entry point
// ---------------------------------------------------------------------------

fn build_router(state: Arc<AppState>) -> Router {
    let read_routes = Router::new()
        .route("/health", get(health))
        .route("/v1/index", get(get_index))
        .route("/v1/search", get(search))
        .route("/v1/plugins/:id/latest", get(plugin_latest))
        .route("/v1/plugins/:id/:version", get(plugin_version))
        .route(
            "/v1/plugins/:id/:version/{platform}.tar.gz",
            get(plugin_download),
        )
        .route("/v1/plugins/:id/versions", get(plugin_versions))
        .route("/v1/plugins/:id/:version/web.js", get(plugin_web_ui))
        .route("/v1/packages/:id/latest", get(package_latest))
        .route("/v1/packages/:id/:version", get(package_version))
        .route(
            "/v1/packages/:id/:version/{platform}.tar.gz",
            get(package_download),
        )
        .route("/v1/packages/:id/versions", get(package_versions))
        .route("/v1/registry/public-key", get(registry_public_key))
        .route("/v1/publishers", get(list_publishers));

    let write_routes = Router::new()
        .route("/v1/publishers/register", post(register_publisher))
        .route("/v1/publishers/:id/revoke", post(revoke_publisher))
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
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            check_publish_auth,
        ));

    Router::new()
        .merge(read_routes)
        .merge(write_routes)
        .layer(axum::extract::DefaultBodyLimit::max(100 * 1024 * 1024))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

/// Runs the registry server with its own Tokio runtime.
///
/// cdylib plugins link their own copy of tokio, so the host process's
/// runtime handle is invisible here. We must create a dedicated runtime.
pub fn run_server(port: u16) -> anyhow::Result<()> {
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        let root = data_dir();
        tracing::info!(dir = %root.display(), "Registry data directory");

        let storage = RegistryStorage::new(root);
        storage.init().await?;

        let auth_token = std::env::var("REGISTRY_AUTH_TOKEN").ok().filter(|s| !s.is_empty());
        if auth_token.is_some() {
            tracing::info!("Auth token configured — publish endpoints require authorization");
        }

        let state = Arc::new(AppState { storage, auth_token });
        let app = build_router(state);

        let addr = SocketAddr::from(([0, 0, 0, 0], port));
        tracing::info!("Registry server listening on http://{addr}");
        println!("Registry server listening on http://{addr}");

        let listener = tokio::net::TcpListener::bind(addr).await?;
        axum::serve(listener, app).await?;

        Ok(())
    })
}
