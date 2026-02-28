use axum::{
    Json,
    body::Bytes,
    extract::{Path, State},
    http::HeaderMap,
};
use uuid::Uuid;

use crate::balance_client::{self, DepositRequest};
use crate::error::{ApiError, ApiResult};
use crate::models::{ParsedWebhookEvent, Payment};
use crate::types::ProviderType;
use crate::AppState;

pub async fn handle_webhook(
    State(state): State<AppState>,
    Path(provider_name): Path<String>,
    headers: HeaderMap,
    body: Bytes,
) -> ApiResult<Json<serde_json::Value>> {
    let provider_type = ProviderType::from_str_opt(&provider_name)
        .ok_or_else(|| ApiError::BadRequest(format!("Unknown provider: {provider_name}")))?;

    let provider = state
        .providers
        .get(&provider_type)
        .ok_or_else(|| ApiError::ProviderNotConfigured(provider_name.clone()))?;

    let signature = extract_signature(&headers, &provider_type)
        .ok_or_else(|| ApiError::BadRequest("Missing webhook signature".to_string()))?;

    if !provider.verify_webhook(&body, &signature)? {
        return Err(ApiError::BadRequest("Invalid webhook signature".to_string()));
    }

    let payload_json: serde_json::Value = serde_json::from_slice(&body)
        .map_err(|e| ApiError::BadRequest(format!("Invalid JSON: {e}")))?;

    let provider_event_id = extract_event_id(&payload_json, &provider_type)
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    // Idempotency: skip if already processed
    let existing = sqlx::query_scalar::<_, bool>(
        "SELECT processed FROM webhook_events WHERE provider = $1 AND provider_event_id = $2",
    )
    .bind(provider_type.to_string())
    .bind(&provider_event_id)
    .fetch_optional(state.db.pool())
    .await?;

    if existing.is_some() {
        return Ok(Json(serde_json::json!({ "status": "already_processed" })));
    }

    let event = provider.parse_webhook(&body)?;
    let event_type = describe_event(&event);

    // Store webhook event
    let event_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO webhook_events (id, provider, provider_event_id, event_type, payload, processed)
         VALUES ($1, $2, $3, $4, $5, false)"
    )
    .bind(event_id)
    .bind(provider_type.to_string())
    .bind(&provider_event_id)
    .bind(&event_type)
    .bind(&payload_json)
    .execute(state.db.pool())
    .await?;

    // Process event
    process_event(&state, &event).await?;

    // Mark as processed
    sqlx::query("UPDATE webhook_events SET processed = true WHERE id = $1")
        .bind(event_id)
        .execute(state.db.pool())
        .await?;

    Ok(Json(serde_json::json!({ "status": "ok" })))
}

fn extract_signature(headers: &HeaderMap, provider: &ProviderType) -> Option<String> {
    let header_name = match provider {
        ProviderType::Coinbase => "X-CC-Webhook-Signature",
        ProviderType::Paddle => "Paddle-Signature",
    };

    headers
        .get(header_name)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
}

fn extract_event_id(json: &serde_json::Value, provider: &ProviderType) -> Option<String> {
    match provider {
        ProviderType::Coinbase => json["event"]["id"].as_str().map(|s| s.to_string()),
        ProviderType::Paddle => json["event_id"].as_str().map(|s| s.to_string()),
    }
}

fn describe_event(event: &ParsedWebhookEvent) -> String {
    match event {
        ParsedWebhookEvent::PaymentCompleted { .. } => "payment.completed".to_string(),
        ParsedWebhookEvent::PaymentFailed { .. } => "payment.failed".to_string(),
        ParsedWebhookEvent::SubscriptionUpdated { .. } => "subscription.updated".to_string(),
        ParsedWebhookEvent::SubscriptionCancelled { .. } => "subscription.cancelled".to_string(),
        ParsedWebhookEvent::Unknown { event_type } => event_type.clone(),
    }
}

