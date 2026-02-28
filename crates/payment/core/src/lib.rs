pub mod auth;
pub mod balance_client;
pub mod config;
pub mod db;
pub mod error;
pub mod handlers;
pub mod models;
pub mod providers;
pub mod types;

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

use axum::{
    Json, Router,
    http::{HeaderValue, Method, header},
    response::IntoResponse,
    routing::{get, post},
};
use sqlx::postgres::PgPoolOptions;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use config::Config;
use db::Database;
use lib_http_common::version_header_layer;
use providers::{PaymentProvider, create_providers};
use types::ProviderType;

#[derive(Clone)]
pub struct AppState {
    pub db: Database,
    pub config: Arc<Config>,
    pub providers: Arc<HashMap<ProviderType, Box<dyn PaymentProvider>>>,
    pub http_client: reqwest::Client,
}

pub fn run_server(port: u16) -> anyhow::Result<()> {
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        dotenvy::dotenv().ok();

        tracing_subscriber::registry()
            .with(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| "payment_core=debug,tower_http=debug".into()),
            )
            .with(tracing_subscriber::fmt::layer())
            .init();

        let mut config = Config::from_env()?;
        config.port = port;
        let addr: SocketAddr = format!("{}:{}", config.host, config.port).parse()?;

        let pool = PgPoolOptions::new()
            .max_connections(config.database_max_connections)
            .connect(&config.database_url)
            .await
            .map_err(|e| {
                let safe_url = redact_password(&config.database_url);
                anyhow::anyhow!("Failed to connect to database at '{}': {}", safe_url, e)
            })?;

        let db = Database::new(pool);
        let providers = create_providers(&config);

        let state = AppState {
            db,
            config: Arc::new(config.clone()),
            providers: Arc::new(providers),
            http_client: reqwest::Client::new(),
        };

        let cors = CorsLayer::new()
            .allow_origin(
                config
                    .cors_origin
                    .parse::<HeaderValue>()
                    .expect("Invalid CORS_ORIGIN"),
            )
            .allow_methods([
                Method::GET,
                Method::POST,
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
            .route("/checkout", post(handlers::checkout::create_checkout))
            .route(
                "/subscriptions",
                post(handlers::subscriptions::create_subscription),
            )
            .route(
                "/subscriptions/{id}",
                get(handlers::subscriptions::get_subscription)
                    .delete(handlers::subscriptions::cancel_subscription),
            )
            .route(
                "/webhooks/{provider}",
                post(handlers::webhooks::handle_webhook),
            )
            .layer(version_header_layer(
                env!("CARGO_PKG_NAME"),
                env!("CARGO_PKG_VERSION"),
            ))
            .layer(TraceLayer::new_for_http())
            .layer(cors)
            .with_state(state);

        tracing::info!("Starting Payment API on {}", addr);
        let listener = tokio::net::TcpListener::bind(addr).await?;
        axum::serve(listener, app).await?;

        Ok(())
    })
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
