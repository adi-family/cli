use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CreditPool {
    Subscription,
    Extra,
}

impl std::fmt::Display for CreditPool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Subscription => write!(f, "subscription"),
            Self::Extra => write!(f, "extra"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Balance {
    pub user_id: Uuid,
    pub subscription_credits: i64,
    pub extra_credits: i64,
    pub updated_at: DateTime<Utc>,
}

impl Balance {
    pub fn total(&self) -> i64 {
        self.subscription_credits + self.extra_credits
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct BalanceTransaction {
    pub id: Uuid,
    pub user_id: Uuid,
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

#[derive(Debug, Serialize)]
pub struct BalanceResponse {
    pub subscription_credits: i64,
    pub extra_credits: i64,
    pub total_credits: i64,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
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

impl From<BalanceTransaction> for BalanceTransactionResponse {
    fn from(t: BalanceTransaction) -> Self {
        Self {
            id: t.id,
            payment_id: t.payment_id,
            transaction_type: t.transaction_type,
            pool: t.pool,
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
                "INSERT INTO balances (user_id, subscription_credits, extra_credits) VALUES ($1, 0, 0) ON CONFLICT (user_id) DO NOTHING",
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
    credit_pool: CreditPool,
    amount_cents: i64,
    conversion_rate: f64,
    description: &str,
) -> ApiResult<BalanceTransaction> {
    let credits = (amount_cents as f64 * conversion_rate).round() as i64;

    let balance = get_or_create_balance(pool, user_id).await?;
    let balance_before = balance.total();

    let update_sql = match credit_pool {
        CreditPool::Subscription => {
            "UPDATE balances SET subscription_credits = subscription_credits + $1, updated_at = NOW() WHERE user_id = $2"
        }
        CreditPool::Extra => {
            "UPDATE balances SET extra_credits = extra_credits + $1, updated_at = NOW() WHERE user_id = $2"
        }
    };

    sqlx::query(update_sql)
        .bind(credits)
        .bind(user_id)
        .execute(pool)
        .await?;

    let balance_after = balance_before + credits;

    let txn_id = Uuid::new_v4();
    let txn: BalanceTransaction = sqlx::query_as(
        "INSERT INTO balance_transactions (id, user_id, payment_id, transaction_type, pool, amount, balance_before, balance_after, conversion_rate, description)
         VALUES ($1, $2, $3, 'deposit', $4, $5, $6, $7, $8, $9)
         RETURNING *"
    )
    .bind(txn_id)
    .bind(user_id)
    .bind(payment_id)
    .bind(credit_pool.to_string())
    .bind(credits)
    .bind(balance_before)
    .bind(balance_after)
    .bind(conversion_rate)
    .bind(description)
    .fetch_one(pool)
    .await?;

    tracing::info!(
        user_id = %user_id,
        pool = %credit_pool,
        credits = credits,
        balance_after = balance_after,
        "Deposit completed"
    );

    Ok(txn)
}

/// Debit credits from the user's balance.
/// Draws from subscription pool first, then extra pool.
pub async fn debit(
    pool: &PgPool,
    user_id: Uuid,
    amount: i64,
    description: &str,
) -> ApiResult<Vec<BalanceTransaction>> {
    if amount <= 0 {
        return Err(ApiError::BadRequest("Debit amount must be positive".to_string()));
    }

    let balance = get_or_create_balance(pool, user_id).await?;

    if balance.total() < amount {
        return Err(ApiError::InsufficientBalance);
    }

    let balance_before = balance.total();
    let mut remaining = amount;
    let mut txns = Vec::new();

    // 1. Draw from subscription credits first
    if remaining > 0 && balance.subscription_credits > 0 {
        let from_sub = remaining.min(balance.subscription_credits);
        remaining -= from_sub;

        sqlx::query(
            "UPDATE balances SET subscription_credits = subscription_credits - $1, updated_at = NOW() WHERE user_id = $2",
        )
        .bind(from_sub)
        .bind(user_id)
        .execute(pool)
        .await?;

        let after = balance_before - from_sub;
        let txn: BalanceTransaction = sqlx::query_as(
            "INSERT INTO balance_transactions (id, user_id, transaction_type, pool, amount, balance_before, balance_after, conversion_rate, description)
             VALUES ($1, $2, 'debit', 'subscription', $3, $4, $5, 0, $6)
             RETURNING *"
        )
        .bind(Uuid::new_v4())
        .bind(user_id)
        .bind(-from_sub)
        .bind(balance_before)
        .bind(after)
        .bind(description)
        .fetch_one(pool)
        .await?;

        txns.push(txn);
    }

    // 2. Draw remainder from extra credits
    if remaining > 0 {
        let current_total = balance_before - (amount - remaining);

        sqlx::query(
            "UPDATE balances SET extra_credits = extra_credits - $1, updated_at = NOW() WHERE user_id = $2",
        )
        .bind(remaining)
        .bind(user_id)
        .execute(pool)
        .await?;

        let after = current_total - remaining;
        let txn: BalanceTransaction = sqlx::query_as(
            "INSERT INTO balance_transactions (id, user_id, transaction_type, pool, amount, balance_before, balance_after, conversion_rate, description)
             VALUES ($1, $2, 'debit', 'extra', $3, $4, $5, 0, $6)
             RETURNING *"
        )
        .bind(Uuid::new_v4())
        .bind(user_id)
        .bind(-remaining)
        .bind(current_total)
        .bind(after)
        .bind(description)
        .fetch_one(pool)
        .await?;

        txns.push(txn);
    }

    tracing::info!(
        user_id = %user_id,
        amount = amount,
        balance_after = balance_before - amount,
        "Debit completed"
    );

    Ok(txns)
}
