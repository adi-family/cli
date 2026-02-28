use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::types::ProviderType;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Payment {
    pub id: Uuid,
    pub provider: String,
    pub provider_payment_id: Option<String>,
    pub user_id: Uuid,
    pub subscription_id: Option<Uuid>,
    pub amount_cents: i64,
    pub currency: String,
    pub conversion_rate: f64,
    pub credited_amount: Option<i64>,
    pub status: String,
    pub checkout_url: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateCheckoutRequest {
    pub provider: ProviderType,
    pub amount_cents: i64,
    pub currency: String,
    pub conversion_rate: Option<f64>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub success_url: Option<String>,
    pub cancel_url: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CheckoutSession {
    pub id: Uuid,
    pub provider: String,
    pub provider_payment_id: String,
    pub checkout_url: String,
    pub conversion_rate: f64,
    pub expected_credits: i64,
    pub status: String,
}
