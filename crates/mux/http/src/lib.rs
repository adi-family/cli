use axum::{
    body::Body,
    extract::{Request, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::any,
    Router,
};
use lib_http_common::version_header_layer;
use mux_core::{MuxManager, MuxResponse};
use std::path::PathBuf;
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

struct AppState {
    mux: MuxManager,
}

pub fn run_server(port: u16, config_path: Option<PathBuf>) -> anyhow::Result<()> {
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        tracing_subscriber::fmt()
            .with_env_filter(
                tracing_subscriber::EnvFilter::from_default_env()
                    .add_directive(tracing::Level::INFO.into()),
            )
            .init();

        let mux = match config_path {
            Some(path) => MuxManager::load(&path)
                .map_err(|e| anyhow::anyhow!("Failed to load config: {e}"))?,
            None => MuxManager::load_from_env()
                .map_err(|e| anyhow::anyhow!("Failed to load config: {e}"))?,
        };

        let state = Arc::new(AppState { mux });

        let app = Router::new()
            .route("/{*path}", any(handle))
            .route("/", any(handle_root))
            .with_state(state)
            .layer(version_header_layer(
                env!("CARGO_PKG_NAME"),
                env!("CARGO_PKG_VERSION"),
            ))
            .layer(TraceLayer::new_for_http())
            .layer(CorsLayer::permissive());

        let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}"))
            .await
            .map_err(|e| anyhow::anyhow!("Bind error: {e}"))?;

        tracing::info!(port, "ADI Mux gateway listening");

        axum::serve(listener, app)
            .await
            .map_err(|e| anyhow::anyhow!("Server error: {e}"))?;

        Ok(())
    })
}

async fn handle_root(
    State(state): State<Arc<AppState>>,
    req: Request<Body>,
) -> impl IntoResponse {
    dispatch(state, req, "/").await
}

async fn handle(
    State(state): State<Arc<AppState>>,
    req: Request<Body>,
) -> impl IntoResponse {
    let path = req.uri().path().to_string();
    dispatch(state, req, &path).await
}

async fn dispatch(state: Arc<AppState>, req: Request<Body>, path: &str) -> Response {
    let method = req.method().as_str().to_string();
    let headers = extract_headers(req.headers());

    let body = match axum::body::to_bytes(req.into_body(), usize::MAX).await {
        Ok(b) => b,
        Err(e) => {
            tracing::error!(error = %e, "failed to read request body");
            return error_response(StatusCode::BAD_REQUEST, "failed to read request body");
        }
    };

    match state.mux.handle(&method, path, headers, body).await {
        Ok(MuxResponse::Single(r)) => proxy_response(r),
        Ok(MuxResponse::Aggregate(responses)) => {
            let json: Vec<_> = responses
                .into_iter()
                .map(|r| {
                    serde_json::json!({
                        "backend": r.backend_url,
                        "status": r.status,
                        "body": String::from_utf8_lossy(&r.body),
                    })
                })
                .collect();

            axum::Json(json).into_response()
        }
        Ok(MuxResponse::NoMatch) => {
            error_response(StatusCode::NOT_FOUND, "no route matched")
        }
        Err(mux_core::Error::NoBackends(name)) => error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            &format!("no backends enabled for route {name}"),
        ),
        Err(e) => {
            tracing::error!(error = %e, "mux error");
            error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string())
        }
    }
}

fn proxy_response(r: mux_core::BackendResponse) -> Response {
    let status = StatusCode::from_u16(r.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    let mut builder = axum::http::Response::builder().status(status);

    for (name, value) in &r.headers {
        // Skip hop-by-hop headers.
        let lower = name.to_lowercase();
        if matches!(
            lower.as_str(),
            "transfer-encoding" | "connection" | "keep-alive" | "te" | "trailers" | "upgrade"
        ) {
            continue;
        }
        builder = builder.header(name, value);
    }

    builder
        .body(Body::from(r.body))
        .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
}

fn error_response(status: StatusCode, msg: &str) -> Response {
    let body = serde_json::json!({ "error": msg });
    (status, axum::Json(body)).into_response()
}

fn extract_headers(headers: &HeaderMap) -> Vec<(String, String)> {
    headers
        .iter()
        .filter_map(|(k, v)| v.to_str().ok().map(|v| (k.to_string(), v.to_string())))
        .collect()
}
