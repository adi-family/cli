use axum::{Json, extract::State};

use crate::auth::AuthUser;
use crate::error::{ApiError, ApiResult};
use crate::models::{CheckoutSession, CreateCheckoutRequest};
use crate::AppState;

pub async fn create_checkout(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<CreateCheckoutRequest>,
) -> ApiResult<Json<CheckoutSession>> {
    let provider = state
        .providers
        .get(&req.provider)
        .ok_or_else(|| ApiError::ProviderNotConfigured(req.provider.to_string()))?;

    let conversion_rate = req
        .conversion_rate
        .unwrap_or(state.config.default_conversion_rate);
    let expected_credits = (req.amount_cents as f64 * conversion_rate).round() as i64;

    let mut session = provider.create_checkout(&req).await?;
    session.conversion_rate = conversion_rate;
    session.expected_credits = expected_credits;

    sqlx::query(
        "INSERT INTO payments (id, provider, provider_payment_id, user_id, amount_cents, currency, conversion_rate, status, checkout_url, metadata)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)"
    )
    .bind(session.id)
    .bind(&session.provider)
    .bind(&session.provider_payment_id)
    .bind(auth.id)
    .bind(req.amount_cents)
    .bind(&req.currency)
    .bind(conversion_rate)
    .bind(&session.status)
    .bind(&session.checkout_url)
    .bind(&req.metadata)
    .execute(state.db.pool())
    .await?;

    Ok(Json(session))
}
