//! Error types for adi-llm-proxy.

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

    #[error("Transform error: {0}")]
    TransformError(String),

    #[error("Encryption error: {0}")]
    EncryptionError(String),

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl ApiError {
    /// Get the HTTP status code for this error.
    pub fn status_code(&self) -> StatusCode {
        match self {
            ApiError::NotFound(_) => StatusCode::NOT_FOUND,
            ApiError::Unauthorized => StatusCode::UNAUTHORIZED,
            ApiError::Forbidden(_) => StatusCode::FORBIDDEN,
            ApiError::BadRequest(_) => StatusCode::BAD_REQUEST,
            ApiError::Conflict(_) => StatusCode::CONFLICT,
            ApiError::RateLimited => StatusCode::TOO_MANY_REQUESTS,
            ApiError::UpstreamError(_) => StatusCode::BAD_GATEWAY,
            ApiError::TransformError(_) => StatusCode::UNPROCESSABLE_ENTITY,
            ApiError::EncryptionError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ApiError::Database(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ApiError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    /// Get the error type string.
    pub fn error_type(&self) -> &'static str {
        match self {
            ApiError::NotFound(_) => "not_found",
            ApiError::Unauthorized => "unauthorized",
            ApiError::Forbidden(_) => "forbidden",
            ApiError::BadRequest(_) => "bad_request",
            ApiError::Conflict(_) => "conflict",
            ApiError::RateLimited => "rate_limited",
            ApiError::UpstreamError(_) => "upstream_error",
            ApiError::TransformError(_) => "transform_error",
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

/// Result type alias for API operations.
pub type ApiResult<T> = Result<T, ApiError>;

/// Convert anyhow errors to internal errors.
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
            ApiError::Unauthorized => AdiServiceError::new("unauthorized".to_string(), "Unauthorized".to_string()),
            ApiError::Forbidden(m) => AdiServiceError::new("forbidden".to_string(), m),
            _ => AdiServiceError::internal(e.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_codes() {
        assert_eq!(ApiError::NotFound("x".into()).status_code(), StatusCode::NOT_FOUND);
        assert_eq!(ApiError::Unauthorized.status_code(), StatusCode::UNAUTHORIZED);
        assert_eq!(ApiError::Forbidden("x".into()).status_code(), StatusCode::FORBIDDEN);
        assert_eq!(ApiError::BadRequest("x".into()).status_code(), StatusCode::BAD_REQUEST);
        assert_eq!(ApiError::Conflict("x".into()).status_code(), StatusCode::CONFLICT);
        assert_eq!(ApiError::RateLimited.status_code(), StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(ApiError::UpstreamError("x".into()).status_code(), StatusCode::BAD_GATEWAY);
        assert_eq!(ApiError::TransformError("x".into()).status_code(), StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(ApiError::EncryptionError("x".into()).status_code(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(ApiError::Internal("x".into()).status_code(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn test_error_types() {
        assert_eq!(ApiError::NotFound("x".into()).error_type(), "not_found");
        assert_eq!(ApiError::Unauthorized.error_type(), "unauthorized");
        assert_eq!(ApiError::Forbidden("x".into()).error_type(), "forbidden");
        assert_eq!(ApiError::BadRequest("x".into()).error_type(), "bad_request");
        assert_eq!(ApiError::Conflict("x".into()).error_type(), "conflict");
        assert_eq!(ApiError::RateLimited.error_type(), "rate_limited");
        assert_eq!(ApiError::UpstreamError("x".into()).error_type(), "upstream_error");
        assert_eq!(ApiError::TransformError("x".into()).error_type(), "transform_error");
        assert_eq!(ApiError::EncryptionError("x".into()).error_type(), "encryption_error");
        assert_eq!(ApiError::Internal("x".into()).error_type(), "internal_error");
    }

    #[test]
    fn test_into_response_json_shape() {
        let err = ApiError::NotFound("user 42".into());
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn test_api_error_to_adi_service_error() {
        let not_found: lib_adi_service::AdiServiceError = ApiError::NotFound("missing".into()).into();
        assert!(not_found.message.contains("missing"));

        let bad_req: lib_adi_service::AdiServiceError = ApiError::BadRequest("invalid".into()).into();
        assert!(bad_req.message.contains("invalid"));

        let conflict: lib_adi_service::AdiServiceError = ApiError::Conflict("dup".into()).into();
        assert!(conflict.message.contains("dup"));

        let internal: lib_adi_service::AdiServiceError = ApiError::Internal("boom".into()).into();
        assert!(internal.message.contains("boom"));

        let unauthorized: lib_adi_service::AdiServiceError = ApiError::Unauthorized.into();
        assert_eq!(unauthorized.code, "unauthorized");

        let forbidden: lib_adi_service::AdiServiceError = ApiError::Forbidden("denied".into()).into();
        assert_eq!(forbidden.code, "forbidden");
        assert!(forbidden.message.contains("denied"));

        let upstream: lib_adi_service::AdiServiceError = ApiError::UpstreamError("timeout".into()).into();
        assert!(upstream.message.contains("timeout"));
    }

    #[test]
    fn test_from_anyhow() {
        let anyhow_err = anyhow::anyhow!("something went wrong");
        let api_err: ApiError = anyhow_err.into();
        assert!(matches!(api_err, ApiError::Internal(m) if m.contains("something went wrong")));
    }
}
