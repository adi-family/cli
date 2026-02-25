pub mod read_server;

mod ingestion_server;

use lib_plugin_abi_v3::{
    async_trait,
    cli::{CliCommand, CliCommands, CliContext, CliResult},
    Plugin, PluginContext, PluginMetadata, PluginType, Result as PluginResult,
    SERVICE_CLI_COMMANDS,
};

pub fn run_api_server(port: u16) -> anyhow::Result<()> {
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        dotenvy::dotenv().ok();

        tracing_subscriber::registry()
            .with(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| "analytics_http=info,tower_http=debug".into()),
            )
            .with(tracing_subscriber::fmt::layer())
            .init();

        let database_url = lib_env_parse::env_opt("DATABASE_URL")
            .or_else(|| lib_env_parse::env_opt("PLATFORM_DATABASE_URL"))
            .expect("DATABASE_URL or PLATFORM_DATABASE_URL must be set");

        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(10)
            .connect(&database_url)
            .await?;

        tracing::info!("Connected to database");

        let app = read_server::create_router(pool)
            .layer(lib_http_common::version_header_layer(
                env!("CARGO_PKG_NAME"),
                env!("CARGO_PKG_VERSION"),
            ))
            .layer(
                tower_http::cors::CorsLayer::new()
                    .allow_origin(tower_http::cors::Any)
                    .allow_methods([
                        axum::http::Method::GET,
                        axum::http::Method::POST,
                        axum::http::Method::OPTIONS,
                    ])
                    .allow_headers([
                        axum::http::header::CONTENT_TYPE,
                        axum::http::header::AUTHORIZATION,
                    ]),
            )
            .layer(tower_http::trace::TraceLayer::new_for_http());

        let addr = format!("0.0.0.0:{}", port);
        tracing::info!("Analytics API listening on {}", addr);

        let listener = tokio::net::TcpListener::bind(&addr).await?;
        axum::serve(listener, app).await?;

        Ok(())
    })
}

pub fn run_ingestion_server(port: u16) -> anyhow::Result<()> {
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        dotenvy::dotenv().ok();

        tracing_subscriber::registry()
            .with(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| "adi_analytics_ingestion=info,tower_http=debug".into()),
            )
            .with(tracing_subscriber::fmt::layer())
            .init();

        let database_url = lib_env_parse::env_opt("DATABASE_URL")
            .or_else(|| lib_env_parse::env_opt("PLATFORM_DATABASE_URL"))
            .expect("DATABASE_URL or PLATFORM_DATABASE_URL must be set");

        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(10)
            .connect(&database_url)
            .await?;

        tracing::info!("Connected to database");

        let app = ingestion_server::create_router(pool);

        let addr = format!("0.0.0.0:{}", port);
        tracing::info!("Analytics ingestion service listening on {}", addr);

        let listener = tokio::net::TcpListener::bind(&addr).await?;
        axum::serve(listener, app).await?;

        Ok(())
    })
}

pub struct AnalyticsPlugin;

impl AnalyticsPlugin {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Plugin for AnalyticsPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            id: "adi.analytics".to_string(),
            name: "Analytics".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            plugin_type: PluginType::Extension,
            author: Some("ADI Team".to_string()),
            description: Some("Analytics HTTP servers (read API + ingestion)".to_string()),
            category: None,
        }
    }

    async fn init(&mut self, _ctx: &PluginContext) -> PluginResult<()> {
        Ok(())
    }

    async fn shutdown(&self) -> PluginResult<()> {
        Ok(())
    }

    fn provides(&self) -> Vec<&'static str> {
        vec![SERVICE_CLI_COMMANDS]
    }
}

#[async_trait]
impl CliCommands for AnalyticsPlugin {
    async fn list_commands(&self) -> Vec<CliCommand> {
        vec![
            CliCommand {
                name: "start-api".to_string(),
                description: "Start the Analytics read API server (Ctrl+C to stop)".to_string(),
                args: vec![],
                has_subcommands: false,
            },
            CliCommand {
                name: "start-ingestion".to_string(),
                description: "Start the Analytics ingestion server (Ctrl+C to stop)".to_string(),
                args: vec![],
                has_subcommands: false,
            },
        ]
    }

    async fn run_command(&self, ctx: &CliContext) -> PluginResult<CliResult> {
        let subcommand = ctx.subcommand.as_deref().unwrap_or("");

        let parse_port = |default: u16| -> u16 {
            ctx.option::<u16>("port")
                .or_else(|| ctx.option::<String>("port").and_then(|s| s.parse().ok()))
                .or_else(|| ctx.args.first().and_then(|s| s.parse().ok()))
                .unwrap_or(default)
        };

        match subcommand {
            "start-api" => {
                let port = parse_port(8023);
                if let Err(e) = run_api_server(port) {
                    return Ok(CliResult::error(format!("Analytics API server failed: {e}")));
                }
                Ok(CliResult::success("Analytics API server stopped"))
            }
            "start-ingestion" => {
                let port = parse_port(8022);
                if let Err(e) = run_ingestion_server(port) {
                    return Ok(CliResult::error(format!("Analytics ingestion server failed: {e}")));
                }
                Ok(CliResult::success("Analytics ingestion server stopped"))
            }
            _ => Ok(CliResult::error(format!(
                "Unknown command: {subcommand}\nUsage:\n  adi run adi.analytics start-api [--port PORT]\n  adi run adi.analytics start-ingestion [--port PORT]"
            ))),
        }
    }
}

#[no_mangle]
pub fn plugin_create() -> Box<dyn Plugin> {
    Box::new(AnalyticsPlugin::new())
}

#[no_mangle]
pub fn plugin_create_cli() -> Box<dyn CliCommands> {
    Box::new(AnalyticsPlugin::new())
}

use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
