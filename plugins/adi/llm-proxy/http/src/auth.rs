use std::future::Future;
use std::pin::Pin;

use axum::extract::FromRequestParts;
use axum::http::header::AUTHORIZATION;
use axum::http::request::Parts;
use jsonwebtoken::{DecodingKey, Validation, decode};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::AppState;
use llm_proxy_core::ApiError;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: Uuid,
    pub email: String,
    pub exp: i64,
    pub iat: i64,
}

/// JWT-authenticated user for management routes.
#[derive(Debug, Clone)]
pub struct AuthUser {
    pub id: Uuid,
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
    ) -> Pin<Box<dyn Future<Output = Result<Self, Self::Rejection>> + Send + 'async_trait>>
    where
        'life0: 'async_trait,
        'life1: 'async_trait,
        Self: 'async_trait,
    {
        Box::pin(async move {
            let app_state = AppState::from_ref(state);
            let token = extract_bearer(parts).ok_or(ApiError::Unauthorized)?;

            let data = decode::<Claims>(
                &token,
                &DecodingKey::from_secret(app_state.config.jwt_secret.as_bytes()),
                &Validation::default(),
            )
            .map_err(|_| ApiError::Unauthorized)?;

            Ok(AuthUser {
                id: data.claims.sub,
                email: data.claims.email,
            })
        })
    }
}

fn extract_bearer(parts: &Parts) -> Option<String> {
    parts
        .headers
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .map(String::from)
}

/// Proxy token authentication for /v1/* routes.
#[derive(Debug, Clone)]
pub struct ProxyTokenAuth {
    pub raw_token: String,
}

impl<S> FromRequestParts<S> for ProxyTokenAuth
where
    S: Send + Sync,
{
    type Rejection = ApiError;

    fn from_request_parts<'life0, 'life1, 'async_trait>(
        parts: &'life0 mut Parts,
        _state: &'life1 S,
    ) -> Pin<Box<dyn Future<Output = Result<Self, Self::Rejection>> + Send + 'async_trait>>
    where
        'life0: 'async_trait,
        'life1: 'async_trait,
        Self: 'async_trait,
    {
        Box::pin(async move {
            let token = extract_bearer(parts).ok_or(ApiError::Unauthorized)?;
            if !token.starts_with("adi_pk_") {
                return Err(ApiError::Unauthorized);
            }
            Ok(ProxyTokenAuth { raw_token: token })
        })
    }
}

pub trait FromRef<T> {
    fn from_ref(input: &T) -> Self;
}

impl FromRef<AppState> for AppState {
    fn from_ref(input: &AppState) -> Self {
        input.clone()
    }
}
