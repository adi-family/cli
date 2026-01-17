use adi_agent_loop_core::{AgentLoop, LoopConfig, Message, MockLlmProvider};
use axum::{
    extract::State,
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use lib_http_common::version_header_layer;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

#[derive(Clone)]
struct AppState {
    agent: Arc<Mutex<Option<AgentLoop>>>,
}

#[derive(Debug, Deserialize)]
struct RunRequest {
    task: String,
    #[serde(default)]
    max_iterations: Option<usize>,
    #[serde(default)]
    system_prompt: Option<String>,
}

#[derive(Debug, Serialize)]
struct RunResponse {
    success: bool,
    response: Option<String>,
    error: Option<String>,
}

#[derive(Debug, Serialize)]
struct StatusResponse {
    initialized: bool,
    version: String,
}

async fn run_agent(
    State(state): State<AppState>,
    Json(request): Json<RunRequest>,
) -> (StatusCode, Json<RunResponse>) {
    let config = LoopConfig {
        max_iterations: request.max_iterations.unwrap_or(50),
        ..Default::default()
    };

    let provider = Arc::new(MockLlmProvider::with_responses(vec![Message::assistant(
        "This is a demo response. Connect a real LLM provider for actual functionality.",
    )]));

    let mut agent = AgentLoop::new(provider).with_loop_config(config);

    if let Some(prompt) = request.system_prompt {
        agent = agent.with_system_prompt(prompt);
    }

    *state.agent.lock().await = Some(agent);

    let mut agent_guard = state.agent.lock().await;
    let agent = agent_guard.as_mut().unwrap();

    match agent.run(&request.task).await {
        Ok(response) => (
            StatusCode::OK,
            Json(RunResponse {
                success: true,
                response: Some(response),
                error: None,
            }),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(RunResponse {
                success: false,
                response: None,
                error: Some(e.to_string()),
            }),
        ),
    }
}

async fn get_status(State(state): State<AppState>) -> Json<StatusResponse> {
    let agent = state.agent.lock().await;
    Json(StatusResponse {
        initialized: agent.is_some(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

async fn health_check() -> &'static str {
    "OK"
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env().add_directive("info".parse()?))
        .init();

    let state = AppState {
        agent: Arc::new(Mutex::new(None)),
    };

    let app = Router::new()
        .route("/health", get(health_check))
        .route("/api/status", get(get_status))
        .route("/api/run", post(run_agent))
        .layer(version_header_layer(
            env!("CARGO_PKG_NAME"),
            env!("CARGO_PKG_VERSION"),
        ))
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .with_state(state);

    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let addr = format!("0.0.0.0:{}", port);

    tracing::info!("Starting ADI Agent Loop HTTP server on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
