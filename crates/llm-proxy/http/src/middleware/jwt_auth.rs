//! JWT authentication for management API.

use axum::{
    extract::{FromRef, FromRequestParts},
    http::{header::AUTHORIZATION, request::Parts},
};
use jsonwebtoken::{decode, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::state::AppState;
use llm_proxy_core::ApiError;

/// JWT claims structure.
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    /// User ID (subject)
    pub sub: Uuid,
    /// User email
    pub email: String,
    /// Expiration timestamp
    pub exp: i64,
    /// Issued at timestamp
    pub iat: i64,
    /// Admin flag
    #[serde(default)]
    pub is_admin: bool,
}

/// Authenticated user extracted from JWT.
#[derive(Debug, Clone)]
pub struct AuthUser {
    /// User ID
    pub id: Uuid,
    /// User email
    pub email: String,
}

impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
    AppState: FromRef<S>,
{
    type Rejection = ApiError;

    fn from_request_parts<'life0, 'life1, 'async_trait>(
        parts: &'life0 mut Parts,
        state: &'life1 S,
    ) -> ::core::pin::Pin<
        Box<
            dyn ::core::future::Future<Output = Result<Self, Self::Rejection>>
                + ::core::marker::Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        'life1: 'async_trait,
        Self: 'async_trait,
    {
        Box::pin(async move {
            let app_state = AppState::from_ref(state);

            // Extract token from Authorization header or cookie
            let token = extract_token(parts).ok_or(ApiError::Unauthorized)?;

            // Decode and validate JWT
            let token_data = decode::<Claims>(
                &token,
                &DecodingKey::from_secret(app_state.config.jwt_secret.as_bytes()),
                &Validation::default(),
            )
            .map_err(|_| ApiError::Unauthorized)?;

            Ok(AuthUser {
                id: token_data.claims.sub,
                email: token_data.claims.email,
            })
        })
    }
}

/// Extract token from Authorization header or cookie.
fn extract_token(parts: &Parts) -> Option<String> {
    // Try Authorization: Bearer <token> first
    if let Some(auth_header) = parts
        .headers
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
    {
        if let Some(token) = auth_header.strip_prefix("Bearer ") {
            return Some(token.to_string());
        }
    }

    // Fall back to adi_token cookie
    if let Some(cookie_header) = parts
        .headers
        .get(http::header::COOKIE)
        .and_then(|v| v.to_str().ok())
    {
        for cookie in cookie_header.split(';') {
            if let Some(token) = cookie.trim().strip_prefix("adi_token=") {
                return Some(token.to_string());
            }
        }
    }

    None
}

/// Admin user extractor - validates is_admin claim from JWT.
///
/// Use for admin-only routes (e.g., platform key management).
/// Admin status is determined by ADMIN_EMAILS in adi-auth service.
#[derive(Debug, Clone)]
pub struct AdminUser {
    /// User ID
    pub id: Uuid,
    /// User email
    pub email: String,
}

impl<S> FromRequestParts<S> for AdminUser
where
    S: Send + Sync,
    AppState: FromRef<S>,
{
    type Rejection = ApiError;

    fn from_request_parts<'life0, 'life1, 'async_trait>(
        parts: &'life0 mut Parts,
        state: &'life1 S,
    ) -> ::core::pin::Pin<
        Box<
            dyn ::core::future::Future<Output = Result<Self, Self::Rejection>>
                + ::core::marker::Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        'life1: 'async_trait,
        Self: 'async_trait,
    {
        Box::pin(async move {
            let app_state = AppState::from_ref(state);

            // Extract token from Authorization header or cookie
            let token = extract_token(parts).ok_or(ApiError::Unauthorized)?;

            // Decode and validate JWT
            let token_data = decode::<Claims>(
                &token,
                &DecodingKey::from_secret(app_state.config.jwt_secret.as_bytes()),
                &Validation::default(),
            )
            .map_err(|_| ApiError::Unauthorized)?;

            // Check is_admin claim
            if !token_data.claims.is_admin {
                return Err(ApiError::Forbidden("Admin access required".into()));
            }

            Ok(AdminUser {
                id: token_data.claims.sub,
                email: token_data.claims.email,
            })
        })
    }
}
