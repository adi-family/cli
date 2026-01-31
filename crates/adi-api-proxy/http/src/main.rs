//! ADI API Proxy - HTTP Server
//!
//! LLM API proxy with BYOK/Platform modes, Rhai scripting, and analytics.

mod handlers;
mod middleware;
mod routes;
mod state;

use std::net::SocketAddr;

use adi_api_proxy_core::{Config, Database, SecretManager};
use axum::{
    routing::{delete, get, patch, post},
    Router,
};
use lib_analytics_core::AnalyticsClient;
use lib_http_common::version_header_layer;

use sqlx::postgres::PgPoolOptions;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use state::AppState;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load .env file
    dotenvy::dotenv().ok();

    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "adi_api_proxy=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Load configuration
    let config = Config::from_env()?;
    let addr: SocketAddr = format!("{}:{}", config.host, config.port).parse()?;

    // Create database pool
    let pool = PgPoolOptions::new()
        .max_connections(config.database_max_connections)
        .connect(&config.database_url)
        .await?;
    let db = Database::new(pool);

    // Create secret manager
    let secrets = SecretManager::from_hex(&config.encryption_key)?;

    // Create analytics client
    let analytics = AnalyticsClient::new(&config.analytics_url);

    // Create application state
    let state = AppState::new(db, config, analytics, secrets);

    // Build CORS layer
    let cors = CorsLayer::permissive();

    // Build router
    let app = Router::new()
        // Health check
        .route("/health", get(health_check))
        // Management API (JWT auth)
        .nest("/api/proxy", management_routes())
        // Proxy API (proxy token auth)
        .nest("/v1", proxy_routes())
        // Layers
        .layer(version_header_layer(
            env!("CARGO_PKG_NAME"),
            env!("CARGO_PKG_VERSION"),
        ))
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .with_state(state);

    // Start server
    tracing::info!("Starting adi-api-proxy on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

/// Health check endpoint.
async fn health_check() -> &'static str {
    "ok"
}

/// Management API routes (JWT authenticated).
fn management_routes() -> Router<AppState> {
    Router::new()
        // Upstream API keys (user's BYOK keys)
        .route("/keys", post(routes::keys::create_key))
        .route("/keys", get(routes::keys::list_keys))
        .route("/keys/:id", get(routes::keys::get_key))
        .route("/keys/:id", patch(routes::keys::update_key))
        .route("/keys/:id", delete(routes::keys::delete_key))
        .route("/keys/:id/verify", post(routes::keys::verify_key))
        // Platform keys (admin - platform's own API keys)
        .route(
            "/platform-keys",
            get(routes::platform_keys::list_platform_keys),
        )
        .route(
            "/platform-keys",
            post(routes::platform_keys::upsert_platform_key),
        )
        .route(
            "/platform-keys/:id",
            patch(routes::platform_keys::update_platform_key),
        )
        .route(
            "/platform-keys/:id",
            delete(routes::platform_keys::delete_platform_key),
        )
        .route(
            "/platform-keys/:provider_type/verify",
            post(routes::platform_keys::verify_platform_key),
        )
        // Proxy tokens
        .route("/tokens", post(routes::tokens::create_token))
        .route("/tokens", get(routes::tokens::list_tokens))
        .route("/tokens/:id", get(routes::tokens::get_token))
        .route("/tokens/:id", patch(routes::tokens::update_token))
        .route("/tokens/:id", delete(routes::tokens::delete_token))
        .route("/tokens/:id/rotate", post(routes::tokens::rotate_token))
        // Providers
        .route("/providers", get(routes::providers::list_providers))
        // Usage
        .route("/usage", get(routes::usage::query_usage))
        .route("/usage/summary", get(routes::usage::usage_summary))
        .route("/usage/export", get(routes::usage::export_usage))
}

/// Proxy API routes (proxy token authenticated).
fn proxy_routes() -> Router<AppState> {
    Router::new()
        .route("/chat/completions", post(handlers::chat::chat_completions))
        .route("/completions", post(handlers::completions::completions))
        .route("/embeddings", post(handlers::embeddings::embeddings))
        .route("/models", get(handlers::models::list_models))
        .route("/messages", post(handlers::messages::messages))
}