async fn process_event(state: &AppState, event: &ParsedWebhookEvent) -> ApiResult<()> {
    match event {
        ParsedWebhookEvent::PaymentCompleted {
            provider_payment_id,
            status,
        } => {
            sqlx::query(
                "UPDATE payments SET status = $1, updated_at = NOW() WHERE provider_payment_id = $2",
            )
            .bind(status.to_string())
            .bind(provider_payment_id)
            .execute(state.db.pool())
            .await?;

            // If this payment is linked to a subscription, activate it and deposit to balance
            handle_subscription_payment(state, provider_payment_id).await?;
        }
        ParsedWebhookEvent::PaymentFailed {
            provider_payment_id,
            status,
        } => {
            sqlx::query(
                "UPDATE payments SET status = $1, updated_at = NOW() WHERE provider_payment_id = $2",
            )
            .bind(status.to_string())
            .bind(provider_payment_id)
            .execute(state.db.pool())
            .await?;
        }
        ParsedWebhookEvent::SubscriptionUpdated {
            provider_subscription_id,
            status,
        } => {
            sqlx::query(
                "UPDATE subscriptions SET status = $1, updated_at = NOW() WHERE provider_subscription_id = $2",
            )
            .bind(status.to_string())
            .bind(provider_subscription_id)
            .execute(state.db.pool())
            .await?;
        }
        ParsedWebhookEvent::SubscriptionCancelled {
            provider_subscription_id,
        } => {
            sqlx::query(
                "UPDATE subscriptions SET status = 'cancelled', updated_at = NOW() WHERE provider_subscription_id = $1",
            )
            .bind(provider_subscription_id)
            .execute(state.db.pool())
            .await?;
        }
        ParsedWebhookEvent::Unknown { event_type } => {
            tracing::debug!("Unhandled webhook event type: {event_type}");
        }
    }
    Ok(())
}

async fn handle_subscription_payment(
    state: &AppState,
    provider_payment_id: &str,
) -> ApiResult<()> {
    let payment: Option<Payment> = sqlx::query_as(
        "SELECT * FROM payments WHERE provider_payment_id = $1 AND subscription_id IS NOT NULL",
    )
    .bind(provider_payment_id)
    .fetch_optional(state.db.pool())
    .await?;

    let payment = match payment {
        Some(p) => p,
        None => return Ok(()),
    };

    let subscription_id = match payment.subscription_id {
        Some(id) => id,
        None => return Ok(()),
    };

    // Activate the subscription
    sqlx::query(
        "UPDATE subscriptions SET status = 'active', current_period_start = NOW(), updated_at = NOW() WHERE id = $1",
    )
    .bind(subscription_id)
    .execute(state.db.pool())
    .await?;

    tracing::info!(
        subscription_id = %subscription_id,
        payment_id = %payment.id,
        amount_cents = payment.amount_cents,
        "Subscription activated via crypto payment"
    );

    // Deposit to internal balance if configured
    let balance_url = match state.config.balance_api_url.as_deref() {
        Some(url) => url,
        None => {
            tracing::debug!("BALANCE_API_URL not configured, skipping balance deposit");
            return Ok(());
        }
    };

    // Convert cents to microtokens (1 cent = 10,000 microtokens, since 1 dollar = 1,000,000)
    let microtokens = payment.amount_cents * 10_000;

    let deposit_req = DepositRequest {
        user_id: payment.user_id,
        amount: microtokens,
        description: Some(format!(
            "Crypto subscription payment: {} {}",
            payment.amount_cents as f64 / 100.0,
            payment.currency
        )),
        reference_type: Some("coinbase_subscription".to_string()),
        reference_id: Some(payment.id.to_string()),
        idempotency_key: Some(format!("payment:{}", payment.id)),
    };

    if let Err(e) = balance_client::deposit(&state.http_client, balance_url, &deposit_req).await {
        tracing::error!(
            payment_id = %payment.id,
            error = %e,
            "Failed to deposit to balance after subscription payment"
        );
        return Err(ApiError::Internal(format!("Balance deposit failed: {e}")));
    }

    tracing::info!(
        user_id = %payment.user_id,
        microtokens = microtokens,
        "Deposited to internal balance from subscription payment"
    );

    Ok(())
}
