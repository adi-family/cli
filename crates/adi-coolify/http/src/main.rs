//! ADI Coolify HTTP Server
//!
//! REST API for Coolify deployment management.

use adi_coolify_core::{CoolifyClient, Service};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::{net::SocketAddr, sync::Arc};
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

/// Application state.
struct AppState {
    client: CoolifyClient,
    services: Vec<Service>,
}

/// Health check response.
#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    version: &'static str,
}

/// Status response for a service.
#[derive(Serialize)]
struct ServiceStatusResponse {
    id: String,
    name: String,
    uuid: String,
    status: String,
    icon: String,
}

/// Deploy request query params.
#[derive(Deserialize)]
struct DeployQuery {
    #[serde(default)]
    force: bool,
}

/// Deploy response.
#[derive(Serialize)]
struct DeployResponse {
    success: bool,
    deployment_uuid: Option<String>,
    message: String,
}

/// Deployment info response.
#[derive(Serialize)]
struct DeploymentResponse {
    uuid: String,
    status: String,
    icon: String,
    commit: Option<String>,
    created_at: Option<String>,
}

/// Default services configuration.
fn default_services() -> Vec<Service> {
    vec![
        Service {
            id: "auth".to_string(),
            name: "Auth API".to_string(),
            uuid: "ngg488ogoc80c8wogowkckow".to_string(),
            status: None,
        },
        Service {
            id: "platform".to_string(),
            name: "Platform API".to_string(),
            uuid: "cosw4cw0gscso88w8sskgk8g".to_string(),
            status: None,
        },
        Service {
            id: "signaling".to_string(),
            name: "Signaling Server".to_string(),
            uuid: "t0k0owcw00w00s4w4o0c000w".to_string(),
            status: None,
        },
        Service {
            id: "web".to_string(),
            name: "Web UI".to_string(),
            uuid: "tkg84kg0o0ok8gkcs8wcggck".to_string(),
            status: None,
        },
        Service {
            id: "analytics-ingestion".to_string(),
            name: "Analytics Ingestion".to_string(),
            uuid: "TODO_COOLIFY_UUID".to_string(),
            status: None,
        },
        Service {
            id: "analytics".to_string(),
            name: "Analytics API".to_string(),
            uuid: "TODO_COOLIFY_UUID".to_string(),
            status: None,
        },
        Service {
            id: "registry".to_string(),
            name: "Plugin Registry".to_string(),
            uuid: "TODO_COOLIFY_UUID".to_string(),
            status: None,
        },
    ]
}

/// Health check endpoint.
async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
    })
}

/// Get all services status.
async fn get_status(State(state): State<Arc<AppState>>) -> Json<Vec<ServiceStatusResponse>> {
    let mut responses = Vec::new();

    for service in &state.services {
        let (status_label, icon) = match state.client.get_application_status(&service.uuid).await {
            Ok(status) => (status.label().to_string(), status.icon().to_string()),
            Err(_) => ("error".to_string(), "?".to_string()),
        };

        responses.push(ServiceStatusResponse {
            id: service.id.clone(),
            name: service.name.clone(),
            uuid: service.uuid.clone(),
            status: status_label,
            icon,
        });
    }

    Json(responses)
}

/// List available services.
async fn list_services(State(state): State<Arc<AppState>>) -> Json<Vec<Service>> {
    Json(state.services.clone())
}

/// Deploy a service.
async fn deploy_service(
    State(state): State<Arc<AppState>>,
    Path(service_id): Path<String>,
    Query(query): Query<DeployQuery>,
) -> Result<Json<DeployResponse>, (StatusCode, Json<DeployResponse>)> {
    let service = state
        .services
        .iter()
        .find(|s| s.id == service_id)
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(DeployResponse {
                    success: false,
                    deployment_uuid: None,
                    message: format!("Service not found: {}", service_id),
                }),
            )
        })?;

    match state.client.deploy(&service.uuid, query.force).await {
        Ok(deployment) => Ok(Json(DeployResponse {
            success: true,
            deployment_uuid: Some(deployment.uuid),
            message: "Deployment started".to_string(),
        })),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(DeployResponse {
                success: false,
                deployment_uuid: None,
                message: e.to_string(),
            }),
        )),
    }
}

/// Get recent deployments for a service.
async fn get_deployments(
    State(state): State<Arc<AppState>>,
    Path(service_id): Path<String>,
) -> Result<Json<Vec<DeploymentResponse>>, (StatusCode, String)> {
    let service = state
        .services
        .iter()
        .find(|s| s.id == service_id)
        .ok_or_else(|| (StatusCode::NOT_FOUND, format!("Service not found: {}", service_id)))?;

    let deployments = state
        .client
        .get_deployments(&service.uuid, 10)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let responses: Vec<DeploymentResponse> = deployments
        .into_iter()
        .map(|d| DeploymentResponse {
            uuid: d.uuid,
            status: format!("{:?}", d.status).to_lowercase(),
            icon: d.status.icon().to_string(),
            commit: d.commit,
            created_at: d.created_at,
        })
        .collect();

    Ok(Json(responses))
}

/// Get deployment logs.
async fn get_logs(
    State(state): State<Arc<AppState>>,
    Path(deployment_uuid): Path<String>,
) -> Result<String, (StatusCode, String)> {
    state
        .client
        .get_deployment_logs(&deployment_uuid)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "adi_coolify_http=info,tower_http=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Get configuration from environment
    let coolify_url =
        std::env::var("COOLIFY_URL").unwrap_or_else(|_| "http://in.the-ihor.com".to_string());
    let api_key = std::env::var("COOLIFY_API_KEY")
        .expect("COOLIFY_API_KEY environment variable is required");
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8095);

    // Create Coolify client
    let client = CoolifyClient::new(&coolify_url, &api_key)?;

    // Create app state
    let state = Arc::new(AppState {
        client,
        services: default_services(),
    });

    // Build router
    let app = Router::new()
        .route("/health", get(health))
        .route("/api/status", get(get_status))
        .route("/api/services", get(list_services))
        .route("/api/deploy/:service_id", post(deploy_service))
        .route("/api/deployments/:service_id", get(get_deployments))
        .route("/api/logs/:deployment_uuid", get(get_logs))
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .with_state(state);

    // Start server
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!("Starting ADI Coolify HTTP server on {}", addr);
    tracing::info!("Coolify URL: {}", coolify_url);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
