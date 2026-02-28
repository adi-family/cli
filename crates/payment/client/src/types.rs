use chrono::{DateTime, Utc};
use serde::Deserialize;
use uuid::Uuid;

#[derive(Debug, Clone, Deserialize)]
pub struct BalanceResponse {
    pub subscription_credits: i64,
    pub extra_credits: i64,
    pub total_credits: i64,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BalanceTransactionResponse {
    pub id: Uuid,
    pub payment_id: Option<Uuid>,
    pub transaction_type: String,
    pub pool: String,
    pub amount: i64,
    pub balance_before: i64,
    pub balance_after: i64,
    pub conversion_rate: f64,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
}
