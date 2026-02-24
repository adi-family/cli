use lib_console_output::{
    blocks::{Columns, KeyValue, Renderable, Section},
    out_info, out_success, theme,
};
use lib_plugin_prelude::*;
use logging_core::{LogQueryParams, LogReader};
use once_cell::sync::OnceCell;
use tokio::runtime::Runtime;

mod routes;

static RUNTIME: OnceCell<Runtime> = OnceCell::new();

fn get_runtime() -> &'static Runtime {
    RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("Failed to create tokio runtime")
    })
}

// ============================================================================
// CLI ARGS
// ============================================================================

#[derive(CliArgs)]
pub struct StartArgs {
    #[arg(long)]
    pub port: Option<String>,

    #[arg(long)]
    pub database_url: Option<String>,
}

#[derive(CliArgs)]
pub struct QueryArgs {
    #[arg(long)]
    pub service: Option<String>,

    #[arg(long)]
    pub level: Option<String>,

    #[arg(long)]
    pub search: Option<String>,

    #[arg(long)]
    pub limit: Option<String>,

    #[arg(long)]
    pub trace_id: Option<String>,
}

#[derive(CliArgs)]
pub struct TraceArgs {
    #[arg(position = 0)]
    pub trace_id: String,
}

// ============================================================================
// PLUGIN
// ============================================================================

pub struct LoggingPlugin;

impl LoggingPlugin {
    pub fn new() -> Self {
        Self
    }
}

impl Default for LoggingPlugin {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Plugin for LoggingPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata::new("adi.logging", "ADI Logging", env!("CARGO_PKG_VERSION"))
            .with_type(PluginType::Core)
            .with_author("ADI Team")
            .with_description("Centralized log ingestion and query service")
    }

    async fn init(&mut self, ctx: &PluginContext) -> Result<()> {
        PluginCtx::init(ctx);

        let _ = tracing_subscriber::fmt()
            .with_env_filter(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| "logging_plugin=info".into()),
            )
            .with_target(true)
            .try_init();

        let _ = get_runtime();
        Ok(())
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    fn provides(&self) -> Vec<&'static str> {
        vec![SERVICE_CLI_COMMANDS]
    }
}

#[async_trait]
impl CliCommands for LoggingPlugin {
    async fn list_commands(&self) -> Vec<CliCommand> {
        vec![
            Self::__sdk_cmd_meta_cmd_start(),
            Self::__sdk_cmd_meta_cmd_query(),
            Self::__sdk_cmd_meta_cmd_trace(),
            Self::__sdk_cmd_meta_cmd_stats(),
        ]
    }

    async fn run_command(&self, ctx: &CliContext) -> Result<CliResult> {
        match ctx.subcommand.as_deref() {
            Some("start") => self.__sdk_cmd_handler_cmd_start(ctx).await,
            Some("query") => self.__sdk_cmd_handler_cmd_query(ctx).await,
            Some("trace") => self.__sdk_cmd_handler_cmd_trace(ctx).await,
            Some("stats") => self.__sdk_cmd_handler_cmd_stats(ctx).await,
            Some("") | Some("help") | None => Ok(CliResult::success(self.help())),
            Some(cmd) => Ok(CliResult::error(format!(
                "Unknown command: {}. Run 'adi run adi.logging' for help.",
                cmd
            ))),
        }
    }
}

// ============================================================================
// COMMAND HANDLERS
// ============================================================================

impl LoggingPlugin {
    fn help(&self) -> String {
        format!(
            "{}\n\n\
             {}\n\
             \x20 start     Start the logging HTTP service\n\
             \x20 query     Query logs with filters\n\
             \x20 trace     Get all logs for a trace ID\n\
             \x20 stats     Show logging statistics\n\n\
             {}\n\
             \x20 adi run adi.logging start [--port 8040] [--database-url URL]\n\
             \x20 adi run adi.logging query --service my-svc --level error\n\
             \x20 adi run adi.logging trace <trace-id>\n\
             \x20 adi run adi.logging stats",
            theme::brand_bold("ADI Logging Service"),
            theme::bold("Commands:"),
            theme::bold("Usage:"),
        )
    }

    #[command(name = "start", description = "Start the logging HTTP service")]
    async fn cmd_start(&self, args: StartArgs) -> CmdResult {
        let database_url = args
            .database_url
            .or_else(|| std::env::var("DATABASE_URL").ok())
            .ok_or_else(|| {
                "DATABASE_URL not set. Pass --database-url or set DATABASE_URL env var".to_string()
            })?;

        let port: u16 = args
            .port
            .and_then(|p| p.parse().ok())
            .or_else(|| {
                std::env::var("PORT")
                    .ok()
                    .and_then(|p| p.parse().ok())
            })
            .unwrap_or(8040);

        out_info!("Starting logging service on port {}", port);

        let runtime = get_runtime();
        runtime
            .block_on(routes::run_server(&database_url, port))
            .map_err(|e| format!("Server error: {}", e))?;

        Ok(String::new())
    }

