use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use thiserror::Error;

/// API error type with HTTP status mapping.
#[derive(Error, Debug)]
pub enum ApiError {
    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Unauthorized")]
    Unauthorized,

    #[error("Forbidden: {0}")]
    Forbidden(String),

    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Conflict: {0}")]
    Conflict(String),

    #[error("Rate limited")]
    RateLimited,

    #[error("Upstream error: {0}")]
    UpstreamError(String),

    #[error("Encryption error: {0}")]
    EncryptionError(String),

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl ApiError {
    pub fn status_code(&self) -> StatusCode {
        match self {
            ApiError::NotFound(_) => StatusCode::NOT_FOUND,
            ApiError::Unauthorized => StatusCode::UNAUTHORIZED,
            ApiError::Forbidden(_) => StatusCode::FORBIDDEN,
            ApiError::BadRequest(_) => StatusCode::BAD_REQUEST,
            ApiError::Conflict(_) => StatusCode::CONFLICT,
            ApiError::RateLimited => StatusCode::TOO_MANY_REQUESTS,
            ApiError::UpstreamError(_) => StatusCode::BAD_GATEWAY,
            ApiError::EncryptionError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ApiError::Database(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ApiError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    pub fn error_type(&self) -> &'static str {
        match self {
            ApiError::NotFound(_) => "not_found",
            ApiError::Unauthorized => "unauthorized",
            ApiError::Forbidden(_) => "forbidden",
            ApiError::BadRequest(_) => "bad_request",
            ApiError::Conflict(_) => "conflict",
            ApiError::RateLimited => "rate_limited",
            ApiError::UpstreamError(_) => "upstream_error",
            ApiError::EncryptionError(_) => "encryption_error",
            ApiError::Database(_) => "database_error",
            ApiError::Internal(_) => "internal_error",
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = self.status_code();
        let error_type = self.error_type();
        let message = self.to_string();

        let mut error = serde_json::json!({
            "type": error_type,
            "message": message
        });

        if matches!(self, ApiError::Unauthorized) {
            if let Some(domain) = auth_domain() {
                error["auth_kind"] = serde_json::json!("adi.auth");
                error["auth_domain"] = serde_json::Value::String(domain.clone());
                error["auth_options"] = serde_json::json!(["verified"]);
            }
        }

        let body = Json(serde_json::json!({ "error": error }));
        (status, body).into_response()
    }
}

fn auth_domain() -> Option<&'static String> {
    use std::sync::OnceLock;
    static AUTH_DOMAIN: OnceLock<Option<String>> = OnceLock::new();
    AUTH_DOMAIN
        .get_or_init(|| std::env::var("AUTH_DOMAIN").ok())
        .as_ref()
}

pub type ApiResult<T> = Result<T, ApiError>;

impl From<anyhow::Error> for ApiError {
    fn from(err: anyhow::Error) -> Self {
        ApiError::Internal(err.to_string())
    }
}

impl From<ApiError> for lib_adi_service::AdiServiceError {
    fn from(e: ApiError) -> Self {
        use lib_adi_service::AdiServiceError;
        match e {
            ApiError::NotFound(m) => AdiServiceError::not_found(m),
            ApiError::BadRequest(m) => AdiServiceError::invalid_params(m),
            ApiError::Conflict(m) => AdiServiceError::invalid_params(m),
            ApiError::Unauthorized => {
                AdiServiceError::new("unauthorized".to_string(), "Unauthorized".to_string())
            }
            ApiError::Forbidden(m) => AdiServiceError::new("forbidden".to_string(), m),
            _ => AdiServiceError::internal(e.to_string()),
        }
    }
}
