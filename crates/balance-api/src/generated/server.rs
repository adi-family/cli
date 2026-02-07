//! Auto-generated server handlers from TypeSpec.
//! DO NOT EDIT.
//!
//! Implement the handler traits and use the generated router.

#![allow(unused_imports)]

use super::models::*;
use super::enums::*;
use async_trait::async_trait;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{delete, get, patch, post, put};
use axum::{Json, Router};
use serde::Deserialize;
use uuid::Uuid;


#[derive(Debug, serde::Serialize)]
pub struct ApiError {
    pub status: u16,
    pub code: String,
    pub message: String,
}

impl axum::response::IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        let status = StatusCode::from_u16(self.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        (status, Json(self)).into_response()
    }
}


#[async_trait]
pub trait BalanceServiceHandler: Send + Sync + 'static {
    async fn get_my_balance(&self) -> Result<BalanceResponse, ApiError>;
    async fn init_balance(&self, body: InitBalanceRequest) -> Result<BalanceResponse, ApiError>;
    async fn get_balance_by_user(&self, user_id: Uuid) -> Result<BalanceResponse, ApiError>;
}

async fn balance_service_get_my_balance<S: BalanceServiceHandler>(
    State(state): State<S>,
) -> Result<Json<BalanceResponse>, ApiError> {
    let result = state.get_my_balance().await?;
    Ok(Json(result))
}

async fn balance_service_init_balance<S: BalanceServiceHandler>(
    State(state): State<S>,
    Json(body): Json<InitBalanceRequest>,
) -> Result<(StatusCode, Json<BalanceResponse>), ApiError> {
    let result = state.init_balance(body).await?;
    Ok((StatusCode::CREATED, Json(result)))
}

async fn balance_service_get_balance_by_user<S: BalanceServiceHandler>(
    State(state): State<S>,
    Path(user_id): Path<Uuid>,
) -> Result<Json<BalanceResponse>, ApiError> {
    let result = state.get_balance_by_user(user_id).await?;
    Ok(Json(result))
}

pub fn balance_service_routes<S: BalanceServiceHandler + Clone + 'static>() -> Router<S> {
    Router::new()
        .route("/balances/me", get(balance_service_get_my_balance::<S>))
        .route("/balances/init", post(balance_service_init_balance::<S>))
        .route("/balances/{userId}", get(balance_service_get_balance_by_user::<S>))
}

#[async_trait]
pub trait TransactionServiceHandler: Send + Sync + 'static {
    async fn list(&self, query: TransactionServiceListQuery) -> Result<Vec<TransactionResponse>, ApiError>;
    async fn deposit(&self, body: DepositRequest) -> Result<TransactionResponse, ApiError>;
    async fn debit(&self, body: DebitRequest) -> Result<TransactionResponse, ApiError>;
    async fn check_balance(&self, body: CheckBalanceRequest) -> Result<CheckBalanceResponse, ApiError>;
    async fn get_transaction(&self, id: Uuid) -> Result<TransactionResponse, ApiError>;
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionServiceListQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub transaction_type: Option<String>,
    pub reference_type: Option<String>,
}

async fn transaction_service_list<S: TransactionServiceHandler>(
    State(state): State<S>,
    Query(query): Query<TransactionServiceListQuery>,
) -> Result<Json<Vec<TransactionResponse>>, ApiError> {
    let result = state.list(query).await?;
    Ok(Json(result))
}

async fn transaction_service_deposit<S: TransactionServiceHandler>(
    State(state): State<S>,
    Json(body): Json<DepositRequest>,
) -> Result<(StatusCode, Json<TransactionResponse>), ApiError> {
    let result = state.deposit(body).await?;
    Ok((StatusCode::CREATED, Json(result)))
}

async fn transaction_service_debit<S: TransactionServiceHandler>(
    State(state): State<S>,
    Json(body): Json<DebitRequest>,
) -> Result<(StatusCode, Json<TransactionResponse>), ApiError> {
    let result = state.debit(body).await?;
    Ok((StatusCode::CREATED, Json(result)))
}

async fn transaction_service_check_balance<S: TransactionServiceHandler>(
    State(state): State<S>,
    Json(body): Json<CheckBalanceRequest>,
) -> Result<Json<CheckBalanceResponse>, ApiError> {
    let result = state.check_balance(body).await?;
    Ok(Json(result))
}

async fn transaction_service_get_transaction<S: TransactionServiceHandler>(
    State(state): State<S>,
    Path(id): Path<Uuid>,
) -> Result<Json<TransactionResponse>, ApiError> {
    let result = state.get_transaction(id).await?;
    Ok(Json(result))
}

pub fn transaction_service_routes<S: TransactionServiceHandler + Clone + 'static>() -> Router<S> {
    Router::new()
        .route("/transactions", get(transaction_service_list::<S>))
        .route("/transactions/deposit", post(transaction_service_deposit::<S>))
        .route("/transactions/debit", post(transaction_service_debit::<S>))
        .route("/transactions/check", post(transaction_service_check_balance::<S>))
        .route("/transactions/{id}", get(transaction_service_get_transaction::<S>))
}

pub fn create_router<S: BalanceServiceHandler + TransactionServiceHandler + Clone + 'static>() -> Router<S> {
    Router::new()
        .merge(balance_service_routes())
        .merge(transaction_service_routes())
}
