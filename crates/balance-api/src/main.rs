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

mod auth;
mod config;
mod db;
mod error;
mod generated;
mod handlers;
mod middleware;
mod models;

use config::Config;
use db::Database;
use lib_analytics_core::AnalyticsClient;
use lib_http_common::version_header_layer;
use lib_logging_core::trace_layer;

#[derive(Clone)]
pub struct AppState {
    pub db: Database,
    pub config: Arc<Config>,
    pub analytics: AnalyticsClient,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "adi_balance_api=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = Config::from_env()?;
    let addr: SocketAddr = format!("{}:{}", config.host, config.port).parse()?;

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
        std::env::var("ANALYTICS_URL").unwrap_or_else(|_| "http://localhost:8094".to_string());

    let analytics_client = AnalyticsClient::new(analytics_url);
    tracing::info!("Analytics client initialized");

    let state = AppState {
        db,
        config: Arc::new(config),
        analytics: analytics_client,
    };

    let cors = CorsLayer::new()
        .allow_origin(
            std::env::var("CORS_ORIGIN")
                .unwrap_or_else(|_| "http://localhost:8013".to_string())
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
        .route("/balances/me", get(handlers::balances::get_my_balance))
        .route("/balances/init", post(handlers::balances::init_balance))
        .route(
            "/balances/{user_id}",
            get(handlers::balances::get_balance_by_user),
        )
        .route(
            "/transactions",
            get(handlers::transactions::list_transactions),
        )
        .route(
            "/transactions/deposit",
            post(handlers::transactions::deposit),
        )
        .route("/transactions/debit", post(handlers::transactions::debit))
        .route(
            "/transactions/check",
            post(handlers::transactions::check_balance),
        )
        .route(
            "/transactions/{id}",
            get(handlers::transactions::get_transaction),
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

    tracing::info!("Starting Balance API on {}", addr);
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
