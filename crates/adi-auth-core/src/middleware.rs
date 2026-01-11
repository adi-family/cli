use axum::{
    extract::FromRequestParts,
    http::{header, request::Parts, StatusCode},
    response::{IntoResponse, Response},
    Json,
};

use crate::{TokenClaims, TokenManager};

/// Axum extractor for authenticated requests.
///
/// Use in any service that shares the JWT_SECRET:
/// ```ignore
/// async fn protected_route(claims: AuthUser) -> impl IntoResponse {
///     format!("Hello, {}", claims.email)
/// }
/// ```
#[derive(Debug, Clone)]
pub struct AuthUser(pub TokenClaims);

impl std::ops::Deref for AuthUser {
    type Target = TokenClaims;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug)]
pub enum AuthError {
    MissingToken,
    InvalidToken(String),
    Expired,
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            AuthError::MissingToken => (StatusCode::UNAUTHORIZED, "Missing authorization token"),
            AuthError::InvalidToken(_) => (StatusCode::UNAUTHORIZED, "Invalid token"),
            AuthError::Expired => (StatusCode::UNAUTHORIZED, "Token expired"),
        };

        (status, Json(serde_json::json!({ "error": message }))).into_response()
    }
}

impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
{
    type Rejection = AuthError;

    fn from_request_parts<'life0, 'life1, 'async_trait>(
        parts: &'life0 mut Parts,
        _state: &'life1 S,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Self, Self::Rejection>> + Send + 'async_trait>,
    >
    where
        'life0: 'async_trait,
        'life1: 'async_trait,
        Self: 'async_trait,
    {
        Box::pin(async move {
            let token = parts
                .headers
                .get(header::AUTHORIZATION)
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.strip_prefix("Bearer "))
                .ok_or(AuthError::MissingToken)?;

            let manager = TokenManager::from_env();

            manager
                .verify_token(token)
                .map(AuthUser)
                .map_err(|e| match e {
                    crate::Error::TokenExpired => AuthError::Expired,
                    crate::Error::InvalidToken(msg) => AuthError::InvalidToken(msg),
                    _ => AuthError::InvalidToken(e.to_string()),
                })
        })
    }
}

/// Admin user extractor - validates against ADMIN_JWT_SECRET.
///
/// Use for admin-only routes (e.g., platform key management):
/// ```ignore
/// async fn admin_route(admin: AdminUser) -> impl IntoResponse {
///     format!("Admin access granted for {}", admin.email)
/// }
/// ```
///
/// Requires `ADMIN_JWT_SECRET` environment variable to be set.
/// Admin tokens are generated separately from regular user tokens.
#[derive(Debug, Clone)]
pub struct AdminUser(pub TokenClaims);

impl std::ops::Deref for AdminUser {
    type Target = TokenClaims;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug)]
pub enum AdminAuthError {
    MissingToken,
    InvalidToken(String),
    Expired,
    NotConfigured,
}

impl IntoResponse for AdminAuthError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            AdminAuthError::MissingToken => (StatusCode::UNAUTHORIZED, "Missing admin token"),
            AdminAuthError::InvalidToken(_) => (StatusCode::FORBIDDEN, "Invalid admin token"),
            AdminAuthError::Expired => (StatusCode::UNAUTHORIZED, "Admin token expired"),
            AdminAuthError::NotConfigured => {
                (StatusCode::SERVICE_UNAVAILABLE, "Admin auth not configured")
            }
        };

        (status, Json(serde_json::json!({ "error": message }))).into_response()
    }
}

impl<S> FromRequestParts<S> for AdminUser
where
    S: Send + Sync,
{
    type Rejection = AdminAuthError;

    fn from_request_parts<'life0, 'life1, 'async_trait>(
        parts: &'life0 mut Parts,
        _state: &'life1 S,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Self, Self::Rejection>> + Send + 'async_trait>,
    >
    where
        'life0: 'async_trait,
        'life1: 'async_trait,
        Self: 'async_trait,
    {
        Box::pin(async move {
            let token = parts
                .headers
                .get(header::AUTHORIZATION)
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.strip_prefix("Bearer "))
                .ok_or(AdminAuthError::MissingToken)?;

            let admin_secret =
                std::env::var("ADMIN_JWT_SECRET").map_err(|_| AdminAuthError::NotConfigured)?;

            let manager = crate::TokenManager::new(&admin_secret);

            manager
                .verify_token(token)
                .map(AdminUser)
                .map_err(|e| match e {
                    crate::Error::TokenExpired => AdminAuthError::Expired,
                    crate::Error::InvalidToken(msg) => AdminAuthError::InvalidToken(msg),
                    _ => AdminAuthError::InvalidToken(e.to_string()),
                })
        })
    }
}

/// Optional auth - extracts user if token present, None otherwise.
///
/// ```ignore
/// async fn maybe_auth(user: OptionalAuthUser) -> impl IntoResponse {
///     match user.0 {
///         Some(claims) => format!("Hello, {}", claims.email),
///         None => "Hello, guest".to_string(),
///     }
/// }
/// ```
#[derive(Debug, Clone)]
pub struct OptionalAuthUser(pub Option<TokenClaims>);

impl<S> FromRequestParts<S> for OptionalAuthUser
where
    S: Send + Sync,
{
    type Rejection = std::convert::Infallible;

    fn from_request_parts<'life0, 'life1, 'async_trait>(
        parts: &'life0 mut Parts,
        _state: &'life1 S,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Self, Self::Rejection>> + Send + 'async_trait>,
    >
    where
        'life0: 'async_trait,
        'life1: 'async_trait,
        Self: 'async_trait,
    {
        Box::pin(async move {
            let token = parts
                .headers
                .get(header::AUTHORIZATION)
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.strip_prefix("Bearer "));

            let claims = token.and_then(|t| TokenManager::from_env().verify_token(t).ok());

            Ok(OptionalAuthUser(claims))
        })
    }
}
