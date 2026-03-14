mod generated;

use auth_core::{AuthManager, UserId};
use axum::{
    Json, Router,
    extract::State,
    http::{HeaderMap, header},
    response::{IntoResponse, Redirect},
    routing::{get, post},
};
use generated::models::*;
use generated::server::ApiError;
use lib_http_common::version_header_layer;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use uuid::Uuid;

struct AppState {
    auth: AuthManager,
}

fn auth_error(status: u16, msg: &str) -> ApiError {
    ApiError {
        status,
        code: "auth_error".to_string(),
        message: msg.to_string(),
    }
}

fn extract_bearer_token(headers: &HeaderMap) -> Option<&str> {
    if let Some(auth_header) = headers.get(header::AUTHORIZATION) {
        if let Ok(auth_str) = auth_header.to_str() {
            if let Some(token) = auth_str.strip_prefix("Bearer ") {
                return Some(token);
            }
        }
    }
    if let Some(cookie_header) = headers.get(header::COOKIE) {
        if let Ok(cookies) = cookie_header.to_str() {
            for cookie in cookies.split(';') {
                if let Some(token) = cookie.trim().strip_prefix("adi_token=") {
                    return Some(token);
                }
            }
        }
    }
    None
}

fn is_valid_email(email: &str) -> bool {
    let parts: Vec<&str> = email.split('@').collect();
    parts.len() == 2 && !parts[0].is_empty() && parts[1].contains('.')
}

fn extract_user_id(state: &AppState, headers: &HeaderMap) -> Result<UserId, ApiError> {
    let token = extract_bearer_token(headers)
        .ok_or_else(|| auth_error(401, "Missing or invalid Authorization header"))?;
    let claims = state
        .auth
        .verify_token(token)
        .map_err(|e| match e {
            auth_core::Error::TokenExpired => auth_error(401, "Token expired"),
            e => auth_error(401, &e.to_string()),
        })?;
    Uuid::parse_str(&claims.sub)
        .map(UserId)
        .map_err(|_| auth_error(500, "Invalid user ID in token"))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LoginInput {
    login: String,
    password: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AnonymousResponse {
    login: String,
    password: String,
    access_token: String,
    token_type: String,
    expires_in: i64,
}

pub fn run_server(port: u16) -> anyhow::Result<()> {
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        tracing_subscriber::fmt()
            .with_env_filter(
                tracing_subscriber::EnvFilter::from_default_env()
                    .add_directive(tracing::Level::INFO.into()),
            )
            .init();

        let auth = AuthManager::open_from_env()
            .await
            .expect("Failed to initialize auth manager. Ensure DATABASE_URL is set.");

        let state = Arc::new(AppState { auth });

        let app = Router::new()
            .route("/", get(health))
            .route("/health", get(health))
            .route("/request-code", post(request_code))
            .route("/verify", post(verify_code))
            .route("/verify-totp", post(verify_totp))
            .route("/anonymous", post(create_anonymous))
            .route("/login", post(login_with_credentials))
            .route("/me", get(get_current_user))
            .route("/logout", get(logout).post(logout))
            .route("/totp/setup", post(setup_totp))
            .route("/totp/enable", post(enable_totp))
            .route("/totp/disable", post(disable_totp))
            .route("/subtoken", post(create_subtoken))
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

        Ok(())
    })
}

async fn health() -> impl IntoResponse {
    Json(serde_json::json!({ "status": "ok", "service": "adi-auth-http" }))
}

#[derive(Deserialize)]
struct LogoutQuery {
    #[serde(default = "default_redirect")]
    next: String,
}

fn default_redirect() -> String {
    "/".to_string()
}

async fn logout(axum::extract::Query(query): axum::extract::Query<LogoutQuery>) -> impl IntoResponse {
    Redirect::temporary(&query.next)
}

async fn request_code(
    State(state): State<Arc<AppState>>,
    Json(input): Json<RequestCodeInput>,
) -> Result<Json<MessageResponse>, ApiError> {
    if !is_valid_email(&input.email) {
        return Err(auth_error(400, "Invalid email format"));
    }

    state
        .auth
        .request_code(&input.email)
        .await
        .map_err(|e| {
            tracing::error!(email = %input.email, error = %e, "request_code failed");
            auth_error(500, &e.to_string())
        })?;

    Ok(Json(MessageResponse {
        message: "Verification code sent to your email".to_string(),
    }))
}

async fn verify_code(
    State(state): State<Arc<AppState>>,
    Json(input): Json<VerifyCodeInput>,
) -> Result<Json<AuthToken>, ApiError> {
    let token = state
        .auth
        .verify_code(&input.email, &input.code)
        .await
        .map_err(|e| match e {
            auth_core::Error::InvalidCode => auth_error(401, "Invalid verification code"),
            auth_core::Error::CodeExpired => auth_error(401, "Verification code expired"),
            e => {
                tracing::error!(email = %input.email, error = %e, "verify_code failed");
                auth_error(500, &e.to_string())
            }
        })?;

    Ok(Json(AuthToken {
        access_token: token.access_token,
        token_type: token.token_type,
        expires_in: token.expires_in,
    }))
}

async fn verify_totp(
    State(state): State<Arc<AppState>>,
    Json(input): Json<VerifyTotpInput>,
) -> Result<Json<AuthToken>, ApiError> {
    let token = state
        .auth
        .verify_totp(&input.email, &input.code)
        .await
        .map_err(|e| match e {
            auth_core::Error::InvalidTotp => auth_error(401, "Invalid TOTP code"),
            auth_core::Error::TotpNotConfigured => {
                auth_error(400, "TOTP not configured for this user")
            }
            auth_core::Error::UserNotFound(_) => auth_error(404, "User not found"),
            e => {
                tracing::error!(email = %input.email, error = %e, "verify_totp failed");
                auth_error(500, &e.to_string())
            }
        })?;

    Ok(Json(AuthToken {
        access_token: token.access_token,
        token_type: token.token_type,
        expires_in: token.expires_in,
    }))
}

