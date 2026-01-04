use crate::models::{EndpointLatency, TimeRangeParams};
use axum::{extract::{Query, State}, http::StatusCode, Json};
use sqlx::{PgPool, FromRow};
use chrono::{DateTime, Utc};

#[derive(FromRow)]
struct EndpointLatencyRow {
    hour: DateTime<Utc>,
    service: String,
    endpoint: Option<String>,
    method: Option<String>,
    request_count: i64,
    avg_duration_ms: Option<f64>,
    p50_duration_ms: Option<f64>,
    p95_duration_ms: Option<f64>,
    p99_duration_ms: Option<f64>,
    error_4xx_count: Option<i64>,
    error_5xx_count: Option<i64>,
}

/// Get API endpoint latency statistics
pub async fn get_endpoint_latency(
    Query(params): Query<TimeRangeParams>,
    State(pool): State<PgPool>,
) -> Result<Json<Vec<EndpointLatency>>, StatusCode> {
    let rows = sqlx::query_as::<_, EndpointLatencyRow>(
        r#"
        SELECT
            hour,
            service,
            endpoint,
            method,
            request_count,
            avg_duration_ms,
            p50_duration_ms,
            p95_duration_ms,
            p99_duration_ms,
            error_4xx_count,
            error_5xx_count
        FROM analytics_api_latency_hourly
        WHERE hour >= $1 AND hour <= $2
        ORDER BY hour DESC, p99_duration_ms DESC
        LIMIT 1000
        "#,
    )
    .bind(params.start_date)
    .bind(params.end_date)
    .fetch_all(&pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to fetch endpoint latency: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let latencies = rows
        .into_iter()
        .map(|row| EndpointLatency {
            hour: row.hour,
            service: row.service,
            endpoint: row.endpoint.unwrap_or_default(),
            method: row.method.unwrap_or_default(),
            request_count: row.request_count,
            avg_duration_ms: row.avg_duration_ms.unwrap_or(0.0),
            p50_duration_ms: row.p50_duration_ms.unwrap_or(0.0),
            p95_duration_ms: row.p95_duration_ms.unwrap_or(0.0),
            p99_duration_ms: row.p99_duration_ms.unwrap_or(0.0),
            error_4xx_count: row.error_4xx_count.unwrap_or(0),
            error_5xx_count: row.error_5xx_count.unwrap_or(0),
        })
        .collect();

    Ok(Json(latencies))
}

/// Get slowest endpoints (top 10)
pub async fn get_slowest_endpoints(
    State(pool): State<PgPool>,
) -> Result<Json<Vec<EndpointLatency>>, StatusCode> {
    let rows = sqlx::query_as::<_, EndpointLatencyRow>(
        r#"
        SELECT
            hour,
            service,
            endpoint,
            method,
            request_count,
            avg_duration_ms,
            p50_duration_ms,
            p95_duration_ms,
            p99_duration_ms,
            error_4xx_count,
            error_5xx_count
        FROM analytics_api_latency_hourly
        WHERE hour >= NOW() - INTERVAL '24 hours'
        ORDER BY p99_duration_ms DESC
        LIMIT 10
        "#,
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to fetch slowest endpoints: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let latencies = rows
        .into_iter()
        .map(|row| EndpointLatency {
            hour: row.hour,
            service: row.service,
            endpoint: row.endpoint.unwrap_or_default(),
            method: row.method.unwrap_or_default(),
            request_count: row.request_count,
            avg_duration_ms: row.avg_duration_ms.unwrap_or(0.0),
            p50_duration_ms: row.p50_duration_ms.unwrap_or(0.0),
            p95_duration_ms: row.p95_duration_ms.unwrap_or(0.0),
            p99_duration_ms: row.p99_duration_ms.unwrap_or(0.0),
            error_4xx_count: row.error_4xx_count.unwrap_or(0),
            error_5xx_count: row.error_5xx_count.unwrap_or(0),
        })
        .collect();

    Ok(Json(latencies))
}
