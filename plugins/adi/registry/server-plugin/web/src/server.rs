use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::{header, StatusCode},
    routing::{get, post},
    Json, Router,
};
use adi_registry_core_web::WebRegistryStorage;
use serde::Deserialize;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs::File;
use tokio_util::io::ReaderStream;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

struct AppState {
    storage: WebRegistryStorage,
    auth_token: Option<String>,
}

impl AppState {
    fn check_auth(&self, token: Option<&str>) -> Result<(), ApiError> {
        let Some(expected) = &self.auth_token else { return Ok(()); };
        let token = token.ok_or_else(unauthorized)?;
        if token != expected { return Err(unauthorized()); }
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
        .join("web-registry")
}

#[derive(Debug, serde::Serialize)]
struct ApiError { status: u16, code: String, message: String }

impl axum::response::IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        let status = StatusCode::from_u16(self.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        (status, Json(self)).into_response()
    }
}

fn internal_error(e: impl std::fmt::Display) -> ApiError {
    ApiError { status: 500, code: "internal_error".into(), message: e.to_string() }
}
fn not_found(msg: &str) -> ApiError {
    ApiError { status: 404, code: "not_found".into(), message: msg.into() }
}
fn bad_request(msg: &str) -> ApiError {
    ApiError { status: 400, code: "bad_request".into(), message: msg.into() }
}
fn unauthorized() -> ApiError {
    ApiError { status: 401, code: "unauthorized".into(), message: "Invalid or missing authorization token".into() }
}

async fn check_publish_auth(
    State(state): State<Arc<AppState>>,
    req: axum::http::Request<Body>,
    next: axum::middleware::Next,
) -> Result<axum::response::Response, ApiError> {
    let token = req.headers().get("X-Registry-Token").and_then(|v| v.to_str().ok());
    state.check_auth(token)?;
    Ok(next.run(req).await)
}

async fn serve_static(path: PathBuf, content_type: &str) -> Result<axum::response::Response, ApiError> {
    let file = File::open(&path).await.map_err(internal_error)?;
    let body = Body::from_stream(ReaderStream::new(file));
    axum::response::Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, content_type)
        .header(header::CACHE_CONTROL, "public, max-age=31536000, immutable")
        .header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
        .body(body)
        .map_err(internal_error)
}

fn matches_query(id: &str, name: &str, description: &str, tags: &[String], q: &str) -> bool {
    let q = q.to_lowercase();
    id.to_lowercase().contains(&q)
        || name.to_lowercase().contains(&q)
        || description.to_lowercase().contains(&q)
        || tags.iter().any(|t| t.to_lowercase().contains(&q))
}

// --- Handlers ---

async fn root() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "plugin": "cli.adi.web-registry-server", "version": env!("CARGO_PKG_VERSION") }))
}

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "status": "ok", "service": "adi-web-registry-server", "version": env!("CARGO_PKG_VERSION") }))
}

async fn get_index(State(st): State<Arc<AppState>>) -> Result<Json<serde_json::Value>, ApiError> {
    let index = st.storage.load_index().await.map_err(internal_error)?;
    serde_json::to_value(&index).map(Json).map_err(internal_error)
}

#[derive(Deserialize)]
struct SearchQuery { q: String }

async fn search(
    State(st): State<Arc<AppState>>,
    Query(query): Query<SearchQuery>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let index = st.storage.load_index().await.map_err(internal_error)?;
    let plugins: Vec<_> = index.plugins.iter()
        .filter(|p| matches_query(&p.id, &p.name, &p.description, &p.tags, &query.q))
        .collect();
    serde_json::to_value(&serde_json::json!({ "plugins": plugins })).map(Json).map_err(internal_error)
}

async fn plugin_latest(
    State(st): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let info = st.storage.get_plugin_latest(&id).await.map_err(|_| not_found("Plugin not found"))?;
    serde_json::to_value(&info).map(Json).map_err(internal_error)
}

