use analytics_client::TimeRangeParams;
use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
};
use sqlx::PgPool;

pub async fn get_endpoint_latency(
    Query(params): Query<TimeRangeParams>,
    State(pool): State<PgPool>,
) -> Result<Json<Vec<analytics_client::EndpointLatency>>, StatusCode> {
    analytics_client::get_endpoint_latency(&pool, params.start_date, params.end_date)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to fetch endpoint latency: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })
}

pub async fn get_slowest_endpoints(
    State(pool): State<PgPool>,
) -> Result<Json<Vec<analytics_client::EndpointLatency>>, StatusCode> {
    analytics_client::get_slowest_endpoints(&pool)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to fetch slowest endpoints: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })
}
