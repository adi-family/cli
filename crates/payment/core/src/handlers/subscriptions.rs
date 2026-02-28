use axum::{Json, extract::{Path, State}};
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::error::{ApiError, ApiResult};
use crate::models::{
    CreateSubscriptionRequest, Subscription, SubscriptionResponse,
};
use crate::types::ProviderType;
use crate::AppState;

pub async fn create_subscription(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<CreateSubscriptionRequest>,
) -> ApiResult<Json<SubscriptionResponse>> {
    let provider = state
        .providers
        .get(&req.provider)
        .ok_or_else(|| ApiError::ProviderNotConfigured(req.provider.to_string()))?;

    let mut sub = provider.create_subscription(&req).await?;
    sub.user_id = auth.id;

    sqlx::query(
        "INSERT INTO subscriptions (id, provider, provider_subscription_id, user_id, plan_id, status, billing_interval, amount_cents, currency, metadata)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)"
    )
    .bind(sub.id)
    .bind(&sub.provider)
    .bind(&sub.provider_subscription_id)
    .bind(sub.user_id)
    .bind(&sub.plan_id)
    .bind(&sub.status)
    .bind(&sub.billing_interval)
    .bind(sub.amount_cents)
    .bind(&sub.currency)
    .bind(&sub.metadata)
    .execute(state.db.pool())
    .await?;

    // For Coinbase: also create a linked payment record for the charge
    let checkout_url = if req.provider == ProviderType::Coinbase {
        let checkout_url = sub
            .metadata
            .as_ref()
            .and_then(|m| m.get("checkout_url"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let provider_payment_id = sub
            .metadata
            .as_ref()
            .and_then(|m| m.get("provider_payment_id"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        if let Some(ref charge_id) = provider_payment_id {
            let conversion_rate = req
                .conversion_rate
                .unwrap_or(state.config.default_conversion_rate);

            sqlx::query(
                "INSERT INTO payments (id, provider, provider_payment_id, user_id, subscription_id, amount_cents, currency, conversion_rate, status, checkout_url)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)"
            )
            .bind(Uuid::new_v4())
            .bind(ProviderType::Coinbase.to_string())
            .bind(charge_id)
            .bind(auth.id)
            .bind(sub.id)
            .bind(sub.amount_cents.unwrap_or(0))
            .bind(sub.currency.as_deref().unwrap_or("USD"))
            .bind(conversion_rate)
            .bind("pending")
            .bind(checkout_url.as_deref())
            .execute(state.db.pool())
            .await?;
        }

        checkout_url
    } else {
        None
    };

    Ok(Json(SubscriptionResponse::from_subscription(sub, checkout_url)))
}

pub async fn get_subscription(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<SubscriptionResponse>> {
    let sub: Subscription = sqlx::query_as(
        "SELECT * FROM subscriptions WHERE id = $1 AND user_id = $2",
    )
    .bind(id)
    .bind(auth.id)
    .fetch_optional(state.db.pool())
    .await?
    .ok_or(ApiError::NotFound)?;

    Ok(Json(sub.into()))
}

pub async fn cancel_subscription(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    let sub: Subscription = sqlx::query_as(
        "SELECT * FROM subscriptions WHERE id = $1 AND user_id = $2",
    )
    .bind(id)
    .bind(auth.id)
    .fetch_optional(state.db.pool())
    .await?
    .ok_or(ApiError::NotFound)?;

    let provider_type = ProviderType::from_str_opt(&sub.provider)
        .ok_or_else(|| ApiError::Internal(format!("Unknown provider: {}", sub.provider)))?;

    let provider = state
        .providers
        .get(&provider_type)
        .ok_or_else(|| ApiError::ProviderNotConfigured(sub.provider.clone()))?;

    // For Coinbase, cancel is local-only; for Paddle, calls external API
    if let Some(provider_sub_id) = sub.provider_subscription_id.as_deref() {
        let _ = provider.cancel_subscription(provider_sub_id).await;
    }

    sqlx::query("UPDATE subscriptions SET status = 'cancelled', updated_at = NOW() WHERE id = $1")
        .bind(id)
        .execute(state.db.pool())
        .await?;

    Ok(Json(serde_json::json!({ "status": "cancelled" })))
}
