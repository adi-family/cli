use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use balance_api_core::ApiError;

pub struct HttpError(pub ApiError);

pub type HttpResult<T> = Result<T, HttpError>;

impl From<ApiError> for HttpError {
    fn from(err: ApiError) -> Self {
        HttpError(err)
    }
}

impl From<sqlx::Error> for HttpError {
    fn from(err: sqlx::Error) -> Self {
        HttpError(ApiError::Database(err))
    }
}

impl IntoResponse for HttpError {
    fn into_response(self) -> Response {
        let (status, message) = match &self.0 {
            ApiError::NotFound => (StatusCode::NOT_FOUND, self.0.to_string()),
            ApiError::Unauthorized => (StatusCode::UNAUTHORIZED, self.0.to_string()),
            ApiError::Forbidden => (StatusCode::FORBIDDEN, self.0.to_string()),
            ApiError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
            ApiError::Conflict(msg) => (StatusCode::CONFLICT, msg.clone()),
            ApiError::InsufficientBalance => (StatusCode::PAYMENT_REQUIRED, self.0.to_string()),
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

        let body = serde_json::json!({
            "error": message,
        });

        (status, Json(body)).into_response()
    }
}
