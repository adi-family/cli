use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Balance {
    pub user_id: Uuid,
    pub credits: i64,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct BalanceTransaction {
    pub id: Uuid,
    pub user_id: Uuid,
    pub payment_id: Option<Uuid>,
    pub transaction_type: String,
    pub amount: i64,
    pub balance_before: i64,
    pub balance_after: i64,
    pub conversion_rate: f64,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct BalanceResponse {
    pub credits: i64,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct BalanceTransactionResponse {
    pub id: Uuid,
    pub payment_id: Option<Uuid>,
    pub transaction_type: String,
    pub amount: i64,
    pub balance_before: i64,
    pub balance_after: i64,
    pub conversion_rate: f64,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl From<BalanceTransaction> for BalanceTransactionResponse {
    fn from(t: BalanceTransaction) -> Self {
        Self {
            id: t.id,
            payment_id: t.payment_id,
            transaction_type: t.transaction_type,
            amount: t.amount,
            balance_before: t.balance_before,
            balance_after: t.balance_after,
            conversion_rate: t.conversion_rate,
            description: t.description,
            created_at: t.created_at,
        }
    }
}

pub async fn get_or_create_balance(pool: &PgPool, user_id: Uuid) -> ApiResult<Balance> {
    let balance: Option<Balance> =
        sqlx::query_as("SELECT * FROM balances WHERE user_id = $1")
            .bind(user_id)
            .fetch_optional(pool)
            .await?;

    match balance {
        Some(b) => Ok(b),
        None => {
            sqlx::query(
                "INSERT INTO balances (user_id, credits) VALUES ($1, 0) ON CONFLICT (user_id) DO NOTHING",
            )
            .bind(user_id)
            .execute(pool)
            .await?;

            sqlx::query_as("SELECT * FROM balances WHERE user_id = $1")
                .bind(user_id)
                .fetch_one(pool)
                .await
                .map_err(ApiError::from)
        }
    }
}

pub async fn deposit(
    pool: &PgPool,
    user_id: Uuid,
    payment_id: Uuid,
    amount_cents: i64,
    conversion_rate: f64,
    description: &str,
) -> ApiResult<BalanceTransaction> {
    let credits = (amount_cents as f64 * conversion_rate).round() as i64;

    let balance = get_or_create_balance(pool, user_id).await?;
    let balance_before = balance.credits;
    let balance_after = balance_before + credits;

    // Atomic: update balance + insert transaction
    sqlx::query("UPDATE balances SET credits = $1, updated_at = NOW() WHERE user_id = $2")
        .bind(balance_after)
        .bind(user_id)
        .execute(pool)
        .await?;

    let txn_id = Uuid::new_v4();
    let txn: BalanceTransaction = sqlx::query_as(
        "INSERT INTO balance_transactions (id, user_id, payment_id, transaction_type, amount, balance_before, balance_after, conversion_rate, description)
         VALUES ($1, $2, $3, 'deposit', $4, $5, $6, $7, $8)
         RETURNING *"
    )
    .bind(txn_id)
    .bind(user_id)
    .bind(payment_id)
    .bind(credits)
    .bind(balance_before)
    .bind(balance_after)
    .bind(conversion_rate)
    .bind(description)
    .fetch_one(pool)
    .await?;

    tracing::info!(
        user_id = %user_id,
        payment_id = %payment_id,
        amount_cents = amount_cents,
        conversion_rate = conversion_rate,
        credits = credits,
        balance_after = balance_after,
        "Balance deposit completed"
    );

    Ok(txn)
}
