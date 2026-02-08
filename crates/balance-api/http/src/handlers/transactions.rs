use axum::{
    Json,
    extract::{Path, Query, State},
};
use lib_analytics_core::AnalyticsEvent;
use uuid::Uuid;

use balance_api_core::{
    ApiError, Balance, CheckBalanceRequest, CheckBalanceResponse, DebitRequest, DepositRequest,
    Transaction, TransactionQuery, TransactionResponse,
};
use crate::{
    AppState,
    auth::AuthUser,
    error::HttpResult,
};

pub async fn deposit(
    State(state): State<AppState>,
    Json(input): Json<DepositRequest>,
) -> HttpResult<Json<TransactionResponse>> {
    if input.amount <= 0 {
        return Err(ApiError::BadRequest("Amount must be positive".into()).into());
    }

    if let Some(key) = &input.idempotency_key {
        let existing = sqlx::query_as::<_, Transaction>(
            "SELECT * FROM transactions WHERE user_id = $1 AND idempotency_key = $2",
        )
        .bind(input.user_id)
        .bind(key)
        .fetch_optional(state.db.pool())
        .await?;

        if let Some(tx) = existing {
            return Ok(Json(TransactionResponse::from(tx)));
        }
    }

    let mut db_tx = state.db.pool().begin().await?;

    let balance =
        sqlx::query_as::<_, Balance>("SELECT * FROM balances WHERE user_id = $1 FOR UPDATE")
            .bind(input.user_id)
            .fetch_optional(&mut *db_tx)
            .await?
            .ok_or(ApiError::NotFound)?;

    let new_amount = balance.amount + input.amount;

    sqlx::query_as::<_, Balance>(
        r#"
        UPDATE balances
        SET amount = $1, version = version + 1, updated_at = NOW()
        WHERE id = $2 AND version = $3
        RETURNING *
        "#,
    )
    .bind(new_amount)
    .bind(balance.id)
    .bind(balance.version)
    .fetch_optional(&mut *db_tx)
    .await?
    .ok_or(ApiError::Conflict(
        "Balance was modified concurrently".into(),
    ))?;

    let metadata = if input.metadata.is_null() {
        serde_json::json!({})
    } else {
        input.metadata
    };

    let transaction = sqlx::query_as::<_, Transaction>(
        r#"
        INSERT INTO transactions
            (user_id, balance_id, transaction_type, status, amount,
             balance_before, balance_after, description, reference_type,
             reference_id, idempotency_key, metadata)
        VALUES ($1, $2, 'deposit', 'completed', $3, $4, $5, $6, $7, $8, $9, $10)
        RETURNING *
        "#,
    )
    .bind(input.user_id)
    .bind(balance.id)
    .bind(input.amount)
    .bind(balance.amount)
    .bind(new_amount)
    .bind(&input.description)
    .bind(&input.reference_type)
    .bind(&input.reference_id)
    .bind(&input.idempotency_key)
    .bind(&metadata)
    .fetch_one(&mut *db_tx)
    .await?;

    db_tx.commit().await?;

    state.analytics.track(AnalyticsEvent::BalanceDeposit {
        user_id: input.user_id,
        transaction_id: transaction.id,
        amount: input.amount,
        reference_type: input.reference_type.clone(),
    });

    Ok(Json(TransactionResponse::from(transaction)))
}

