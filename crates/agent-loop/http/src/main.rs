mod generated;

use agent_loop_core::{AgentLoop, LoopConfig, Message, MockLlmProvider};
use async_trait::async_trait;
use axum::{routing::get, Router};
use generated::models::*;
use generated::server::{AgentLoopServiceHandler, ApiError};
use lib_http_common::version_header_layer;
use std::sync::Arc;
use tokio::sync::Mutex;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

struct AppState {
    agent: Mutex<Option<AgentLoop>>,
}

#[async_trait]
impl AgentLoopServiceHandler for AppState {
    async fn get_status(&self) -> Result<StatusResponse, ApiError> {
        let agent = self.agent.lock().await;
        Ok(StatusResponse {
            initialized: agent.is_some(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        })
    }

    async fn run(&self, body: RunRequest) -> Result<RunResponse, ApiError> {
        let config = LoopConfig {
            max_iterations: body.max_iterations.map(|v| v as usize).unwrap_or(50),
            ..Default::default()
        };

        let provider = Arc::new(MockLlmProvider::with_responses(vec![Message::assistant(
            "This is a demo response. Connect a real LLM provider for actual functionality.",
        )]));

        let mut agent = AgentLoop::new(provider).with_loop_config(config);

        if let Some(prompt) = body.system_prompt {
            agent = agent.with_system_prompt(prompt);
        }

        *self.agent.lock().await = Some(agent);

        let mut agent_guard = self.agent.lock().await;
        let agent = agent_guard.as_mut().unwrap();

        match agent.run(&body.task).await {
            Ok(response) => Ok(RunResponse {
                success: true,
                response: Some(response),
                error: None,
            }),
            Err(e) => Ok(RunResponse {
                success: false,
                response: None,
                error: Some(e.to_string()),
            }),
        }
    }
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

    let state = Arc::new(AppState {
        agent: Mutex::new(None),
    });

    let app = Router::new()
        .route("/health", get(health_check))
        .merge(generated::server::create_router::<AppState>())
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
