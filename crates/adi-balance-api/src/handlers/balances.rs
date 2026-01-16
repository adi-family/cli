use axum::{
    extract::{Path, State},
    Json,
};
use lib_analytics_core::AnalyticsEvent;
use uuid::Uuid;

use crate::{
    auth::AuthUser,
    error::{ApiError, ApiResult},
    models::{Balance, BalanceResponse, InitBalanceRequest},
    AppState,
};

pub async fn get_my_balance(
    State(state): State<AppState>,
    user: AuthUser,
) -> ApiResult<Json<BalanceResponse>> {
    let balance = sqlx::query_as::<_, Balance>(
        "SELECT * FROM balances WHERE user_id = $1"
    )
    .bind(user.id)
    .fetch_optional(state.db.pool())
    .await?
    .ok_or(ApiError::NotFound)?;

    Ok(Json(BalanceResponse::from(balance)))
}

pub async fn get_balance_by_user(
    State(state): State<AppState>,
    Path(user_id): Path<Uuid>,
) -> ApiResult<Json<BalanceResponse>> {
    let balance = sqlx::query_as::<_, Balance>(
        "SELECT * FROM balances WHERE user_id = $1"
    )
    .bind(user_id)
    .fetch_optional(state.db.pool())
    .await?
    .ok_or(ApiError::NotFound)?;

    Ok(Json(BalanceResponse::from(balance)))
}

pub async fn init_balance(
    State(state): State<AppState>,
    user: AuthUser,
    Json(input): Json<InitBalanceRequest>,
) -> ApiResult<Json<BalanceResponse>> {
    let target_user_id = input.user_id.unwrap_or(user.id);

    let existing = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM balances WHERE user_id = $1)"
    )
    .bind(target_user_id)
    .fetch_one(state.db.pool())
    .await?;

    if existing {
        return Err(ApiError::Conflict("Balance already exists for this user".into()));
    }

    let balance = sqlx::query_as::<_, Balance>(
        r#"
        INSERT INTO balances (user_id, amount, currency)
        VALUES ($1, 0, 'ADI_TOKEN')
        RETURNING *
        "#
    )
    .bind(target_user_id)
    .fetch_one(state.db.pool())
    .await?;

    state.analytics.track(AnalyticsEvent::BalanceCreated {
        user_id: target_user_id,
        balance_id: balance.id,
    });

    Ok(Json(BalanceResponse::from(balance)))
}
