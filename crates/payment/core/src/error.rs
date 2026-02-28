use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ApiError {
    #[error("Not found")]
    NotFound,

    #[error("Unauthorized")]
    Unauthorized,

    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Provider error: {0}")]
    Provider(String),

    #[error("Provider not configured: {0}")]
    ProviderNotConfigured(String),

    #[error("Not supported: {0}")]
    NotSupported(String),

    #[error("Insufficient balance")]
    InsufficientBalance,

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Internal error: {0}")]
    Internal(String),
}

pub type ApiResult<T> = Result<T, ApiError>;

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            ApiError::NotFound => (StatusCode::NOT_FOUND, self.to_string()),
            ApiError::Unauthorized => (StatusCode::UNAUTHORIZED, self.to_string()),
            ApiError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
            ApiError::Provider(msg) => (StatusCode::BAD_GATEWAY, msg.clone()),
            ApiError::ProviderNotConfigured(msg) => (StatusCode::SERVICE_UNAVAILABLE, msg.clone()),
            ApiError::NotSupported(msg) => (StatusCode::NOT_IMPLEMENTED, msg.clone()),
            ApiError::InsufficientBalance => (StatusCode::PAYMENT_REQUIRED, self.to_string()),
            ApiError::Database(e) => {
                tracing::error!("Database error: {:?}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Database error".to_string(),
                )
            }
            ApiError::Internal(msg) => {
                tracing::error!("Internal error: {}", msg);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal error".to_string(),
                )
            }
        };

        let body = serde_json::json!({ "error": message });
        (status, Json(body)).into_response()
    }
}