pub async fn debit(
    State(state): State<AppState>,
    Json(input): Json<DebitRequest>,
) -> HttpResult<Json<TransactionResponse>> {
    if input.amount <= 0 {
        return Err(ApiError::BadRequest("Amount must be positive".into()).into());
    }

    if let Some(key) = &input.idempotency_key {
        let existing = sqlx::query_as::<_, Transaction>(
            "SELECT * FROM transactions WHERE user_id = $1 AND idempotency_key = $2",
        )
        .bind(input.user_id)
        .bind(key)
        .fetch_optional(state.db.pool())
        .await?;

        if let Some(tx) = existing {
            return Ok(Json(TransactionResponse::from(tx)));
        }
    }

    let mut db_tx = state.db.pool().begin().await?;

    let balance =
        sqlx::query_as::<_, Balance>("SELECT * FROM balances WHERE user_id = $1 FOR UPDATE")
            .bind(input.user_id)
            .fetch_optional(&mut *db_tx)
            .await?
            .ok_or(ApiError::NotFound)?;

    if balance.amount < input.amount {
        state.analytics.track(AnalyticsEvent::BalanceInsufficient {
            user_id: input.user_id,
            requested_amount: input.amount,
            current_balance: balance.amount,
            reference_type: input.reference_type.clone(),
        });
        return Err(ApiError::InsufficientBalance.into());
    }

    let new_amount = balance.amount - input.amount;

    sqlx::query_as::<_, Balance>(
        r#"
        UPDATE balances
        SET amount = $1, version = version + 1, updated_at = NOW()
        WHERE id = $2 AND version = $3
        RETURNING *
        "#,
    )
    .bind(new_amount)
    .bind(balance.id)
    .bind(balance.version)
    .fetch_optional(&mut *db_tx)
    .await?
    .ok_or(ApiError::Conflict(
        "Balance was modified concurrently".into(),
    ))?;

    let metadata = if input.metadata.is_null() {
        serde_json::json!({})
    } else {
        input.metadata
    };

    let transaction = sqlx::query_as::<_, Transaction>(
        r#"
        INSERT INTO transactions
            (user_id, balance_id, transaction_type, status, amount,
             balance_before, balance_after, description, reference_type,
             reference_id, idempotency_key, metadata)
        VALUES ($1, $2, 'debit', 'completed', $3, $4, $5, $6, $7, $8, $9, $10)
        RETURNING *
        "#,
    )
    .bind(input.user_id)
    .bind(balance.id)
    .bind(-input.amount)
    .bind(balance.amount)
    .bind(new_amount)
    .bind(&input.description)
    .bind(&input.reference_type)
    .bind(&input.reference_id)
    .bind(&input.idempotency_key)
    .bind(&metadata)
    .fetch_one(&mut *db_tx)
    .await?;

    db_tx.commit().await?;

    state.analytics.track(AnalyticsEvent::BalanceDebit {
        user_id: input.user_id,
        transaction_id: transaction.id,
        amount: input.amount,
        reference_type: input.reference_type.clone(),
    });

    Ok(Json(TransactionResponse::from(transaction)))
}

pub async fn check_balance(
    State(state): State<AppState>,
    Json(input): Json<CheckBalanceRequest>,
) -> HttpResult<Json<CheckBalanceResponse>> {
    let balance = sqlx::query_as::<_, Balance>("SELECT * FROM balances WHERE user_id = $1")
        .bind(input.user_id)
        .fetch_optional(state.db.pool())
        .await?
        .ok_or(ApiError::NotFound)?;

    let sufficient = balance.amount >= input.amount;
    let shortfall = if sufficient {
        None
    } else {
        Some(input.amount - balance.amount)
    };

    Ok(Json(CheckBalanceResponse {
        sufficient,
        current_balance: balance.amount,
        required_amount: input.amount,
        shortfall,
    }))
}

pub async fn list_transactions(
    State(state): State<AppState>,
    user: AuthUser,
    Query(query): Query<TransactionQuery>,
) -> HttpResult<Json<Vec<TransactionResponse>>> {
    let limit = query.limit.unwrap_or(50).min(100);
    let offset = query.offset.unwrap_or(0);

    let transactions = sqlx::query_as::<_, Transaction>(
        r#"
        SELECT * FROM transactions
        WHERE user_id = $1
        ORDER BY created_at DESC
        LIMIT $2 OFFSET $3
        "#,
    )
    .bind(user.id)
    .bind(limit)
    .bind(offset)
    .fetch_all(state.db.pool())
    .await?;

    Ok(Json(
        transactions
            .into_iter()
            .map(TransactionResponse::from)
            .collect(),
    ))
}

pub async fn get_transaction(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> HttpResult<Json<TransactionResponse>> {
    let transaction = sqlx::query_as::<_, Transaction>(
        "SELECT * FROM transactions WHERE id = $1 AND user_id = $2",
    )
    .bind(id)
    .bind(user.id)
    .fetch_optional(state.db.pool())
    .await?
    .ok_or(ApiError::NotFound)?;

    Ok(Json(TransactionResponse::from(transaction)))
}