async fn plugin_version(
    State(st): State<Arc<AppState>>,
    Path((id, version)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let info = st.storage.get_plugin_info(&id, &version).await.map_err(|_| not_found("Plugin version not found"))?;
    serde_json::to_value(&info).map(Json).map_err(internal_error)
}

async fn download_js(
    State(st): State<Arc<AppState>>,
    Path((id, version)): Path<(String, String)>,
) -> Result<axum::response::Response, ApiError> {
    let path = st.storage.js_path(&id, &version);
    if !path.exists() { return Err(not_found("JS bundle not found")); }

    let root = st.storage.inner().root().to_path_buf();
    let id_clone = id.clone();
    tokio::spawn(async move {
        let s = WebRegistryStorage::new(root);
        let _ = s.increment_downloads(&id_clone).await;
    });

    serve_static(path, "application/javascript").await
}

async fn download_css(
    State(st): State<Arc<AppState>>,
    Path((id, version)): Path<(String, String)>,
) -> Result<axum::response::Response, ApiError> {
    let path = st.storage.css_path(&id, &version);
    if !path.exists() { return Err(not_found("CSS bundle not found")); }
    serve_static(path, "text/css").await
}

async fn preview_page(
    State(st): State<Arc<AppState>>,
    Path((id, version)): Path<(String, String)>,
) -> Result<axum::response::Response, ApiError> {
    let path = st.storage.preview_html_path(&id, &version);
    if !path.exists() { return Err(not_found("Preview page not found")); }
    serve_static(path, "text/html").await
}

async fn preview_image(
    State(st): State<Arc<AppState>>,
    Path((id, version, index)): Path<(String, String, String)>,
) -> Result<axum::response::Response, ApiError> {
    let idx: u8 = index.parse().map_err(|_| bad_request("Invalid preview index"))?;
    let path = st.storage.preview_image_path(&id, &version, idx);
    if !path.exists() { return Err(not_found("Preview image not found")); }
    serve_static(path, "image/webp").await
}

async fn publish_web_plugin(
    State(st): State<Arc<AppState>>,
    Path((id, version)): Path<(String, String)>,
    body: axum::body::Bytes,
) -> Result<(StatusCode, Json<serde_json::Value>), ApiError> {
    if body.is_empty() {
        return Err(bad_request("Empty body — expected .tar.gz archive"));
    }

    st.storage
        .publish_web_plugin(&id, &version, &body)
        .await
        .map_err(|e| {
            let msg = e.to_string();
            if msg.contains("manifest") || msg.contains("main.js") {
                bad_request(&msg)
            } else {
                internal_error(e)
            }
        })?;

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "status": "published", "id": id, "version": version })),
    ))
}

// --- Publisher management (shared pattern) ---

#[derive(Deserialize)]
struct RegisterPublisherBody { publisher_id: String, public_key: String }

async fn register_publisher(
    State(st): State<Arc<AppState>>,
    Json(body): Json<RegisterPublisherBody>,
) -> Result<(StatusCode, Json<serde_json::Value>), ApiError> {
    let storage = st.storage.inner();
    let cert = storage.publishers().register(storage.keypair(), &body.publisher_id, &body.public_key)
        .await.map_err(|e| bad_request(&e.to_string()))?;
    serde_json::to_value(&cert).map(|v| (StatusCode::CREATED, Json(v))).map_err(internal_error)
}

async fn list_publishers(State(st): State<Arc<AppState>>) -> Result<Json<serde_json::Value>, ApiError> {
    let publishers = st.storage.inner().publishers().list_active().await.map_err(internal_error)?;
    serde_json::to_value(&serde_json::json!({ "publishers": publishers })).map(Json).map_err(internal_error)
}

async fn revoke_publisher(State(st): State<Arc<AppState>>, Path(id): Path<String>) -> Result<Json<serde_json::Value>, ApiError> {
    st.storage.inner().publishers().revoke(&id).await.map_err(|e| bad_request(&e.to_string()))?;
    Ok(Json(serde_json::json!({ "status": "revoked", "publisher_id": id })))
}

async fn registry_public_key(State(st): State<Arc<AppState>>) -> Result<Json<serde_json::Value>, ApiError> {
    let public_key = st.storage.inner().keypair().load_public_key().await.map_err(internal_error)?;
    Ok(Json(serde_json::json!({ "public_key": public_key, "algorithm": "Ed25519", "encoding": "base64" })))
}

// --- Router ---

fn build_router(state: Arc<AppState>) -> Router {
    let read_routes = Router::new()
        .route("/", get(root))
        .route("/health", get(health))
        .route("/v1/index.json", get(get_index))
        .route("/v1/search", get(search))
        .route("/v1/:id/latest.json", get(plugin_latest))
        .route("/v1/:id/:version.json", get(plugin_version))
        .route("/v1/:id/:version/main.js", get(download_js))
        .route("/v1/:id/:version/main.css", get(download_css))
        .route("/v1/:id/:version/preview.html", get(preview_page))
        .route("/v1/:id/:version/preview/:index", get(preview_image))
        .route("/v1/registry/public-key", get(registry_public_key))
        .route("/v1/publishers", get(list_publishers));

    let write_routes = Router::new()
        .route("/v1/publishers/register", post(register_publisher))
        .route("/v1/publishers/:id/revoke", post(revoke_publisher))
        .route("/v1/publish/:id/:version", post(publish_web_plugin))
        .layer(axum::middleware::from_fn_with_state(state.clone(), check_publish_auth));

    Router::new()
        .merge(read_routes)
        .merge(write_routes)
        .layer(axum::extract::DefaultBodyLimit::max(50 * 1024 * 1024))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

pub fn run_server(port: u16) -> anyhow::Result<()> {
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        let root = data_dir();
        tracing::info!(dir = %root.display(), "Web registry data directory");

        let storage = WebRegistryStorage::new(root);
        storage.init().await?;

        let auth_token = std::env::var("REGISTRY_AUTH_TOKEN").ok().filter(|s| !s.is_empty());
        if auth_token.is_some() {
            tracing::info!("Auth token configured — publish endpoints require authorization");
        }

        let state = Arc::new(AppState { storage, auth_token });
        let app = build_router(state);

        let addr = SocketAddr::from(([0, 0, 0, 0], port));
        tracing::info!("Web registry server listening on http://{addr}");
        println!("Web registry server listening on http://{addr}");

        let listener = tokio::net::TcpListener::bind(addr).await?;
        axum::serve(listener, app).await?;
        Ok(())
    })
}
