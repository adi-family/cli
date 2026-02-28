use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::types::{PaymentStatus, SubscriptionStatus};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct WebhookEvent {
    pub id: Uuid,
    pub provider: String,
    pub provider_event_id: String,
    pub event_type: String,
    pub payload: serde_json::Value,
    pub processed: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug)]
pub enum ParsedWebhookEvent {
    PaymentCompleted {
        provider_payment_id: String,
        status: PaymentStatus,
    },
    PaymentFailed {
        provider_payment_id: String,
        status: PaymentStatus,
    },
    SubscriptionUpdated {
        provider_subscription_id: String,
        status: SubscriptionStatus,
    },
    SubscriptionCancelled {
        provider_subscription_id: String,
    },
    Unknown {
        event_type: String,
    },
}
