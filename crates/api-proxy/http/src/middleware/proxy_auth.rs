//! Proxy token authentication for proxy API.

use axum::{
    extract::{FromRef, FromRequestParts},
    http::{header::AUTHORIZATION, request::Parts},
};
use chrono::Utc;

use crate::state::AppState;
use api_proxy_core::{db, ApiError, ProxyToken};

/// Authenticated proxy token.
#[derive(Debug, Clone)]
pub struct ProxyAuth {
    /// The proxy token record
    pub token: ProxyToken,
}

impl<S> FromRequestParts<S> for ProxyAuth
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

            // Extract token from Authorization header
            let raw_token = extract_proxy_token(parts).ok_or(ApiError::Unauthorized)?;

            // Hash the token for lookup
            let token_hash = db::tokens::hash_token(&raw_token);

            // Look up the token in the database
            let token =
                db::tokens::get_proxy_token_by_hash(app_state.db.pool(), &token_hash).await?;

            // Check if token is active
            if !token.is_active {
                return Err(ApiError::Forbidden("Token is inactive".to_string()));
            }

            // Check expiration
            if let Some(expires_at) = token.expires_at {
                if expires_at < Utc::now() {
                    return Err(ApiError::Forbidden("Token has expired".to_string()));
                }
            }

            Ok(ProxyAuth { token })
        })
    }
}

impl ProxyAuth {
    /// Check if a model is allowed for this token.
    pub fn is_model_allowed(&self, model: &str) -> bool {
        // Check blocked list first
        if let Some(blocked) = &self.token.blocked_models {
            if blocked.iter().any(|m| m == model || model.starts_with(m)) {
                return false;
            }
        }

        // Check allowed list
        if let Some(allowed) = &self.token.allowed_models {
            return allowed.iter().any(|m| m == model || model.starts_with(m));
        }

        // If no allowed list, all models are allowed (except blocked ones)
        true
    }
}

/// Extract proxy token from Authorization header.
fn extract_proxy_token(parts: &Parts) -> Option<String> {
    let auth_header = parts
        .headers
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())?;

    // Accept "Bearer adi_pk_xxx" format
    if let Some(token) = auth_header.strip_prefix("Bearer ") {
        if token.starts_with("adi_pk_") {
            return Some(token.to_string());
        }
    }

    // Also accept raw token
    if auth_header.starts_with("adi_pk_") {
        return Some(auth_header.to_string());
    }

    None
}
