//! HTTP route handlers for the logging service.

use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use chrono::{DateTime, Utc};
use lib_logging_core::EnrichedLogEntry;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::AppState;

// ============================================================================
// Health Check
// ============================================================================

pub async fn health_check() -> impl IntoResponse {
    Json(serde_json::json!({ "status": "ok" }))
}

// ============================================================================
// Ingestion
// ============================================================================

/// Receive and persist a batch of log entries.
pub async fn ingest_logs(
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

// ============================================================================
// Query API
// ============================================================================

/// Query parameters for log search.
#[derive(Debug, Deserialize)]
pub struct LogQuery {
    /// Filter by service name
    pub service: Option<String>,

    /// Filter by minimum log level (trace, debug, info, notice, warn, error, fatal)
    pub level: Option<String>,

    /// Filter by trace ID
    pub trace_id: Option<Uuid>,

    /// Search in message text
    pub search: Option<String>,

    /// Start time (ISO 8601)
    pub from: Option<DateTime<Utc>>,

    /// End time (ISO 8601)
    pub to: Option<DateTime<Utc>>,

    /// Maximum number of results (default: 100, max: 1000)
    pub limit: Option<i64>,

    /// Offset for pagination
    pub offset: Option<i64>,
}

/// Log entry response.
#[derive(Debug, Serialize)]
pub struct LogResponse {
    pub id: i64,
    pub timestamp: DateTime<Utc>,
    pub service: String,
    pub hostname: Option<String>,
    pub environment: Option<String>,
    pub level: String,
    pub message: String,
    pub trace_id: Uuid,
    pub span_id: Uuid,
    pub parent_span_id: Option<Uuid>,
    pub fields: Option<serde_json::Value>,
    pub error_kind: Option<String>,
    pub error_message: Option<String>,
    pub source: Option<String>,
    pub target: Option<String>,
}

/// Query logs with filters.
pub async fn query_logs(
    State(state): State<AppState>,
    Query(query): Query<LogQuery>,
) -> Result<impl IntoResponse, StatusCode> {
    let limit = query.limit.unwrap_or(100).min(1000);
    let offset = query.offset.unwrap_or(0);

    // Parse level filter
    let level_filter: Option<i16> = query.level.as_ref().and_then(|l| {
        match l.to_lowercase().as_str() {
            "trace" => Some(0),
            "debug" => Some(1),
            "info" => Some(2),
            "notice" => Some(3),
            "warn" | "warning" => Some(4),
            "error" => Some(5),
            "fatal" | "critical" => Some(6),
            _ => None,
        }
    });

    // Default time range: last 24 hours
    let from = query.from.unwrap_or_else(|| Utc::now() - chrono::Duration::hours(24));
    let to = query.to.unwrap_or_else(Utc::now);

    let rows = sqlx::query_as::<_, LogRow>(
        r#"
        SELECT
            id, timestamp, service, hostname, environment,
            level_name as level, message,
            trace_id, span_id, parent_span_id,
            fields, error_kind, error_message, source, target
        FROM logs
        WHERE timestamp >= $1 AND timestamp <= $2
            AND ($3::varchar IS NULL OR service = $3)
            AND ($4::smallint IS NULL OR level >= $4)
            AND ($5::uuid IS NULL OR trace_id = $5)
            AND ($6::text IS NULL OR message ILIKE '%' || $6 || '%')
        ORDER BY timestamp DESC
        LIMIT $7 OFFSET $8
        "#,
    )
    .bind(from)
    .bind(to)
    .bind(&query.service)
    .bind(level_filter)
    .bind(query.trace_id)
    .bind(&query.search)
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to query logs: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let logs: Vec<LogResponse> = rows.into_iter().map(|r| r.into()).collect();

    Ok(Json(serde_json::json!({
        "logs": logs,
        "count": logs.len(),
        "limit": limit,
        "offset": offset,
    })))
}

/// Get all logs for a specific trace.
pub async fn get_trace_logs(
    State(state): State<AppState>,
    Path(trace_id): Path<Uuid>,
) -> Result<impl IntoResponse, StatusCode> {
    let rows = sqlx::query_as::<_, LogRow>(
        r#"
        SELECT
            id, timestamp, service, hostname, environment,
            level_name as level, message,
            trace_id, span_id, parent_span_id,
            fields, error_kind, error_message, source, target
        FROM logs
        WHERE trace_id = $1
        ORDER BY timestamp ASC
        LIMIT 1000
        "#,
    )
    .bind(trace_id)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to query trace logs: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let logs: Vec<LogResponse> = rows.into_iter().map(|r| r.into()).collect();

    Ok(Json(serde_json::json!({
        "trace_id": trace_id,
        "logs": logs,
        "count": logs.len(),
    })))
}

/// Get logs for a specific span.
pub async fn get_span_logs(
    State(state): State<AppState>,
    Path(span_id): Path<Uuid>,
) -> Result<impl IntoResponse, StatusCode> {
    let rows = sqlx::query_as::<_, LogRow>(
        r#"
        SELECT
            id, timestamp, service, hostname, environment,
            level_name as level, message,
            trace_id, span_id, parent_span_id,
            fields, error_kind, error_message, source, target
        FROM logs
        WHERE span_id = $1
        ORDER BY timestamp ASC
        LIMIT 1000
        "#,
    )
    .bind(span_id)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to query span logs: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let logs: Vec<LogResponse> = rows.into_iter().map(|r| r.into()).collect();

    Ok(Json(serde_json::json!({
        "span_id": span_id,
        "logs": logs,
        "count": logs.len(),
    })))
}

/// Get logging statistics.
pub async fn get_stats(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, StatusCode> {
    // Get stats for last 24 hours
    let stats = sqlx::query_as::<_, StatsRow>(
        r#"
        SELECT
            COUNT(*) as total_logs,
            COUNT(DISTINCT service) as services_count,
            COUNT(DISTINCT trace_id) as traces_count,
            COUNT(*) FILTER (WHERE level >= 5) as error_count,
            COUNT(*) FILTER (WHERE level >= 4 AND level < 5) as warn_count
        FROM logs
        WHERE timestamp >= NOW() - INTERVAL '24 hours'
        "#,
    )
    .fetch_one(&state.pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to query stats: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Get per-service stats
    let service_stats = sqlx::query_as::<_, ServiceStatsRow>(
        r#"
        SELECT
            service,
            COUNT(*) as log_count,
            COUNT(*) FILTER (WHERE level >= 5) as error_count
        FROM logs
        WHERE timestamp >= NOW() - INTERVAL '24 hours'
        GROUP BY service
        ORDER BY log_count DESC
        LIMIT 20
        "#,
    )
    .fetch_all(&state.pool)
    .await
    .map_err(|e| {
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
        "by_service": service_stats.into_iter().map(|s| serde_json::json!({
            "service": s.service,
            "log_count": s.log_count,
            "error_count": s.error_count,
        })).collect::<Vec<_>>(),
    })))
}

// ============================================================================
// Database Row Types
// ============================================================================

#[derive(sqlx::FromRow)]
struct LogRow {
    id: i64,
    timestamp: DateTime<Utc>,
    service: String,
    hostname: Option<String>,
    environment: Option<String>,
    level: String,
    message: String,
    trace_id: Uuid,
    span_id: Uuid,
    parent_span_id: Option<Uuid>,
    fields: Option<serde_json::Value>,
    error_kind: Option<String>,
    error_message: Option<String>,
    source: Option<String>,
    target: Option<String>,
}

impl From<LogRow> for LogResponse {
    fn from(row: LogRow) -> Self {
        Self {
            id: row.id,
            timestamp: row.timestamp,
            service: row.service,
            hostname: row.hostname,
            environment: row.environment,
            level: row.level,
            message: row.message,
            trace_id: row.trace_id,
            span_id: row.span_id,
            parent_span_id: row.parent_span_id,
            fields: row.fields,
            error_kind: row.error_kind,
            error_message: row.error_message,
            source: row.source,
            target: row.target,
        }
    }
}

#[derive(sqlx::FromRow)]
struct StatsRow {
    total_logs: i64,
    services_count: i64,
    traces_count: i64,
    error_count: i64,
    warn_count: i64,
}

#[derive(sqlx::FromRow)]
struct ServiceStatsRow {
    service: String,
    log_count: i64,
    error_count: i64,
}
