//! HTTP route handlers for the logging service.

use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use chrono::{DateTime, Utc};
use lib_logging_core::EnrichedLogEntry;
use logging_core::{LogQueryParams, LogReader, LogWriter, PaginationParams};
use serde::Deserialize;
use sqlx::postgres::PgPoolOptions;
use tower_http::trace::TraceLayer;
use uuid::Uuid;

#[derive(Clone)]
struct AppState {
    writer: LogWriter,
    reader: LogReader,
}

/// Start the HTTP server.
pub async fn run_server(database_url: &str, port: u16) -> lib_plugin_abi_v3::Result<()> {
    let pool = PgPoolOptions::new()
        .max_connections(20)
        .connect(database_url)
        .await
        .map_err(|e| lib_plugin_abi_v3::PluginError::Runtime(format!("Database connection failed: {}", e)))?;

    tracing::info!("Connected to database");

    let state = AppState {
        writer: LogWriter::new(pool.clone()),
        reader: LogReader::new(pool),
    };

    let app = Router::new()
        .route("/health", get(health_check))
        .route("/logs/batch", post(ingest_logs))
        .route("/logs", get(query_logs))
        .route("/logs/trace/:trace_id", get(get_trace_logs))
        .route("/logs/span/:span_id", get(get_span_logs))
        .route("/logs/cocoon/:cocoon_id", get(get_cocoon_logs))
        .route("/logs/user/:user_id", get(get_user_logs))
        .route("/logs/session/:session_id", get(get_session_logs))
        .route("/logs/stats", get(get_stats))
        .layer(lib_http_common::version_header_layer(
            env!("CARGO_PKG_NAME"),
            env!("CARGO_PKG_VERSION"),
        ))
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr = format!("0.0.0.0:{}", port);
    tracing::info!("Logging service listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .map_err(|e| lib_plugin_abi_v3::PluginError::Runtime(format!("Bind failed: {}", e)))?;

    axum::serve(listener, app)
        .await
        .map_err(|e| lib_plugin_abi_v3::PluginError::Runtime(format!("Server error: {}", e)))?;

    Ok(())
}

async fn health_check() -> impl IntoResponse {
    Json(serde_json::json!({ "status": "ok" }))
}

async fn ingest_logs(
    State(state): State<AppState>,
    Json(entries): Json<Vec<EnrichedLogEntry>>,
) -> Result<impl IntoResponse, StatusCode> {
    let count = entries.len();
    if count == 0 {
        return Ok((StatusCode::OK, Json(serde_json::json!({ "received": 0 }))));
    }

    tracing::debug!("Received batch of {} log entries", count);

    state.writer.write_batch(&entries).await.map_err(|e| {
        tracing::error!("Failed to write logs to database: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({ "received": count })),
    ))
}

#[derive(Debug, Deserialize)]
struct LogQueryRequest {
    service: Option<String>,
    level: Option<String>,
    trace_id: Option<Uuid>,
    cocoon_id: Option<String>,
    user_id: Option<String>,
    session_id: Option<String>,
    hive_id: Option<String>,
    search: Option<String>,
    from: Option<DateTime<Utc>>,
    to: Option<DateTime<Utc>>,
    limit: Option<i64>,
    offset: Option<i64>,
}

async fn query_logs(
    State(state): State<AppState>,
    Query(query): Query<LogQueryRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    let limit = query.limit.unwrap_or(100).min(1000);
    let offset = query.offset.unwrap_or(0);

    let params = LogQueryParams {
        service: query.service,
        level: query.level,
        trace_id: query.trace_id,
        cocoon_id: query.cocoon_id,
        user_id: query.user_id,
        session_id: query.session_id,
        hive_id: query.hive_id,
        search: query.search,
        from: query.from,
        to: query.to,
        limit: Some(limit),
        offset: Some(offset),
    };

    let logs = state.reader.query(&params).await.map_err(|e| {
        tracing::error!("Failed to query logs: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let count = logs.len() as i64;
    Ok(Json(serde_json::json!({
        "logs": logs,
        "count": count,
        "limit": limit,
        "offset": offset,
    })))
}

async fn get_trace_logs(
    State(state): State<AppState>,
    Path(trace_id): Path<Uuid>,
) -> Result<impl IntoResponse, StatusCode> {
    let logs = state.reader.trace_logs(trace_id).await.map_err(|e| {
        tracing::error!("Failed to query trace logs: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let count = logs.len() as i64;
    Ok(Json(serde_json::json!({
        "trace_id": trace_id,
        "logs": logs,
        "count": count,
    })))
}

async fn get_span_logs(
    State(state): State<AppState>,
    Path(span_id): Path<Uuid>,
) -> Result<impl IntoResponse, StatusCode> {
    let logs = state.reader.span_logs(span_id).await.map_err(|e| {
        tracing::error!("Failed to query span logs: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let count = logs.len() as i64;
    Ok(Json(serde_json::json!({
        "span_id": span_id,
        "logs": logs,
        "count": count,
    })))
}

#[derive(Debug, Deserialize)]
struct PaginationQuery {
    from: Option<DateTime<Utc>>,
    to: Option<DateTime<Utc>>,
    limit: Option<i64>,
    offset: Option<i64>,
}

async fn get_cocoon_logs(
    State(state): State<AppState>,
    Path(cocoon_id): Path<String>,
    Query(query): Query<PaginationQuery>,
) -> Result<impl IntoResponse, StatusCode> {
    let page = PaginationParams {
        from: query.from,
        to: query.to,
        limit: query.limit,
        offset: query.offset,
    };
    let limit = page.limit.unwrap_or(100);
    let offset = page.offset.unwrap_or(0);

    let logs = state
        .reader
        .correlation_logs("cocoon_id", &cocoon_id, &page)
        .await
        .map_err(|e| {
            tracing::error!("Failed to query cocoon logs: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let count = logs.len() as i64;
    Ok(Json(serde_json::json!({
        "cocoon_id": cocoon_id,
        "logs": logs,
        "count": count,
        "limit": limit,
        "offset": offset,
    })))
}

async fn get_user_logs(
    State(state): State<AppState>,
    Path(user_id): Path<String>,
    Query(query): Query<PaginationQuery>,
) -> Result<impl IntoResponse, StatusCode> {
    let page = PaginationParams {
        from: query.from,
        to: query.to,
        limit: query.limit,
        offset: query.offset,
    };
    let limit = page.limit.unwrap_or(100);
    let offset = page.offset.unwrap_or(0);

    let logs = state
        .reader
        .correlation_logs("user_id", &user_id, &page)
        .await
        .map_err(|e| {
            tracing::error!("Failed to query user logs: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let count = logs.len() as i64;
    Ok(Json(serde_json::json!({
        "user_id": user_id,
        "logs": logs,
        "count": count,
        "limit": limit,
        "offset": offset,
    })))
}

async fn get_session_logs(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
    Query(query): Query<PaginationQuery>,
) -> Result<impl IntoResponse, StatusCode> {
    let page = PaginationParams {
        from: query.from,
        to: query.to,
        limit: query.limit,
        offset: query.offset,
    };
    let limit = page.limit.unwrap_or(100);
    let offset = page.offset.unwrap_or(0);

    let logs = state
        .reader
        .correlation_logs("session_id", &session_id, &page)
        .await
        .map_err(|e| {
            tracing::error!("Failed to query session logs: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let count = logs.len() as i64;
    Ok(Json(serde_json::json!({
        "session_id": session_id,
        "logs": logs,
        "count": count,
        "limit": limit,
        "offset": offset,
    })))
}

async fn get_stats(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, StatusCode> {
    let stats = state.reader.stats().await.map_err(|e| {
        tracing::error!("Failed to query stats: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let service_stats = state.reader.service_stats().await.map_err(|e| {
        tracing::error!("Failed to query service stats: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(serde_json::json!({
        "period": "24h",
        "total_logs": stats.total_logs,
        "services_count": stats.services_count,
        "traces_count": stats.traces_count,
        "error_count": stats.error_count,
        "warn_count": stats.warn_count,
        "by_service": service_stats,
    })))
}
