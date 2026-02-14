use std::net::SocketAddr;
use std::sync::Arc;

use axum::{
    Json, Router,
    http::{HeaderValue, Method, header},
    response::IntoResponse,
    routing::{delete, get, post, put},
};
use sqlx::postgres::PgPoolOptions;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod auth;
mod error;
mod generated;
mod handlers;
mod middleware;

use credentials_core::{Config, Database, SecretManager};
use lib_analytics_core::AnalyticsClient;
use lib_http_common::version_header_layer;
use lib_logging_core::trace_layer;

#[derive(Clone)]
pub struct AppState {
    pub db: Database,
    pub config: Arc<Config>,
    pub analytics: AnalyticsClient,
    pub secrets: SecretManager,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "credentials_http=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = Config::from_env()?;
    let addr: SocketAddr = format!("{}:{}", config.host, config.port).parse()?;

    let secrets = SecretManager::from_hex(&config.encryption_key)?;
    tracing::info!("Encryption initialized");

    let pool = PgPoolOptions::new()
        .max_connections(config.database_max_connections)
        .connect(&config.database_url)
        .await
        .map_err(|e| {
            let safe_url = redact_password(&config.database_url);
            anyhow::anyhow!("Failed to connect to database at '{}': {}", safe_url, e)
        })?;

    let db = Database::new(pool.clone());

    let analytics_url =
        lib_env_parse::env_or("ANALYTICS_URL", "http://localhost:8094");

    let analytics_client = AnalyticsClient::new(analytics_url);
    tracing::info!("Analytics client initialized");

    let state = AppState {
        db,
        config: Arc::new(config),
        analytics: analytics_client,
        secrets,
    };

    let cors = CorsLayer::new()
        .allow_origin(
            lib_env_parse::env_or("CORS_ORIGIN", "http://localhost:8013")
                .parse::<HeaderValue>()
                .expect("Invalid CORS_ORIGIN"),
        )
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([
            header::CONTENT_TYPE,
            header::AUTHORIZATION,
            header::ACCEPT,
            header::COOKIE,
        ])
        .allow_credentials(true);

    let app = Router::new()
        .route("/health", get(health_check))
        .route("/credentials", get(handlers::credentials::list))
        .route("/credentials", post(handlers::credentials::create))
        .route("/credentials/{id}", get(handlers::credentials::get))
        .route("/credentials/{id}", put(handlers::credentials::update))
        .route("/credentials/{id}", delete(handlers::credentials::delete))
        .route(
            "/credentials/{id}/data",
            get(handlers::credentials::get_with_data),
        )
        .route(
            "/credentials/{id}/verify",
            get(handlers::credentials::verify),
        )
        .route(
            "/credentials/{id}/logs",
            get(handlers::credentials::get_access_logs),
        )
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            middleware::analytics_middleware,
        ))
        .layer(version_header_layer(
            env!("CARGO_PKG_NAME"),
            env!("CARGO_PKG_VERSION"),
        ))
        .layer(trace_layer())
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .with_state(state);

    tracing::info!("Starting Credentials API on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn health_check() -> impl IntoResponse {
    Json(serde_json::json!({ "status": "ok" }))
}

fn redact_password(url: &str) -> String {
    if let Some(at_pos) = url.find('@') {
        if let Some(colon_pos) = url[..at_pos].rfind(':') {
            let scheme_end = url.find("://").map(|p| p + 3).unwrap_or(0);
            if colon_pos > scheme_end {
                return format!("{}***{}", &url[..colon_pos + 1], &url[at_pos..]);
            }
        }
    }
    url.to_string()
}
