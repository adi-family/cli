use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::types::{BillingInterval, ProviderType};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Subscription {
    pub id: Uuid,
    pub provider: String,
    pub provider_subscription_id: Option<String>,
    pub user_id: Uuid,
    pub plan_id: String,
    pub status: String,
    pub billing_interval: Option<String>,
    pub amount_cents: Option<i64>,
    pub currency: Option<String>,
    pub current_period_start: Option<DateTime<Utc>>,
    pub current_period_end: Option<DateTime<Utc>>,
    pub metadata: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateSubscriptionRequest {
    pub provider: ProviderType,
    pub plan_id: String,
    pub billing_interval: Option<BillingInterval>,
    pub success_url: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
pub struct SubscriptionResponse {
    pub id: Uuid,
    pub provider: String,
    pub provider_subscription_id: Option<String>,
    pub plan_id: String,
    pub status: String,
    pub billing_interval: Option<String>,
    pub amount_cents: Option<i64>,
    pub currency: Option<String>,
    pub current_period_start: Option<DateTime<Utc>>,
    pub current_period_end: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl From<Subscription> for SubscriptionResponse {
    fn from(s: Subscription) -> Self {
        Self {
            id: s.id,
            provider: s.provider,
            provider_subscription_id: s.provider_subscription_id,
            plan_id: s.plan_id,
            status: s.status,
            billing_interval: s.billing_interval,
            amount_cents: s.amount_cents,
            currency: s.currency,
            current_period_start: s.current_period_start,
            current_period_end: s.current_period_end,
            created_at: s.created_at,
        }
    }
}
