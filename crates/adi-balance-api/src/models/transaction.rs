use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "transaction_type", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum TransactionType {
    Deposit,
    Debit,
    Adjustment,
    TransferIn,
    TransferOut,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "transaction_status", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum TransactionStatus {
    Pending,
    Completed,
    Failed,
    Reversed,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Transaction {
    pub id: Uuid,
    pub user_id: Uuid,
    pub balance_id: Uuid,
    pub transaction_type: TransactionType,
    pub status: TransactionStatus,
    pub amount: i64,
    pub balance_before: i64,
    pub balance_after: i64,
    pub description: Option<String>,
    pub reference_type: Option<String>,
    pub reference_id: Option<String>,
    pub metadata: serde_json::Value,
    pub idempotency_key: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct TransactionResponse {
    pub id: Uuid,
    pub user_id: Uuid,
    pub transaction_type: TransactionType,
    pub status: TransactionStatus,
    pub amount: i64,
    pub amount_formatted: String,
    pub balance_before: i64,
    pub balance_after: i64,
    pub description: Option<String>,
    pub reference_type: Option<String>,
    pub reference_id: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl From<Transaction> for TransactionResponse {
    fn from(t: Transaction) -> Self {
        Self {
            id: t.id,
            user_id: t.user_id,
            transaction_type: t.transaction_type,
            status: t.status,
            amount: t.amount,
            amount_formatted: format!("{:.2}", t.amount as f64 / 1_000_000.0),
            balance_before: t.balance_before,
            balance_after: t.balance_after,
            description: t.description,
            reference_type: t.reference_type,
            reference_id: t.reference_id,
            created_at: t.created_at,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct DepositRequest {
    pub user_id: Uuid,
    pub amount: i64,
    pub description: Option<String>,
    pub reference_type: Option<String>,
    pub reference_id: Option<String>,
    pub idempotency_key: Option<String>,
    #[serde(default)]
    pub metadata: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct DebitRequest {
    pub user_id: Uuid,
    pub amount: i64,
    pub description: Option<String>,
    pub reference_type: Option<String>,
    pub reference_id: Option<String>,
    pub idempotency_key: Option<String>,
    #[serde(default)]
    pub metadata: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct CheckBalanceRequest {
    pub user_id: Uuid,
    pub amount: i64,
}

#[derive(Debug, Serialize)]
pub struct CheckBalanceResponse {
    pub sufficient: bool,
    pub current_balance: i64,
    pub required_amount: i64,
    pub shortfall: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct TransactionQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub transaction_type: Option<TransactionType>,
    pub reference_type: Option<String>,
}
