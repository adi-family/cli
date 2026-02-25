use analytics_client::{EnrichedEvent, EventWriter};
use axum::{Json, Router, extract::State, http::StatusCode, response::IntoResponse, routing::{get, post}};
use lib_http_common::version_header_layer;
use sqlx::PgPool;
use tower_http::trace::TraceLayer;

#[derive(Clone)]
struct AppState {
    writer: EventWriter,
}

pub fn create_router(pool: PgPool) -> Router {
    let writer = EventWriter::new(pool);
    let state = AppState { writer };

    Router::new()
        .route("/health", get(health_check))
        .route("/events/batch", post(ingest_events))
        .layer(version_header_layer(
            env!("CARGO_PKG_NAME"),
            env!("CARGO_PKG_VERSION"),
        ))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

async fn health_check() -> impl IntoResponse {
    Json(serde_json::json!({ "status": "ok" }))
}

async fn ingest_events(
    State(state): State<AppState>,
    Json(events): Json<Vec<EnrichedEvent>>,
) -> Result<impl IntoResponse, StatusCode> {
    let count = events.len();

    if count == 0 {
        return Ok((StatusCode::OK, Json(serde_json::json!({ "received": 0 }))));
    }

    tracing::debug!("Received batch of {} events", count);

    state.writer.write_batch(&events).await.map_err(|e| {
        tracing::error!("Failed to write events to database: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({ "received": count })),
    ))
}
