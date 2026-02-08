//! ADI Logging Service - Centralized log ingestion and query API.
//!
//! Endpoints:
//! - POST /logs/batch - Ingest batch of logs
//! - GET /logs - Query logs with filters
//! - GET /logs/trace/:trace_id - Get all logs for a trace
//! - GET /logs/span/:span_id - Get logs for a specific span
//! - GET /health - Health check

mod generated;
mod routes;

use axum::{Router, routing::{get, post}};
use lib_http_common::version_header_layer;
use logging_core::LogWriter;
use sqlx::postgres::PgPoolOptions;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Clone)]
pub struct AppState {
    pub writer: LogWriter,
    pub pool: sqlx::PgPool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "logging_http=info,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");

    let pool = PgPoolOptions::new()
        .max_connections(20)
        .connect(&database_url)
        .await?;

    tracing::info!("Connected to database");

    let writer = LogWriter::new(pool.clone());
    let state = AppState { writer, pool };

    let app = Router::new()
        .route("/health", get(routes::health_check))
        .route("/logs/batch", post(routes::ingest_logs))
        .route("/logs", get(routes::query_logs))
        .route("/logs/trace/:trace_id", get(routes::get_trace_logs))
        .route("/logs/span/:span_id", get(routes::get_span_logs))
        .route("/logs/cocoon/:cocoon_id", get(routes::get_cocoon_logs))
        .route("/logs/user/:user_id", get(routes::get_user_logs))
        .route("/logs/session/:session_id", get(routes::get_session_logs))
        .route("/logs/stats", get(routes::get_stats))
        .layer(version_header_layer(
            env!("CARGO_PKG_NAME"),
            env!("CARGO_PKG_VERSION"),
        ))
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let port = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8040);

    let addr = format!("0.0.0.0:{}", port);
    tracing::info!("Logging service listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