    #[command(name = "query", description = "Query logs with filters")]
    async fn cmd_query(&self, args: QueryArgs) -> CmdResult {
        let database_url = std::env::var("DATABASE_URL")
            .map_err(|_| "DATABASE_URL not set".to_string())?;

        let runtime = get_runtime();
        let pool = runtime
            .block_on(
                sqlx::postgres::PgPoolOptions::new()
                    .max_connections(5)
                    .connect(&database_url),
            )
            .map_err(|e| format!("Failed to connect to database: {}", e))?;

        let reader = LogReader::new(pool);

        let params = LogQueryParams {
            service: args.service,
            level: args.level,
            search: args.search,
            limit: args.limit.and_then(|l| l.parse().ok()),
            trace_id: args.trace_id.and_then(|t| t.parse().ok()),
            ..Default::default()
        };

        let logs = runtime
            .block_on(reader.query(&params))
            .map_err(|e| format!("Query failed: {}", e))?;

        if logs.is_empty() {
            out_info!("No logs found matching filters");
            return Ok(String::new());
        }

        let mut cols = Columns::new()
            .header(["TIME", "SERVICE", "LEVEL", "MESSAGE"])
            .indent(0)
            .gap(2);

        for log in &logs {
            let time = log.timestamp.format("%Y-%m-%d %H:%M:%S").to_string();
            let level = format_log_level(&log.level);
            let message = if log.message.len() > 80 {
                format!("{}...", &log.message[..77])
            } else {
                log.message.clone()
            };

            cols = cols.row([time, log.service.clone(), level, message]);
        }

        out_success!("Found {} log(s)", logs.len());
        Ok(cols.to_string())
    }

    #[command(name = "trace", description = "Get all logs for a trace ID")]
    async fn cmd_trace(&self, args: TraceArgs) -> CmdResult {
        let database_url = std::env::var("DATABASE_URL")
            .map_err(|_| "DATABASE_URL not set".to_string())?;

        let trace_id: uuid::Uuid = args
            .trace_id
            .parse()
            .map_err(|_| format!("Invalid trace ID: {}", args.trace_id))?;

        let runtime = get_runtime();
        let pool = runtime
            .block_on(
                sqlx::postgres::PgPoolOptions::new()
                    .max_connections(5)
                    .connect(&database_url),
            )
            .map_err(|e| format!("Failed to connect to database: {}", e))?;

        let reader = LogReader::new(pool);

        let logs = runtime
            .block_on(reader.trace_logs(trace_id))
            .map_err(|e| format!("Query failed: {}", e))?;

        if logs.is_empty() {
            out_info!("No logs found for trace {}", trace_id);
            return Ok(String::new());
        }

        let mut output = String::new();
        output.push_str(&Section::new(&format!("Trace {}", trace_id)).width(80).render());
        output.push('\n');

        for log in &logs {
            let time = log.timestamp.format("%H:%M:%S%.3f").to_string();
            let level = format_log_level(&log.level);
            output.push_str(&format!(
                "  {} {} [{}] {}\n",
                theme::muted(&time),
                theme::bold(&log.service),
                level,
                log.message
            ));
        }

        out_success!("Found {} log(s) in trace", logs.len());
        Ok(output)
    }

    #[command(name = "stats", description = "Show logging statistics")]
    async fn cmd_stats(&self) -> CmdResult {
        let database_url = std::env::var("DATABASE_URL")
            .map_err(|_| "DATABASE_URL not set".to_string())?;

        let runtime = get_runtime();
        let pool = runtime
            .block_on(
                sqlx::postgres::PgPoolOptions::new()
                    .max_connections(5)
                    .connect(&database_url),
            )
            .map_err(|e| format!("Failed to connect to database: {}", e))?;

        let reader = LogReader::new(pool);

        let stats = runtime
            .block_on(reader.stats())
            .map_err(|e| format!("Query failed: {}", e))?;

        let service_stats = runtime
            .block_on(reader.service_stats())
            .map_err(|e| format!("Query failed: {}", e))?;

        let mut output = String::new();
        output.push_str(&Section::new("Logging Statistics (24h)").width(60).render());
        output.push('\n');

        let kv = KeyValue::new()
            .indent(2)
            .entry("Total logs", stats.total_logs.to_string())
            .entry("Services", stats.services_count.to_string())
            .entry("Traces", stats.traces_count.to_string())
            .entry(
                "Errors",
                theme::error(stats.error_count.to_string()).to_string(),
            )
            .entry(
                "Warnings",
                theme::warning(stats.warn_count.to_string()).to_string(),
            );
        output.push_str(&kv.to_string());

        if !service_stats.is_empty() {
            output.push('\n');
            output.push_str(&Section::new("By Service").width(60).render());
            output.push('\n');

            let mut cols = Columns::new()
                .header(["SERVICE", "LOGS", "ERRORS"])
                .indent(2)
                .gap(2);

            for svc in &service_stats {
                let errors = if svc.error_count > 0 {
                    theme::error(svc.error_count.to_string()).to_string()
                } else {
                    theme::muted("0").to_string()
                };
                cols = cols.row([svc.service.clone(), svc.log_count.to_string(), errors]);
            }

            output.push_str(&cols.to_string());
        }

        Ok(output)
    }
}

// ============================================================================
// EXPORTS
// ============================================================================

#[no_mangle]
pub extern "C" fn plugin_abi_version() -> u32 {
    lib_plugin_abi_v3::PLUGIN_API_VERSION
}

#[no_mangle]
pub fn plugin_create() -> Box<dyn Plugin> {
    Box::new(LoggingPlugin::new())
}

#[no_mangle]
pub fn plugin_create_cli() -> Box<dyn CliCommands> {
    Box::new(LoggingPlugin::new())
}

// ============================================================================
// HELPERS
// ============================================================================

fn format_log_level(level: &str) -> String {
    match level {
        "trace" => theme::muted(level).to_string(),
        "debug" => theme::debug(level).to_string(),
        "info" => theme::success(level).to_string(),
        "notice" => theme::debug(level).to_string(),
        "warn" | "warning" => theme::warning(level).to_string(),
        "error" => theme::error(level).to_string(),
        "fatal" => theme::brand_bold(level).to_string(),
        _ => level.to_string(),
    }
}
