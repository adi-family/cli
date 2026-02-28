use axum::{Json, extract::State};
use serde::Deserialize;

use crate::auth::AuthUser;
use crate::balance_client::{
    self, BalanceResponse, BalanceTransaction, BalanceTransactionResponse,
};
use crate::error::ApiResult;
use crate::AppState;

pub async fn get_balance(
    State(state): State<AppState>,
    auth: AuthUser,
) -> ApiResult<Json<BalanceResponse>> {
    let balance = balance_client::get_or_create_balance(state.db.pool(), auth.id).await?;

    Ok(Json(BalanceResponse {
        subscription_credits: balance.subscription_credits,
        extra_credits: balance.extra_credits,
        total_credits: balance.total(),
        updated_at: balance.updated_at,
    }))
}

pub async fn list_transactions(
    State(state): State<AppState>,
    auth: AuthUser,
) -> ApiResult<Json<Vec<BalanceTransactionResponse>>> {
    let txns: Vec<BalanceTransaction> = sqlx::query_as(
        "SELECT * FROM balance_transactions WHERE user_id = $1 ORDER BY created_at DESC LIMIT 100",
    )
    .bind(auth.id)
    .fetch_all(state.db.pool())
    .await?;

    Ok(Json(txns.into_iter().map(Into::into).collect()))
}

#[derive(Debug, Deserialize)]
pub struct DebitRequest {
    pub amount: i64,
    pub description: Option<String>,
}

pub async fn debit_balance(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<DebitRequest>,
) -> ApiResult<Json<BalanceResponse>> {
    let description = req
        .description
        .as_deref()
        .unwrap_or("Manual debit");

    balance_client::debit(state.db.pool(), auth.id, req.amount, description).await?;

    let balance = balance_client::get_or_create_balance(state.db.pool(), auth.id).await?;

    Ok(Json(BalanceResponse {
        subscription_credits: balance.subscription_credits,
        extra_credits: balance.extra_credits,
        total_credits: balance.total(),
        updated_at: balance.updated_at,
    }))
}
