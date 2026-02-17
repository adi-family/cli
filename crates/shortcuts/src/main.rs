use axum::{
    Router,
    extract::{Path, State},
    http::{HeaderValue, StatusCode, header},
    response::{IntoResponse, Json, Response},
    routing::get,
};
use lib_http_common::version_header_layer;
use lib_logging_core::trace_layer;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use lib_env_parse::{env_vars, env_or};

env_vars! {
    ConfigPath => "CONFIG_PATH",
    Port => "PORT",
}

#[derive(Debug, Deserialize)]
struct Config {
    shortcuts: HashMap<String, String>,
}

#[derive(Clone)]
struct AppState {
    shortcuts: Arc<HashMap<String, String>>,
}

#[derive(Serialize)]
struct ShortcutEntry {
    name: String,
    url: String,
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
}

fn load_config(path: &str) -> Config {
    let content = std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("failed to read config at {path}: {e}"));
    serde_yml::from_str(&content)
        .unwrap_or_else(|e| panic!("failed to parse config at {path}: {e}"))
}

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "shortcuts_http=info,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config_path = env_or(EnvVar::ConfigPath.as_str(), ".adi/shortcuts.yaml");
    let config = load_config(&config_path);

    tracing::info!("loaded {} shortcuts from {}", config.shortcuts.len(), config_path);

    let state = AppState {
        shortcuts: Arc::new(config.shortcuts),
    };

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/health", get(health))
        .route("/", get(list_shortcuts))
        .route("/{name}", get(redirect))
        .layer(version_header_layer(
            env!("CARGO_PKG_NAME"),
            env!("CARGO_PKG_VERSION"),
        ))
        .layer(trace_layer())
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let port = env_or(EnvVar::Port.as_str(), "8031");
    let addr = format!("0.0.0.0:{port}");

    tracing::info!("shortcuts-http starting on {addr}");

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn health() -> &'static str {
    "OK"
}

async fn list_shortcuts(State(state): State<AppState>) -> Json<Vec<ShortcutEntry>> {
    let mut entries: Vec<ShortcutEntry> = state
        .shortcuts
        .iter()
        .map(|(name, url)| ShortcutEntry {
            name: name.clone(),
            url: url.clone(),
        })
        .collect();
    entries.sort_by(|a, b| a.name.cmp(&b.name));
    Json(entries)
}

async fn redirect(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Response {
    match state.shortcuts.get(&name) {
        Some(target) => {
            let mut response = StatusCode::FOUND.into_response();
            response.headers_mut().insert(
                header::LOCATION,
                HeaderValue::from_str(target).unwrap_or_else(|_| HeaderValue::from_static("/")),
            );
            response
        }
        None => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("shortcut '{name}' not found"),
            }),
        )
            .into_response(),
    }
}
