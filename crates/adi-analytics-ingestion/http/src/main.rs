use adi_analytics_ingestion_core::EventWriter;
use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::post,
    Json, Router,
};
use lib_analytics_core::EnrichedEvent;
use sqlx::postgres::PgPoolOptions;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Clone)]
struct AppState {
    writer: EventWriter,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load .env file
    dotenvy::dotenv().ok();

    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "adi_analytics_ingestion=info,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Get database URL
    let database_url = std::env::var("DATABASE_URL")
        .or_else(|_| std::env::var("PLATFORM_DATABASE_URL"))
        .expect("DATABASE_URL or PLATFORM_DATABASE_URL must be set");

    // Create database pool
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&database_url)
        .await?;

    tracing::info!("Connected to database");

    // Create event writer
    let writer = EventWriter::new(pool);
    let state = AppState { writer };

    // Build router
    let app = Router::new()
        .route("/health", axum::routing::get(health_check))
        .route("/events/batch", post(ingest_events))
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    // Get port from environment
    let port = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8094);

    let addr = format!("0.0.0.0:{}", port);
    tracing::info!("Analytics ingestion service listening on {}", addr);

    // Start server
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn health_check() -> impl IntoResponse {
    Json(serde_json::json!({ "status": "ok" }))
}

/// Receive and persist a batch of analytics events
async fn ingest_events(
    State(state): State<AppState>,
    Json(events): Json<Vec<EnrichedEvent>>,
) -> Result<impl IntoResponse, StatusCode> {
    let count = events.len();

    if count == 0 {
        return Ok((StatusCode::OK, Json(serde_json::json!({ "received": 0 }))));
    }

    tracing::debug!("Received batch of {} events", count);

    // Write to database
    state
        .writer
        .write_batch(&events)
        .await
        .map_err(|e| {
            tracing::error!("Failed to write events to database: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({ "received": count })),
    ))
}
