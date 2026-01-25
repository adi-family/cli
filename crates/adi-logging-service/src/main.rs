//! ADI Logging Service - Centralized log ingestion and query API.
//!
//! Endpoints:
//! - POST /logs/batch - Ingest batch of logs
//! - GET /logs - Query logs with filters
//! - GET /logs/trace/:trace_id - Get all logs for a trace
//! - GET /logs/span/:span_id - Get logs for a specific span
//! - GET /health - Health check

mod writer;
mod routes;

use axum::{Router, routing::{get, post}};
use lib_http_common::version_header_layer;
use sqlx::postgres::PgPoolOptions;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::writer::LogWriter;

#[derive(Clone)]
pub struct AppState {
    pub writer: LogWriter,
    pub pool: sqlx::PgPool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load .env file
    dotenvy::dotenv().ok();

    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "adi_logging_service=info,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Get database URL
    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");

    // Create database pool
    let pool = PgPoolOptions::new()
        .max_connections(20)
        .connect(&database_url)
        .await?;

    tracing::info!("Connected to database");

    // Create log writer
    let writer = LogWriter::new(pool.clone());
    let state = AppState { writer, pool };

    // Build router
    let app = Router::new()
        // Health check
        .route("/health", get(routes::health_check))
        // Ingestion
        .route("/logs/batch", post(routes::ingest_logs))
        // Query API
        .route("/logs", get(routes::query_logs))
        .route("/logs/trace/:trace_id", get(routes::get_trace_logs))
        .route("/logs/span/:span_id", get(routes::get_span_logs))
        .route("/logs/stats", get(routes::get_stats))
        // Middleware
        .layer(version_header_layer(
            env!("CARGO_PKG_NAME"),
            env!("CARGO_PKG_VERSION"),
        ))
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    // Get port from environment
    let port = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8040);

    let addr = format!("0.0.0.0:{}", port);
    tracing::info!("Logging service listening on {}", addr);

    // Start server
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
