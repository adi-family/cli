use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

pub const MICROTOKENS_PER_TOKEN: i64 = 1_000_000;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Balance {
    pub id: Uuid,
    pub user_id: Uuid,
    pub amount: i64,
    pub currency: String,
    pub version: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct BalanceResponse {
    pub user_id: Uuid,
    pub amount: i64,
    pub amount_formatted: String,
    pub currency: String,
    pub updated_at: DateTime<Utc>,
}

impl From<Balance> for BalanceResponse {
    fn from(b: Balance) -> Self {
        Self {
            user_id: b.user_id,
            amount: b.amount,
            amount_formatted: format!("{:.2}", b.amount as f64 / MICROTOKENS_PER_TOKEN as f64),
            currency: b.currency,
            updated_at: b.updated_at,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct InitBalanceRequest {
    pub user_id: Option<Uuid>,
}