async fn get_current_user(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<UserInfo>, ApiError> {
    let token = extract_bearer_token(&headers)
        .ok_or_else(|| auth_error(401, "Missing or invalid Authorization header"))?;

    let claims = state.auth.verify_token(token).map_err(|e| match e {
        auth_core::Error::TokenExpired => auth_error(401, "Token expired"),
        e => auth_error(401, &e.to_string()),
    })?;

    let user_id = Uuid::parse_str(&claims.sub)
        .map(UserId)
        .map_err(|_| auth_error(500, "Invalid user ID in token"))?;

    let user = state
        .auth
        .get_user(user_id)
        .await
        .map_err(|e| auth_error(404, &e.to_string()))?;

    Ok(Json(UserInfo {
        id: user.id.0.to_string(),
        email: user.email,
        created_at: user.created_at.to_rfc3339(),
        last_login_at: user.last_login_at.map(|t| t.to_rfc3339()),
        is_admin: claims.is_admin,
    }))
}

async fn setup_totp(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<TotpSetup>, ApiError> {
    let user_id = extract_user_id(&state, &headers)?;

    let setup = state.auth.setup_totp(user_id).await.map_err(|e| match e {
        auth_core::Error::TotpAlreadyConfigured => auth_error(409, "TOTP already configured"),
        e => {
            tracing::error!(user_id = %user_id.0, error = %e, "setup_totp failed");
            auth_error(500, &e.to_string())
        }
    })?;

    Ok(Json(TotpSetup {
        secret: setup.secret,
        otpauth_url: setup.otpauth_url,
        qr_code_base_64: setup.qr_code_base64,
    }))
}

async fn enable_totp(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(input): Json<EnableTotpInput>,
) -> Result<Json<MessageResponse>, ApiError> {
    let user_id = extract_user_id(&state, &headers)?;

    state
        .auth
        .enable_totp(user_id, &input.secret, &input.code)
        .await
        .map_err(|e| match e {
            auth_core::Error::InvalidTotp => auth_error(400, "Invalid TOTP code"),
            auth_core::Error::TotpAlreadyConfigured => {
                auth_error(409, "TOTP already configured")
            }
            e => {
                tracing::error!(user_id = %user_id.0, error = %e, "enable_totp failed");
                auth_error(500, &e.to_string())
            }
        })?;

    Ok(Json(MessageResponse {
        message: "TOTP enabled successfully".to_string(),
    }))
}

async fn disable_totp(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<MessageResponse>, ApiError> {
    let user_id = extract_user_id(&state, &headers)?;

    state.auth.disable_totp(user_id).await.map_err(|e| match e {
        auth_core::Error::TotpNotConfigured => auth_error(400, "TOTP not configured"),
        e => {
            tracing::error!(user_id = %user_id.0, error = %e, "disable_totp failed");
            auth_error(500, &e.to_string())
        }
    })?;

    Ok(Json(MessageResponse {
        message: "TOTP disabled successfully".to_string(),
    }))
}

async fn create_anonymous(
    State(state): State<Arc<AppState>>,
) -> Result<Json<AnonymousResponse>, ApiError> {
    let creds = state
        .auth
        .create_anonymous()
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "create_anonymous failed");
            auth_error(500, &e.to_string())
        })?;

    Ok(Json(AnonymousResponse {
        login: creds.login,
        password: creds.password,
        access_token: creds.token.access_token,
        token_type: creds.token.token_type,
        expires_in: creds.token.expires_in,
    }))
}

const SUBTOKEN_MAX_TTL: i64 = 600; // 10 minutes

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SubtokenInput {
    #[serde(default = "default_subtoken_ttl")]
    ttl_seconds: i64,
}

fn default_subtoken_ttl() -> i64 {
    SUBTOKEN_MAX_TTL
}

async fn create_subtoken(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(input): Json<SubtokenInput>,
) -> Result<Json<AuthToken>, ApiError> {
    let parent_token = extract_bearer_token(&headers)
        .ok_or_else(|| auth_error(401, "Missing or invalid Authorization header"))?;

    let ttl = input.ttl_seconds.clamp(1, SUBTOKEN_MAX_TTL);

    let token = state
        .auth
        .generate_subtoken(parent_token, ttl)
        .map_err(|e| match e {
            auth_core::Error::TokenExpired => auth_error(401, "Token expired"),
            e => auth_error(401, &e.to_string()),
        })?;

    Ok(Json(AuthToken {
        access_token: token.access_token,
        token_type: token.token_type,
        expires_in: token.expires_in,
    }))
}

async fn login_with_credentials(
    State(state): State<Arc<AppState>>,
    Json(input): Json<LoginInput>,
) -> Result<Json<AuthToken>, ApiError> {
    let token = state
        .auth
        .login_with_credentials(&input.login, &input.password)
        .await
        .map_err(|e| match e {
            auth_core::Error::InvalidCredentials => auth_error(401, "Invalid login or password"),
            e => {
                tracing::error!(login = %input.login, error = %e, "login_with_credentials failed");
                auth_error(500, &e.to_string())
            }
        })?;

    Ok(Json(AuthToken {
        access_token: token.access_token,
        token_type: token.token_type,
        expires_in: token.expires_in,
    }))
}
