use axum::{extract::State, http::StatusCode, Json};
use sqlx::PgPool;

pub async fn get_overview(
    State(pool): State<PgPool>,
) -> Result<Json<adi_analytics_api_core::OverviewStats>, StatusCode> {
    adi_analytics_api_core::get_overview_stats(&pool)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to fetch overview stats: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })
}
