use axum::Json;
use axum::extract::{Query, State};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use uuid::Uuid;

use crate::AppState;
use crate::auth::AuthUser;
use llm_proxy_core::error::ApiResult;
use llm_proxy_core::db;

#[derive(Deserialize)]
pub struct UsageQuery {
    pub proxy_token_id: Option<Uuid>,
    pub from: Option<String>,
    pub to: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

fn parse_datetime(s: &str) -> Result<DateTime<Utc>, llm_proxy_core::ApiError> {
    s.parse::<DateTime<Utc>>()
        .map_err(|_| llm_proxy_core::ApiError::BadRequest(format!("Invalid datetime: {s}")))
}

pub async fn query(
    State(state): State<AppState>,
    user: AuthUser,
    Query(q): Query<UsageQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    let from_dt = q.from.as_deref().map(parse_datetime).transpose()?;
    let to_dt = q.to.as_deref().map(parse_datetime).transpose()?;
    let limit = q.limit.unwrap_or(50);
    let offset = q.offset.unwrap_or(0);

    let logs = db::query_usage(
        state.db.pool(),
        user.id,
        q.proxy_token_id,
        from_dt,
        to_dt,
        limit,
        offset,
    )
    .await?;

    let summary = db::get_usage_summary(
        state.db.pool(),
        user.id,
        q.proxy_token_id,
        from_dt,
        to_dt,
    )
    .await?;

    Ok(Json(serde_json::json!({
        "logs": logs,
        "total": summary.total_requests,
    })))
}
