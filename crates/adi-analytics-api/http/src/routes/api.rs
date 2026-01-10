use adi_analytics_api_core::TimeRangeParams;
use axum::{extract::{Query, State}, http::StatusCode, Json};
use sqlx::PgPool;

pub async fn get_endpoint_latency(
    Query(params): Query<TimeRangeParams>,
    State(pool): State<PgPool>,
) -> Result<Json<Vec<adi_analytics_api_core::EndpointLatency>>, StatusCode> {
    adi_analytics_api_core::get_endpoint_latency(&pool, params.start_date, params.end_date)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to fetch endpoint latency: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })
}

pub async fn get_slowest_endpoints(
    State(pool): State<PgPool>,
) -> Result<Json<Vec<adi_analytics_api_core::EndpointLatency>>, StatusCode> {
    adi_analytics_api_core::get_slowest_endpoints(&pool)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to fetch slowest endpoints: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })
}
