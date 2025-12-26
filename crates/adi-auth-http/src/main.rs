use adi_auth_core::{AuthManager, UserId};
use axum::{
    extract::State,
    http::{header, HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use uuid::Uuid;

struct AppState {
    auth: RwLock<Option<AuthManager>>,
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

    let db_path = std::env::var("AUTH_DB_PATH")
        .map(PathBuf::from)
        .ok();

    let auth = match db_path {
        Some(path) => AuthManager::open(&path).ok(),
        None => AuthManager::open_global().ok(),
    };

    let state = Arc::new(AppState {
        auth: RwLock::new(auth),
    });

    let app = Router::new()
        .route("/", get(health))
        .route("/health", get(health))
        .route("/auth/request-code", post(request_code))
        .route("/auth/verify", post(verify_code))
        .route("/auth/me", get(get_current_user))
        .with_state(state)
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
        Ok(token) => (StatusCode::OK, Json(serde_json::json!(token))),
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
            )
        }
    };

    let claims = match auth.verify_token(token) {
        Ok(claims) => claims,
        Err(adi_auth_core::Error::TokenExpired) => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({ "error": "Token expired" })),
            )
        }
        Err(e) => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
        }
    };

    let user_id = match Uuid::parse_str(&claims.sub) {
        Ok(id) => UserId(id),
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": "Invalid user ID in token" })),
            )
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
