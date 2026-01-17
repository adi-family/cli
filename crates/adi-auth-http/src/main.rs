use adi_auth_core::{AuthManager, TokenManager, UserId};
use axum::{
    extract::State,
    http::{header, HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use lib_http_common::version_header_layer;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use uuid::Uuid;

struct AppState {
    auth: RwLock<Option<AuthManager>>,
    /// Admin email whitelist (lowercase)
    admin_emails: HashSet<String>,
    /// Admin token manager (if ADMIN_JWT_SECRET is set)
    admin_token_manager: Option<TokenManager>,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    let port = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8090);

    let db_path = std::env::var("AUTH_DB_PATH").map(PathBuf::from).ok();

    let auth = match db_path {
        Some(path) => AuthManager::open(&path).ok(),
        None => AuthManager::open_global().ok(),
    };

    // Parse admin emails from env (comma-separated, case-insensitive)
    let admin_emails: HashSet<String> = std::env::var("ADMIN_EMAILS")
        .unwrap_or_default()
        .split(',')
        .map(|s| s.trim().to_lowercase())
        .filter(|s| !s.is_empty())
        .collect();

    if !admin_emails.is_empty() {
        tracing::info!("Admin emails configured: {:?}", admin_emails);
    }

    // Create admin token manager if ADMIN_JWT_SECRET is set
    let admin_token_manager = std::env::var("ADMIN_JWT_SECRET")
        .ok()
        .map(|secret| TokenManager::new(&secret));

    if admin_token_manager.is_some() {
        tracing::info!("Admin JWT authentication enabled");
    }

    let state = Arc::new(AppState {
        auth: RwLock::new(auth),
        admin_emails,
        admin_token_manager,
    });

    let app = Router::new()
        .route("/", get(health))
        .route("/health", get(health))
        .route("/auth/request-code", post(request_code))
        .route("/auth/verify", post(verify_code))
        .route("/auth/me", get(get_current_user))
        .with_state(state)
        .layer(version_header_layer(
            env!("CARGO_PKG_NAME"),
            env!("CARGO_PKG_VERSION"),
        ))
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive());

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port))
        .await
        .unwrap();

    tracing::info!("ADI Auth HTTP server listening on port {}", port);

    axum::serve(listener, app).await.unwrap();
}

async fn health() -> impl IntoResponse {
    Json(serde_json::json!({ "status": "ok", "service": "adi-auth-http" }))
}

#[derive(Deserialize)]
struct RequestCodeInput {
    email: String,
}

async fn request_code(
    State(state): State<Arc<AppState>>,
    Json(input): Json<RequestCodeInput>,
) -> impl IntoResponse {
    let auth = state.auth.read().await;

    let Some(auth) = auth.as_ref() else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({ "error": "Auth not initialized" })),
        );
    };

    if !is_valid_email(&input.email) {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "Invalid email format" })),
        );
    }

    match auth.request_code(&input.email) {
        Ok(()) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "message": "Verification code sent to your email"
            })),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        ),
    }
}

#[derive(Deserialize)]
struct VerifyCodeInput {
    email: String,
    code: String,
}

async fn verify_code(
    State(state): State<Arc<AppState>>,
    Json(input): Json<VerifyCodeInput>,
) -> impl IntoResponse {
    let auth = state.auth.read().await;

    let Some(auth) = auth.as_ref() else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({ "error": "Auth not initialized" })),
        );
    };

    match auth.verify_code(&input.email, &input.code) {
        Ok(token) => {
            // Check if user is an admin
            let is_admin = state.admin_emails.contains(&input.email.to_lowercase());

            // Generate admin token if user is admin and admin auth is configured
            let admin_token = if is_admin {
                if let Some(ref admin_manager) = state.admin_token_manager {
                    // Get the user to generate admin token
                    if let Ok(Some(user)) = auth.get_user_by_email(&input.email) {
                        admin_manager
                            .generate_token(&user)
                            .ok()
                            .map(|t| t.access_token)
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            };

            let mut response = serde_json::json!(token);
            if let Some(admin_token) = admin_token {
                response["admin_token"] = serde_json::json!(admin_token);
                response["is_admin"] = serde_json::json!(true);
                tracing::info!("Admin token issued for {}", input.email);
            }

            (StatusCode::OK, Json(response))
        }
        Err(adi_auth_core::Error::InvalidCode) => (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({ "error": "Invalid verification code" })),
        ),
        Err(adi_auth_core::Error::CodeExpired) => (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({ "error": "Verification code expired" })),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        ),
    }
}

async fn get_current_user(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let auth = state.auth.read().await;

    let Some(auth) = auth.as_ref() else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({ "error": "Auth not initialized" })),
        );
    };

    let token = match extract_bearer_token(&headers) {
        Some(token) => token,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({ "error": "Missing or invalid Authorization header" })),
            );
        }
    };

    let claims = match auth.verify_token(token) {
        Ok(claims) => claims,
        Err(adi_auth_core::Error::TokenExpired) => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({ "error": "Token expired" })),
            );
        }
        Err(e) => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({ "error": e.to_string() })),
            );
        }
    };

    let user_id = match Uuid::parse_str(&claims.sub) {
        Ok(id) => UserId(id),
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": "Invalid user ID in token" })),
            );
        }
    };

    match auth.get_user(user_id) {
        Ok(user) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "id": user.id.0.to_string(),
                "email": user.email,
                "created_at": user.created_at.to_rfc3339(),
                "last_login_at": user.last_login_at.map(|t| t.to_rfc3339()),
            })),
        ),
        Err(e) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": e.to_string() })),
        ),
    }
}

fn extract_bearer_token(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(header::AUTHORIZATION)?
        .to_str()
        .ok()?
        .strip_prefix("Bearer ")
}

fn is_valid_email(email: &str) -> bool {
    let parts: Vec<&str> = email.split('@').collect();
    parts.len() == 2 && !parts[0].is_empty() && parts[1].contains('.')
}
