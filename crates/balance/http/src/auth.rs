use axum::{
    extract::FromRequestParts,
    http::{header::AUTHORIZATION, header::COOKIE, request::Parts},
};
use jsonwebtoken::{DecodingKey, Validation, decode};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use balance_core::ApiError;
use crate::{AppState, error::HttpError};

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: Uuid,
    pub email: String,
    pub exp: i64,
    pub iat: i64,
}

#[derive(Debug, Clone)]
pub struct AuthUser {
    pub id: Uuid,
    pub email: String,
}

fn extract_token(parts: &Parts) -> Option<String> {
    if let Some(auth_header) = parts
        .headers
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
    {
        if let Some(token) = auth_header.strip_prefix("Bearer ") {
            return Some(token.to_string());
        }
    }

    if let Some(cookie_header) = parts.headers.get(COOKIE).and_then(|v| v.to_str().ok()) {
        for cookie in cookie_header.split(';') {
            let cookie = cookie.trim();
            if let Some(token) = cookie.strip_prefix("adi_token=") {
                return Some(token.to_string());
            }
        }
    }

    None
}

impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
    AppState: FromRef<S>,
{
    type Rejection = HttpError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let app_state = AppState::from_ref(state);

        let token = extract_token(parts).ok_or(HttpError(ApiError::Unauthorized))?;

        let token_data = decode::<Claims>(
            &token,
            &DecodingKey::from_secret(app_state.config.jwt_secret.as_bytes()),
            &Validation::default(),
        )
        .map_err(|_| HttpError(ApiError::Unauthorized))?;

        Ok(AuthUser {
            id: token_data.claims.sub,
            email: token_data.claims.email,
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
