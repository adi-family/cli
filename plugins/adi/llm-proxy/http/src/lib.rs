mod auth;
mod handlers;

use std::net::SocketAddr;
use std::sync::Arc;

use axum::Router;
use axum::routing::{get, patch, post};
use llm_proxy_core::{Config, Database, SecretManager};
use sqlx::postgres::PgPoolOptions;
use tower_http::cors::CorsLayer;
use tracing_subscriber::prelude::*;
use tower_http::trace::TraceLayer;

#[derive(Clone)]
pub struct AppState {
    pub db: Database,
    pub secrets: SecretManager,
    pub config: Arc<Config>,
}

pub fn run_server() -> anyhow::Result<()> {
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        dotenvy::dotenv().ok();

        tracing_subscriber::registry()
            .with(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| "llm_proxy_http=debug,tower_http=debug".into()),
            )
            .with(tracing_subscriber::fmt::layer())
            .init();

        let config = Config::from_env()?;
        let addr: SocketAddr = format!("{}:{}", config.host, config.port).parse()?;

        let pool = PgPoolOptions::new()
            .max_connections(config.database_max_connections)
            .connect(&config.database_url)
            .await?;

        let secrets = SecretManager::from_hex(&config.encryption_key)
            .map_err(|e| anyhow::anyhow!("{e}"))?;

        let state = AppState {
            db: Database::new(pool),
            secrets,
            config: Arc::new(config),
        };

        let app = router(state);

        tracing::info!("Starting LLM Proxy HTTP server on {}", addr);
        let listener = tokio::net::TcpListener::bind(addr).await?;
        axum::serve(listener, app).await?;

        Ok(())
    })
}

fn router(state: AppState) -> Router {
    let management = Router::new()
        // Keys
        .route("/keys", get(handlers::keys::list).post(handlers::keys::create))
        .route(
            "/keys/{id}",
            get(handlers::keys::get_one)
                .patch(handlers::keys::update)
                .delete(handlers::keys::delete_one),
        )
        .route("/keys/{id}/verify", post(handlers::keys::verify))
        // Platform keys
        .route(
            "/platform-keys",
            get(handlers::platform_keys::list).post(handlers::platform_keys::upsert),
        )
        .route(
            "/platform-keys/{id}",
            patch(handlers::platform_keys::update).delete(handlers::platform_keys::delete_one),
        )
        // Tokens
        .route(
            "/tokens",
            get(handlers::tokens::list).post(handlers::tokens::create),
        )
        .route(
            "/tokens/{id}",
            get(handlers::tokens::get_one)
                .patch(handlers::tokens::update)
                .delete(handlers::tokens::delete_one),
        )
        .route("/tokens/{id}/rotate", post(handlers::tokens::rotate))
        // Providers
        .route("/providers", get(handlers::providers::list))
        // Usage
        .route("/usage", get(handlers::usage::query));

    let proxy = Router::new()
        .route("/chat/completions", post(handlers::proxy::forward))
        .route("/completions", post(handlers::proxy::forward))
        .route("/embeddings", post(handlers::proxy::forward))
        .route("/messages", post(handlers::proxy::forward))
        .route("/models", get(handlers::proxy::list_models));

    Router::new()
        .route("/health", get(health))
        .nest("/api/llm-proxy", management)
        .nest("/v1", proxy)
        .layer(lib_http_common::version_header_layer(
            env!("CARGO_PKG_NAME"),
            env!("CARGO_PKG_VERSION"),
        ))
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .with_state(state)
}

async fn health() -> &'static str {
    "ok"
}
