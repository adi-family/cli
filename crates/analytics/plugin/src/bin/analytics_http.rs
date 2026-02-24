use analytics_plugin::read_server;
use axum::http::{Method, header};
use lib_http_common::version_header_layer;
use lib_logging_core::trace_layer;
use sqlx::postgres::PgPoolOptions;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use lib_env_parse::{env_vars, env_opt};

env_vars! {
    DatabaseUrl => "DATABASE_URL",
    PlatformDatabaseUrl => "PLATFORM_DATABASE_URL",
    Port => "PORT",
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load .env file
    dotenvy::dotenv().ok();

    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "analytics_http=info,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Get database URL
    let database_url = env_opt(EnvVar::DatabaseUrl.as_str())
        .or_else(|| env_opt(EnvVar::PlatformDatabaseUrl.as_str()))
        .expect("DATABASE_URL or PLATFORM_DATABASE_URL must be set");

    // Create database pool
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&database_url)
        .await?;

    tracing::info!("Connected to database");

    // Create router
    let app = read_server::create_router(pool)
        .layer(version_header_layer(
            env!("CARGO_PKG_NAME"),
            env!("CARGO_PKG_VERSION"),
        ))
        .layer(trace_layer())
        .layer(
            CorsLayer::new()
                .allow_origin(tower_http::cors::Any)
                .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
                .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION]),
        )
        .layer(TraceLayer::new_for_http());

    // Get port from environment
    let port = env_opt(EnvVar::Port.as_str())
        .and_then(|p| p.parse().ok())
        .unwrap_or(8093);

    let addr = format!("0.0.0.0:{}", port);
    tracing::info!("Analytics API listening on {}", addr);

    // Start server
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
