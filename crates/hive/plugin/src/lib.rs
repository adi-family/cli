use hive_core::{HiveConfigParser, ServiceInfo, ServiceManager, ServiceState};
use lib_console_output::{
    blocks::{Columns, KeyValue, Renderable, Section, Table},
    info, out_error, out_info, out_success, out_warn, spinner, theme,
};
use lib_plugin_abi_v3::{
    logs::{LogLine as AbiLogLine, LogProvider, LogStream as AbiLogStream, LogStreamContext},
    SERVICE_LOG_PROVIDER,
};
use lib_plugin_prelude::*;
use once_cell::sync::OnceCell;
use std::collections::HashMap;
use tokio::runtime::Runtime;
use tracing::{debug, trace};

mod plugin_loader;

static RUNTIME: OnceCell<Runtime> = OnceCell::new();

fn hive_daemon_config() -> hive_core::DaemonConfig {
    hive_core::DaemonConfig::new(PluginCtx::data_dir())
}

fn get_runtime() -> &'static Runtime {
    RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("Failed to create tokio runtime")
    })
}

fn init_plugin_i18n() {
    lib_plugin_prelude::init_plugin_i18n("en-US", include_str!("../locales/en-US/messages.ftl"));
}

#[derive(CliArgs)]
pub struct UpArgs {}

#[derive(CliArgs)]
pub struct DownArgs {}

#[derive(CliArgs)]
pub struct StatusArgs {
    #[arg(long)]
    pub all: bool,
}

#[derive(CliArgs)]
pub struct RestartArgs {
    #[arg(position = 0)]
    pub service: String,
}

#[derive(CliArgs)]
pub struct LogsArgs {
    #[arg(position = 0)]
    pub service: Option<String>,

    #[arg(long = "f")]
    pub follow: bool,

    #[arg(long)]
    pub tail: Option<String>,

    #[arg(long)]
    pub level: Option<String>,
}

#[derive(CliArgs)]
pub struct SourceArgs {
    #[arg(position = 0)]
    pub subcommand: Option<String>,

    #[arg(position = 1)]
    pub path_or_name: Option<String>,

    #[arg(long)]
    pub name: Option<String>,
}

#[derive(CliArgs)]
pub struct DoctorArgs {}

pub struct HivePlugin;

impl HivePlugin {
    pub fn new() -> Self {
        Self
    }
}

impl Default for HivePlugin {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Plugin for HivePlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata::new("adi.hive", t!("plugin-name"), env!("CARGO_PKG_VERSION"))
            .with_type(PluginType::Core)
            .with_author(t!("plugin-author"))
            .with_description(t!("plugin-description"))
    }

    async fn init(&mut self, ctx: &PluginContext) -> Result<()> {
        PluginCtx::init(ctx);

        let _ = tracing_subscriber::fmt()
            .with_env_filter(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| "hive_core=info,hive_plugin=info".into()),
            )
            .with_target(true)
            .try_init();

        init_plugin_i18n();

        let _ = get_runtime();
        Ok(())
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    fn provides(&self) -> Vec<&'static str> {
        vec![
            SERVICE_CLI_COMMANDS,
            SERVICE_LOG_PROVIDER,
            SERVICE_DAEMON_SERVICE,
        ]
    }
}

#[async_trait]
impl CliCommands for HivePlugin {
    async fn list_commands(&self) -> Vec<CliCommand> {
        let mut commands = vec![
            Self::__sdk_cmd_meta_up(),
            Self::__sdk_cmd_meta_down(),
            Self::__sdk_cmd_meta_status(),
            Self::__sdk_cmd_meta_restart(),
            Self::__sdk_cmd_meta_logs(),
        ];

        let mut source_cmd = Self::__sdk_cmd_meta_source();
        source_cmd.has_subcommands = true;
        commands.push(source_cmd);
        commands.push(Self::__sdk_cmd_meta_doctor());

        commands
    }

    async fn run_command(&self, ctx: &CliContext) -> Result<CliResult> {
        match ctx.subcommand.as_deref() {
            Some("up") => self.__sdk_cmd_handler_up(ctx).await,
            Some("down") => self.__sdk_cmd_handler_down(ctx).await,
            Some("status") => self.__sdk_cmd_handler_status(ctx).await,
            Some("restart") => self.__sdk_cmd_handler_restart(ctx).await,
            Some("logs") => self.__sdk_cmd_handler_logs(ctx).await,
            Some("source") => self.__sdk_cmd_handler_source(ctx).await,
            Some("doctor") => self.__sdk_cmd_handler_doctor(ctx).await,
            Some("") | Some("help") | None => Ok(CliResult::success(self.help())),
            Some(cmd) => Ok(CliResult::error(t!(
                "error-unknown-command",
                "cmd" => cmd
            ))),
        }
    }
}

struct HiveLogStream {
    handle: hive_core::daemon::LogStreamHandle,
    _runtime: tokio::runtime::Runtime,
}

#[async_trait]
impl AbiLogStream for HiveLogStream {
    async fn next(&mut self) -> Option<AbiLogLine> {
        match self._runtime.block_on(self.handle.recv()) {
            Ok(Some(line)) => Some(AbiLogLine {
                timestamp: line.timestamp,
                level: line.level,
                service: line.service_fqn,
                message: line.message,
            }),
            _ => None,
        }
    }
}

struct OneShotLogStream {
    lines: std::vec::IntoIter<hive_core::WireLogLine>,
}

#[async_trait]
impl AbiLogStream for OneShotLogStream {
    async fn next(&mut self) -> Option<AbiLogLine> {
        self.lines.next().map(|line| AbiLogLine {
            timestamp: line.timestamp,
            level: line.level,
            service: line.service_fqn,
            message: line.message,
        })
    }
}

#[async_trait]
impl LogProvider for HivePlugin {
    async fn log_stream(
        &self,
        ctx: LogStreamContext,
    ) -> lib_plugin_abi_v3::Result<Box<dyn AbiLogStream>> {
        use hive_core::DaemonClient;

        let config = hive_daemon_config();
        if hive_core::HiveDaemon::is_running(&config)
            .unwrap_or(None)
            .is_none()
        {
            return Err(PluginError::Runtime(t!("error-daemon-not-running")));
        }

        let rt = tokio::runtime::Runtime::new().map_err(|e| {
            PluginError::Runtime(t!("error-create-runtime", "error" => e.to_string()))
        })?;

        let client = DaemonClient::new(config.socket_path());

        if ctx.follow {
            let handle = rt
                .block_on(client.stream_logs(ctx.service.as_deref(), ctx.level.as_deref()))
                .map_err(|e| {
                    PluginError::Runtime(t!("error-start-log-stream", "error" => e.to_string()))
                })?;
            Ok(Box::new(HiveLogStream {
                handle,
                _runtime: rt,
            }))
        } else {
            let logs = rt
                .block_on(client.get_logs(
                    ctx.service.as_deref(),
                    ctx.tail,
                    None,
                    ctx.level.as_deref(),
                ))
                .map_err(|e| {
                    PluginError::Runtime(t!("error-get-logs", "error" => e.to_string()))
                })?;
            Ok(Box::new(OneShotLogStream {
                lines: logs.into_iter(),
            }))
        }
    }
}

#[daemon_service]
impl HivePlugin {
    async fn start(&self, ctx: DaemonContext) -> Result<()> {
        let _ = tracing_subscriber::fmt()
            .with_env_filter(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| "hive_core=info".into()),
            )
            .with_ansi(false)
            .try_init();

        let daemon_config = hive_core::DaemonConfig::from_paths(
            ctx.data_dir.clone(),
            ctx.pid_file.clone(),
            ctx.socket_path.clone(),
        );

        let dns_config = hive_core::DnsConfig {
            enabled: true,
            ..Default::default()
        };
        let daemon_config = daemon_config.with_dns(dns_config);

        let activated = lib_daemon_core::receive_activated_listeners();
        let activated_listeners: Vec<std::net::TcpListener> =
            activated.into_iter().flat_map(|g| g.listeners).collect();
        let daemon_config = daemon_config.with_activated_listeners(activated_listeners);

        // cdylib has its own Tokio copy — dedicated runtime ensures spawned tasks see a valid reactor.
        let (tx, rx) = tokio::sync::oneshot::channel::<std::result::Result<(), String>>();

        std::thread::Builder::new()
            .name("hive-daemon".into())
            .spawn(move || {
                let rt = match tokio::runtime::Builder::new_multi_thread()
                    .enable_all()
                    .build()
                {
                    Ok(rt) => rt,
                    Err(e) => {
                        let _ = tx.send(Err(
                            t!("error-build-tokio-runtime", "error" => e.to_string()),
                        ));
                        return;
                    }
                };

                let result = rt.block_on(async {
                    plugin_loader::load_hive_plugins().await;
                    let daemon = hive_core::HiveDaemon::new(daemon_config);
                    daemon.run().await
                });

                let _ = tx.send(result.map_err(|e| e.to_string()));
            })
            .map_err(|e| {
                anyhow::anyhow!(t!("error-spawn-daemon-thread", "error" => e.to_string()))
            })?;

        rx.await
            .map_err(|_| anyhow::anyhow!(t!("error-daemon-thread-terminated")))?
            .map_err(|e| anyhow::anyhow!("{}", e).into())
    }
}

#[no_mangle]
pub extern "C" fn plugin_abi_version() -> u32 {
    lib_plugin_abi_v3::PLUGIN_API_VERSION
}

#[no_mangle]
pub fn plugin_create() -> Box<dyn Plugin> {
    Box::new(HivePlugin::new())
}

#[no_mangle]
pub fn plugin_create_cli() -> Box<dyn CliCommands> {
    Box::new(HivePlugin::new())
}

#[no_mangle]
pub fn plugin_create_log_provider() -> Box<dyn LogProvider> {
    Box::new(HivePlugin::new())
}

impl HivePlugin {
    fn help(&self) -> String {
        format!(
            "{}\n\n\
             {}\n\
             \x20 up        {}\n\
             \x20 down      {}\n\
             \x20 status    {}\n\
             \x20 restart   {}\n\
             \x20 logs      {}\n\
             \x20 doctor    {}\n\n\
             {}\n\
             \x20 {}\n\
             \x20 {}\n\
             \x20 {}\n\
             \x20 {}\n\
             \x20 {}\n\n\
             {}\n\
             \x20 {}\n\
             \x20 {}\n\n\
             {}\n\
             \x20 {}\n\
             \x20 {}\n\n\
             {}\n\
             \x20 {}\n\
             \x20 {}\n\
             \x20 {}\n\n\
             {}\n\
             \x20 {}\n\
             \x20 {}\n\
             \x20 {}\n\
             \x20 {}\n\
             \x20 {}\n\
             \x20 {}\n\n\
             {}\n\
             \x20 {}",
            t!("hive-help-title"),
            t!("hive-help-service-section"),
            t!("hive-help-up"),
            t!("hive-help-down"),
            t!("hive-help-status"),
            t!("hive-help-restart"),
            t!("hive-help-logs"),
            t!("hive-help-doctor"),
            t!("hive-help-usage-section"),
            t!("hive-help-up-usage"),
            t!("hive-help-down-usage"),
            t!("hive-help-status-usage"),
            t!("hive-help-restart-usage"),
            t!("hive-help-logs-usage"),
            t!("hive-help-source-section"),
            t!("hive-help-source-name"),
            t!("hive-help-source-omit"),
            t!("hive-help-startup-section"),
            t!("hive-help-startup-detached"),
            t!("hive-help-startup-default"),
            t!("hive-help-logs-section"),
            t!("hive-help-logs-follow"),
            t!("hive-help-logs-tail"),
            t!("hive-help-logs-level"),
            t!("hive-help-source-mgmt-section"),
            t!("hive-help-source-list"),
            t!("hive-help-source-add"),
            t!("hive-help-source-remove"),
            t!("hive-help-source-reload"),
            t!("hive-help-source-enable"),
            t!("hive-help-source-disable"),
            t!("hive-help-orchestrator-note"),
            t!("hive-help-orchestrator-plugin"),
        )
    }

    #[command(name = "up", description = "cmd-up-help")]
    async fn up(&self, _args: UpArgs) -> CmdResult {
        use hive_core::DaemonClient;

        trace!("cmd_up started");

        let runtime = get_runtime();
        let project_root = resolve_hive_root()?;
        trace!(project_root = %project_root.display(), "Resolved hive root");

        apply_project_registry_url(&project_root);

        let daemon_config = ensure_daemon_running()?;
        let client = DaemonClient::new(daemon_config.socket_path());

        let source_name = runtime.block_on(async {
            client
                .add_source(&project_root.to_string_lossy(), None)
                .await
                .map_err(|e| t!("error-register-source", "error" => e.to_string()))
        })?;

        let source_display_name = project_root
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("default");

        runtime.block_on(async {
            client
                .send_fire_and_forget(hive_core::DaemonRequest::StartSource {
                    name: source_name.clone(),
                })
                .await
                .map_err(|e| t!("error-start-source", "error" => e.to_string()))
        })?;

        out_success!(
            "{}",
            t!("hive-up-starting", "source" => source_display_name)
        );

        Ok(String::new())
    }

    #[command(name = "down", description = "cmd-down-help")]
    async fn down(&self, _args: DownArgs) -> CmdResult {
        use hive_core::topological_sort_levels;

        trace!("cmd_down started");
        let runtime = get_runtime();
        let project_root = resolve_hive_root()?;
        trace!(project_root = %project_root.display(), "Resolved hive root");

        let parser = HiveConfigParser::new(&project_root);

        if !parser.config_exists() {
            return Err(format!(
                "{}\n{}",
                t!("hive-config-not-found", "path" => project_root.display().to_string()),
                t!("hive-config-not-found-source-hint")
            ));
        }

        let config = parser
            .parse()
            .map_err(|e| t!("hive-config-parse-error", "error" => e.to_string()))?;

        let daemon_config = ensure_daemon_running()?;
        let client = std::sync::Arc::new(hive_core::DaemonClient::new(daemon_config.socket_path()));

        let mut levels: Vec<Vec<String>> = topological_sort_levels(&config)
            .map_err(|e| t!("error-sort-services", "error" => e.to_string()))?;
        levels.reverse();

        let total: usize = levels.iter().map(|l| l.len()).sum();

        let source_name = project_root
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("default");

        out_info!(
            "{}",
            t!("hive-down-stopping", "count" => total.to_string(), "source" => source_name)
        );
        out_info!("");

        let mut stopped = 0;
        let mut failed = 0;

        for level in &levels {
            if level.len() == 1 {
                let service = &level[0];
                let sp = spinner(&t!(
                    "hive-down-stopping-service",
                    "service" => service.as_str()
                ));

                let fqn = format!("{}:{}", source_name, service);
                let result = runtime
                    .block_on(client.stop_service(&fqn))
                    .map_err(|e| anyhow::anyhow!("{}", e));


                match result {
                    Ok(_) => {
                        sp.success(Some(
                            &t!("hive-down-stopped", "service" => service.as_str()),
                        ));
                        stopped += 1;
                    }
                    Err(e) => {
                        sp.fail(
                            Some(&t!("hive-down-failed", "service" => service.as_str())),
                            Some(&e.to_string()),
                        );
                        failed += 1;
                    }
                }
            } else {
                let names = level
                    .iter()
                    .map(|s| s.as_str())
                    .collect::<Vec<_>>()
                    .join(", ");
                let sp = spinner(&t!("hive-down-stopping-parallel", "names" => names.as_str()));

                let results: Vec<anyhow::Result<()>> = runtime.block_on(async {
                    let handles: Vec<_> = level
                        .iter()
                        .map(|name| {
                            let client = client.clone();
                            let fqn = format!("{}:{}", source_name, name);
                            async move {
                                client
                                    .stop_service(&fqn)
                                    .await
                                    .map_err(|e| anyhow::anyhow!("{}", e))
                            }
                        })
                        .collect();
                    futures::future::join_all(handles).await
                });

                let level_failed = results.iter().any(|r| r.is_err());
                if level_failed {
                    sp.fail(
                        Some(&t!("hive-down-stopped-group", "names" => names.as_str())),
                        None,
                    );
                } else {
                    sp.success(Some(&t!(
                        "hive-down-stopped-group",
                        "names" => names.as_str()
                    )));
                }

                for (name, result) in level.iter().zip(results) {
                    match result {
                        Ok(_) => {
                            stopped += 1;
                        }
                        Err(e) => {
                            out_error!("{}: {}", name, e);
                            failed += 1;
                        }
                    }
                }
            }
        }

        out_info!("");
        if failed == 0 {
            out_success!(
                "{}",
                t!("hive-down-success", "count" => stopped.to_string())
            );
        } else {
            out_warn!(
                "{}",
                t!("hive-down-partial",
                    "stopped" => stopped.to_string(),
                    "failed" => failed.to_string())
            );
        }

        Ok(String::new())
    }

    #[command(name = "status", description = "cmd-status-help")]
    async fn status(&self, _args: StatusArgs) -> CmdResult {
        use hive_core::DaemonClient;

        trace!("cmd_status started");
        let runtime = get_runtime();
        let project_root = resolve_hive_root()?;
        trace!(project_root = %project_root.display(), "Resolved hive root");

        let parser = HiveConfigParser::new(&project_root);

        if !parser.config_exists() {
            return Err(format!(
                "{}\n{}",
                t!("hive-config-not-found", "path" => project_root.display().to_string()),
                t!("hive-config-not-found-source-hint")
            ));
        }

        let config = parser
            .parse()
            .map_err(|e| t!("hive-config-parse-error", "error" => e.to_string()))?;

        let daemon_config = hive_daemon_config();
        let daemon_info = hive_core::HiveDaemon::is_running(&daemon_config)
            .ok()
            .flatten()
            .and_then(|_| {
                let client = DaemonClient::new(daemon_config.socket_path());
                runtime.block_on(client.status()).ok()
            });

        let source_name = project_root
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("default");

        // When daemon is running, query it for live service state (handles docker runner etc.).
        // Fall back to local detection when daemon is unavailable.
        let svc_status: HashMap<String, ServiceInfo> = if daemon_info.is_some() {
            let client = DaemonClient::new(daemon_config.socket_path());
            if let Ok(services) = runtime.block_on(client.list_services(Some(source_name))) {
                services
                    .into_iter()
                    .map(|s| {
                        let info = ServiceInfo {
                            name: s.name.clone(),
                            state: parse_service_state(&s.state),
                            pid: s.pid,
                            container_id: s.container_id,
                            ports: s.ports,
                            healthy: s.healthy,
                            last_error: None,
                            restart_count: s.restart_count,
                        };
                        (s.name, info)
                    })
                    .collect()
            } else {
                HashMap::new()
            }
        } else {
            let manager = ServiceManager::new(parser.project_root(), config.clone())
                .map_err(|e| t!("error-init-service-manager", "error" => e.to_string()))?;
            runtime.block_on(async { manager.detect_running_services().await })
        };

        let mut output = String::new();

        output.push_str(&build_daemon_section(daemon_info.as_ref()));
        let (services_output, counts) =
            build_services_section(&config, &svc_status, &parser.config_path());
        output.push_str(&services_output);

        if daemon_info.is_some() {
            let client = DaemonClient::new(daemon_config.socket_path());
            output.push_str(&build_logs_section(&client, runtime, &counts));
        }

        output.push_str(&build_status_summary(&counts));

        Ok(output)
    }

    #[command(name = "restart", description = "cmd-restart-help")]
    async fn restart(&self, args: RestartArgs) -> CmdResult {
        let runtime = get_runtime();
        let service_name = &args.service;
        debug!(service = %service_name, "cmd_restart started");
        let project_root = resolve_hive_root()?;

        let parser = HiveConfigParser::new(&project_root);

        if !parser.config_exists() {
            return Err(format!(
                "{}\n{}",
                t!("hive-config-not-found", "path" => project_root.display().to_string()),
                t!("hive-config-not-found-source-hint")
            ));
        }

        let config = parser
            .parse()
            .map_err(|e| t!("hive-config-parse-error", "error" => e.to_string()))?;

        if !config.services.contains_key(service_name.as_str()) {
            return Err(t!(
                "hive-restart-unknown-service",
                "service" => service_name.as_str()
            ));
        }

        let daemon_config = ensure_daemon_running()?;
        let client = hive_core::DaemonClient::new(daemon_config.socket_path());

        let source_name = project_root
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("default");
        let fqn = format!("{}:{}", source_name, service_name);

        let sp = spinner(&t!(
            "hive-restart-restarting",
            "service" => service_name.as_str()
        ));

        let result: anyhow::Result<()> = runtime
            .block_on(client.restart_service(&fqn))
            .map_err(|e| anyhow::anyhow!("{}", e));

        match result {
            Ok(_) => {
                sp.success(Some(&t!(
                    "hive-restart-success",
                    "service" => service_name.as_str()
                )));
                Ok(String::new())
            }
            Err(e) => {
                sp.fail(
                    Some(&t!(
                        "hive-restart-failed",
                        "service" => service_name.as_str()
                    )),
                    Some(&e.to_string()),
                );
                Err(t!("error-restart-service", "error" => e.to_string()))
            }
        }
    }

    #[command(name = "logs", description = "cmd-logs-help")]
    async fn logs(&self, args: LogsArgs) -> CmdResult {
        trace!("cmd_logs started");

        let follow = args.follow;
        let tail: Option<u32> = args.tail.as_deref().and_then(|s| s.parse().ok());
        let level = args.level.as_deref();

        let (client, runtime) = require_daemon_client()?;
        let service_fqn = args.service.as_deref();

        if follow {
            let service_suffix = service_fqn
                .map(|s| t!("hive-logs-service-suffix", "service" => s))
                .unwrap_or_default();
            info(&t!(
                "hive-logs-streaming",
                "service_suffix" => service_suffix.as_str()
            ));
            info(&t!("hive-logs-press-ctrlc"));

            runtime.block_on(async {
                let mut handle = client
                    .stream_logs(service_fqn, level)
                    .await
                    .map_err(|e| t!("error-start-log-stream", "error" => e.to_string()))?;

                loop {
                    match handle.recv().await {
                        Ok(Some(line)) => {
                            let level_colored = format_log_level(&line.level);
                            let timestamp = line.timestamp.format("%H:%M:%S%.3f");
                            out_info!(
                                "{} {} [{}] {}",
                                timestamp,
                                line.service_fqn,
                                level_colored,
                                line.message
                            );
                        }
                        Ok(None) => break,
                        Err(e) => return Err(t!("error-stream", "error" => e.to_string())),
                    }
                }
                Ok::<(), String>(())
            })?;

            return Ok(t!("hive-logs-stream-ended"));
        }

        let logs = runtime
            .block_on(client.get_logs(service_fqn, tail, None, level))
            .map_err(|e| t!("error-get-logs", "error" => e.to_string()))?;

        if logs.is_empty() {
            return Ok(t!("hive-logs-empty"));
        }

        let mut output = String::new();
        for line in logs {
            let level_colored = format_log_level(&line.level);
            let timestamp = line.timestamp.format("%H:%M:%S%.3f");
            output.push_str(&format!(
                "{} {} [{}] {}\n",
                timestamp, line.service_fqn, level_colored, line.message
            ));
        }
        Ok(output)
    }

    #[command(name = "source", description = "cmd-source-help")]
    async fn source(&self, args: SourceArgs) -> CmdResult {
        let subcommand = args.subcommand.as_deref().unwrap_or("list");
        debug!(subcommand, "cmd_source");

        match subcommand {
            "list" => cmd_source_list(),
            "add" => cmd_source_add(args.path_or_name.as_deref(), args.name.as_deref()),
            "remove" => cmd_source_remove(args.path_or_name.as_deref()),
            "reload" => cmd_source_reload(args.path_or_name.as_deref()),
            "enable" => cmd_source_enable(args.path_or_name.as_deref()),
            "disable" => cmd_source_disable(args.path_or_name.as_deref()),
            "help" | "" => Ok(get_source_help()),
            _ => Err(t!(
                "error-unknown-source-command",
                "cmd" => subcommand
            )),
        }
    }

    #[command(name = "doctor", description = "cmd-doctor-help")]
    async fn doctor(&self, _args: DoctorArgs) -> CmdResult {
        cmd_doctor()
    }
}

fn ensure_daemon_running() -> std::result::Result<hive_core::DaemonConfig, String> {
    let client = lib_daemon_client::DaemonClient::new();
    let runtime = get_runtime();

    runtime.block_on(async {
        client
            .ensure_running()
            .await
            .map_err(|e| t!("error-start-daemon", "error" => e.to_string()))?;

        let services = client
            .list_services()
            .await
            .map_err(|e| t!("error-list-services", "error" => e.to_string()))?;

        let hive_running = services
            .iter()
            .any(|s| s.name == "adi.hive" && s.state.is_running());

        if !hive_running {
            out_info!("{}", t!("hive-daemon-starting"));
            client
                .start_service("adi.hive", None)
                .await
                .map_err(|e| t!("error-start-hive-service", "error" => e.to_string()))?;

            let mut attempts = 0;
            while attempts < 20 {
                tokio::time::sleep(std::time::Duration::from_millis(250)).await;
                if let Ok(services) = client.list_services().await {
                    if services
                        .iter()
                        .any(|s| s.name == "adi.hive" && s.state.is_running())
                    {
                        break;
                    }
                }
                attempts += 1;
            }

            if attempts >= 20 {
                return Err(t!("hive-daemon-start-timeout"));
            }

            out_success!("{}", t!("hive-daemon-started"));
        }

        // The daemon binds its socket after source_manager.init() and proxy startup,
        // so being alive (PID check) doesn't mean the socket is ready yet.
        let daemon_config = hive_daemon_config();
        let socket_client = hive_core::DaemonClient::new(daemon_config.socket_path());
        let mut socket_attempts = 0;
        loop {
            if socket_client.ping().await.unwrap_or(false) {
                break;
            }
            socket_attempts += 1;
            if socket_attempts >= 20 {
                return Err(t!("hive-daemon-start-timeout"));
            }
            tokio::time::sleep(std::time::Duration::from_millis(250)).await;
        }

        Ok(daemon_config)
    })
}

#[allow(dead_code)]
fn collect_services_to_start(
    args: &[&str],
    config: &hive_core::HiveConfig,
    all_services: Vec<String>,
) -> Vec<String> {
    if args.is_empty() {
        all_services
    } else {
        let requested: std::collections::HashSet<String> =
            args.iter().map(|s| s.to_string()).collect();
        let mut needed: std::collections::HashSet<String> = requested.clone();
        let mut to_process: Vec<String> = requested.into_iter().collect();

        while let Some(name) = to_process.pop() {
            if let Some(service_config) = config.services.get(&name) {
                for dep in &service_config.depends_on {
                    if needed.insert(dep.clone()) {
                        to_process.push(dep.clone());
                    }
                }
            }
        }

        all_services
            .into_iter()
            .filter(|s| needed.contains(s))
            .collect()
    }
}

/// Apply the project's `registry_url` from hive.yaml as the process-level
/// `ADI_REGISTRY_URL` env var, but only when the env var is not already set.
/// This lets each project declare its own plugin registry without requiring
/// every developer to export a global env var.
fn apply_project_registry_url(project_root: &std::path::Path) {
    if std::env::var("ADI_REGISTRY_URL").is_ok() {
        return; // explicit env var always wins
    }
    let parser = HiveConfigParser::new(project_root);
    if let Ok(config) = parser.parse() {
        if let Some(url) = config.registry_url {
            std::env::set_var("ADI_REGISTRY_URL", &url);
            trace!(url = %url, "Applied registry URL from hive.yaml");
        }
    }
}

fn resolve_hive_root() -> std::result::Result<std::path::PathBuf, String> {
    trace!("Resolving hive root directory");

    let current_dir = std::env::current_dir()
        .map_err(|e| t!("error-get-current-dir", "error" => e.to_string()))?;

    if let Some(root) = hive_core::find_project_root(&current_dir) {
        return Ok(root);
    }

    Ok(current_dir)
}

fn parse_service_state(s: &str) -> ServiceState {
    match s {
        "running" => ServiceState::Running,
        "starting" => ServiceState::Starting,
        "stopping" => ServiceState::Stopping,
        "crashed" => ServiceState::Crashed,
        "exited" => ServiceState::Exited,
        "unhealthy" => ServiceState::Unhealthy,
        "port conflict" => ServiceState::PortConflict,
        _ => ServiceState::Stopped,
    }
}

fn format_uptime(secs: u64) -> String {
    let days = secs / 86400;
    let hours = (secs % 86400) / 3600;
    let mins = (secs % 3600) / 60;
    let s = secs % 60;
    if days > 0 {
        t!("uptime-days", "days" => days.to_string(), "hours" => hours.to_string(), "mins" => mins.to_string())
    } else if hours > 0 {
        t!("uptime-hours", "hours" => hours.to_string(), "mins" => mins.to_string(), "secs" => s.to_string())
    } else if mins > 0 {
        t!("uptime-minutes", "mins" => mins.to_string(), "secs" => s.to_string())
    } else {
        t!("uptime-seconds", "secs" => s.to_string())
    }
}

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

struct ServiceCounts {
    running: usize,
    stopped: usize,
    problem: usize,
    problem_services: Vec<String>,
}

fn build_daemon_section(daemon_info: Option<&hive_core::DaemonStatus>) -> String {
    let mut output = String::new();
    output.push_str(&Section::new(&t!("hive-daemon-section")).width(60).render());
    output.push('\n');

    if let Some(ds) = daemon_info {
        let uptime = format_uptime(ds.uptime_secs);

        let kv = KeyValue::new()
            .indent(2)
            .entry(
                &t!("label-status"),
                theme::success(&t!("hive-daemon-running")).to_string(),
            )
            .entry(
                &t!("label-pid"),
                ds.pid.map_or_else(|| "-".to_string(), |p| p.to_string()),
            )
            .entry(&t!("label-version"), &ds.version)
            .entry(&t!("label-uptime"), uptime)
            .entry(&t!("label-sources"), ds.source_count.to_string())
            .entry(
                &t!("label-services"),
                format!(
                    "{}/{}",
                    theme::success(ds.running_services),
                    ds.total_services
                ),
            );
        output.push_str(&kv.to_string());
    } else {
        let kv = KeyValue::new()
            .indent(2)
            .entry(
                &t!("label-status"),
                theme::error(&t!("hive-daemon-not-running")).to_string(),
            )
            .entry(
                &t!("label-hint"),
                theme::muted(&t!("hive-daemon-hint")).to_string(),
            );
        output.push_str(&kv.to_string());
    }

    output.push('\n');
    output
}

fn build_service_url(svc_cfg: Option<&hive_core::ServiceConfig>) -> String {
    svc_cfg
        .and_then(|s| s.proxy.as_ref())
        .map(|proxy| {
            proxy
                .endpoints()
                .iter()
                .map(|ep| {
                    let host = ep.host.as_deref().unwrap_or("localhost");
                    format!("http://{}{}", host, ep.path)
                })
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_else(|| theme::muted("-").to_string())
}

fn build_services_section(
    config: &hive_core::HiveConfig,
    svc_status: &HashMap<String, hive_core::ServiceInfo>,
    config_path: &std::path::Path,
) -> (String, ServiceCounts) {
    let mut output = String::new();
    output.push_str(&Section::new(&t!("section-services")).width(60).render());
    output.push('\n');
    output.push_str(&format!("  {}\n\n", theme::muted(config_path.display())));

    let mut service_names: Vec<_> = config.services.keys().collect();
    service_names.sort();

    let mut table = Table::new().header([
        "",
        &t!("header-service"),
        &t!("header-state"),
        &t!("header-health"),
        &t!("header-pid"),
        &t!("header-ports"),
        &t!("header-url"),
    ]);
    let mut counts = ServiceCounts {
        running: 0,
        stopped: 0,
        problem: 0,
        problem_services: Vec::new(),
    };

    for name in &service_names {
        let svc_cfg = config.services.get(*name);
        let url = build_service_url(svc_cfg);

        if let Some(info) = svc_status.get(*name) {
            let (icon, state_str) = match info.state {
                ServiceState::Running => {
                    counts.running += 1;
                    (
                        theme::success(theme::icons::SUCCESS).to_string(),
                        theme::success(&t!("state-running")).to_string(),
                    )
                }
                ServiceState::Crashed | ServiceState::Exited | ServiceState::Unhealthy => {
                    counts.problem += 1;
                    counts.problem_services.push((*name).clone());
                    (
                        theme::error(theme::icons::ERROR).to_string(),
                        theme::error(info.state.to_string()).to_string(),
                    )
                }
                ServiceState::Starting | ServiceState::Stopping => (
                    theme::warning(theme::icons::PENDING).to_string(),
                    theme::warning(info.state.to_string()).to_string(),
                ),
                ServiceState::PortConflict => {
                    counts.problem += 1;
                    counts.problem_services.push((*name).clone());
                    (
                        theme::warning(theme::icons::WARNING).to_string(),
                        theme::warning(&t!("state-port-conflict")).to_string(),
                    )
                }
                ServiceState::Stopped => {
                    counts.stopped += 1;
                    (
                        theme::muted(theme::icons::PENDING).to_string(),
                        theme::muted(&t!("state-stopped")).to_string(),
                    )
                }
            };

            let health = match info.healthy {
                Some(true) => theme::success(&t!("state-healthy")).to_string(),
                Some(false) => {
                    if !counts.problem_services.contains(&(*name).to_string()) {
                        counts.problem += 1;
                        counts.problem_services.push((*name).clone());
                    }
                    theme::error(&t!("state-unhealthy")).to_string()
                }
                None => theme::muted("-").to_string(),
            };

            let pid = info
                .pid
                .map_or_else(|| theme::muted("-").to_string(), |p| p.to_string());

            let ports_str = if info.ports.is_empty() {
                theme::muted("-").to_string()
            } else {
                let mut port_parts: Vec<String> = info
                    .ports
                    .iter()
                    .map(|(k, v)| format!("{}:{}", k, v))
                    .collect();
                port_parts.sort();
                port_parts.join(", ")
            };

            table = table.row([
                icon,
                theme::bold(*name).to_string(),
                state_str,
                health,
                pid,
                ports_str,
                url,
            ]);
        } else {
            counts.stopped += 1;
            table = table.row([
                theme::muted(theme::icons::PENDING).to_string(),
                theme::bold(*name).to_string(),
                theme::muted(&t!("state-stopped")).to_string(),
                theme::muted("-").to_string(),
                theme::muted("-").to_string(),
                theme::muted("-").to_string(),
                url,
            ]);
        }
    }

    output.push_str(&table.to_string());
    output.push('\n');
    (output, counts)
}

fn build_logs_section(
    client: &hive_core::DaemonClient,
    runtime: &Runtime,
    counts: &ServiceCounts,
) -> String {
    let mut output = String::new();

    if !counts.problem_services.is_empty() {
        output.push_str(&Section::new(&t!("section-recent-logs")).width(60).render());
        output.push('\n');

        for svc in &counts.problem_services {
            if let Ok(logs) = runtime.block_on(client.get_logs(Some(svc), Some(5), None, None)) {
                if !logs.is_empty() {
                    output.push_str(&format!("  {}:\n", theme::error(svc)));
                    for line in &logs {
                        let lvl = format_log_level(&line.level);
                        let ts = line.timestamp.format("%H:%M:%S");
                        output.push_str(&format!(
                            "    {} [{}] {}\n",
                            theme::muted(ts),
                            lvl,
                            theme::muted(&line.message)
                        ));
                    }
                    output.push('\n');
                }
            }
        }
    } else if counts.running > 0 {
        if let Ok(logs) = runtime.block_on(client.get_logs(None, Some(5), None, None)) {
            if !logs.is_empty() {
                output.push_str(
                    &Section::new(&t!("section-recent-activity"))
                        .width(60)
                        .render(),
                );
                output.push('\n');

                for line in &logs {
                    let lvl = format_log_level(&line.level);
                    let ts = line.timestamp.format("%H:%M:%S");
                    output.push_str(&format!(
                        "  {} {} [{}] {}\n",
                        theme::muted(ts),
                        theme::muted(&line.service_fqn),
                        lvl,
                        &line.message
                    ));
                }
                output.push('\n');
            }
        }
    }

    output
}

fn build_status_summary(counts: &ServiceCounts) -> String {
    let mut parts: Vec<String> = Vec::new();
    if counts.running > 0 {
        parts.push(format!(
            "{} {}",
            theme::success(theme::icons::SUCCESS),
            theme::success(&t!("summary-running", "count" => counts.running.to_string()))
        ));
    }
    if counts.problem > 0 {
        parts.push(format!(
            "{} {}",
            theme::error(theme::icons::ERROR),
            theme::error(&t!("summary-unhealthy", "count" => counts.problem.to_string()))
        ));
    }
    if counts.stopped > 0 {
        parts.push(format!(
            "{} {}",
            theme::muted(theme::icons::PENDING),
            theme::muted(&t!("summary-stopped", "count" => counts.stopped.to_string()))
        ));
    }
    if parts.is_empty() {
        String::new()
    } else {
        format!("  {}\n", parts.join("  "))
    }
}

fn require_daemon_client(
) -> std::result::Result<(hive_core::DaemonClient, &'static Runtime), String> {
    use hive_core::DaemonClient;

    let config = hive_daemon_config();

    if hive_core::HiveDaemon::is_running(&config)
        .map_err(|e| e.to_string())?
        .is_none()
    {
        return Err(t!("error-daemon-not-running"));
    }

    let runtime = get_runtime();
    let client = DaemonClient::new(config.socket_path());
    Ok((client, runtime))
}

fn get_source_help() -> String {
    format!(
        "{}\n\n\
         {}\n\
         \x20 {}\n\
         \x20 {}\n\
         \x20 {}\n\
         \x20 {}\n\
         \x20 {}\n\
         \x20 {}\n\n\
         {}\n\
         \x20 {}\n\
         \x20 {}\n\
         \x20 {}\n\
         \x20 {}\n\
         \x20 {}\n\n\
         {}\n\
         \x20 - {}\n\
         \x20 - {}\n\n\
         {}",
        t!("hive-source-help-title"),
        t!("hive-source-help-commands"),
        t!("hive-source-help-cmd-list"),
        t!("hive-source-help-cmd-add"),
        t!("hive-source-help-cmd-remove"),
        t!("hive-source-help-cmd-reload"),
        t!("hive-source-help-cmd-enable"),
        t!("hive-source-help-cmd-disable"),
        t!("hive-source-help-usage"),
        t!("hive-source-help-usage-list"),
        t!("hive-source-help-usage-add"),
        t!("hive-source-help-usage-add-name"),
        t!("hive-source-help-usage-remove"),
        t!("hive-source-help-usage-reload"),
        t!("hive-source-help-sources-desc"),
        t!("hive-source-help-yaml-desc"),
        t!("hive-source-help-sqlite-desc"),
        t!("hive-source-help-default"),
    )
}

fn cmd_source_list() -> CmdResult {
    let (client, runtime) = require_daemon_client()?;

    let sources = runtime
        .block_on(client.list_sources())
        .map_err(|e| t!("error-list-sources", "error" => e.to_string()))?;

    if sources.is_empty() {
        return Ok(format!(
            "{}\n\n{}",
            t!("hive-source-no-sources"),
            t!("hive-source-no-sources-hint")
        ));
    }

    let mut cols = Columns::new()
        .header([
            &*t!("header-name"),
            &*t!("header-type"),
            &*t!("header-path"),
            &*t!("header-services"),
            &*t!("header-status"),
        ])
        .indent(0)
        .gap(2);

    for source in sources {
        let type_str = match source.source_type {
            hive_core::WireSourceType::Yaml => "yaml",
            hive_core::WireSourceType::Sqlite => "sqlite",
        };

        let status_str = match &source.status {
            hive_core::WireSourceStatus::Loaded => theme::warning(&t!("state-loaded")).to_string(),
            hive_core::WireSourceStatus::Running => {
                theme::success(&t!("state-running")).to_string()
            }
            hive_core::WireSourceStatus::Stopped => theme::muted(&t!("state-stopped")).to_string(),
            hive_core::WireSourceStatus::Error(e) => {
                theme::error(&t!("state-error", "error" => e.as_str())).to_string()
            }
        };

        let path_str = source.path.display().to_string();
        let path_truncated = if path_str.len() > 38 {
            format!("...{}", &path_str[path_str.len() - 35..])
        } else {
            path_str
        };

        cols = cols.row([
            source.name.clone(),
            type_str.to_string(),
            path_truncated,
            source.service_count.to_string(),
            status_str,
        ]);
    }

    Ok(cols.to_string())
}

fn cmd_source_add(path: Option<&str>, name: Option<&str>) -> CmdResult {
    let path = path.ok_or_else(|| t!("hive-source-missing-path"))?;
    let (client, runtime) = require_daemon_client()?;

    let result = runtime
        .block_on(client.add_source(path, name))
        .map_err(|e| t!("error-add-source", "error" => e.to_string()))?;

    Ok(format!("{}", theme::success(&result)))
}

fn cmd_source_remove(name: Option<&str>) -> CmdResult {
    let name = name.ok_or_else(|| {
        t!(
            "hive-source-missing-name",
            "command" => "remove"
        )
    })?;
    let (client, runtime) = require_daemon_client()?;

    runtime
        .block_on(client.remove_source(name))
        .map_err(|e| t!("error-remove-source", "error" => e.to_string()))?;

    Ok(format!(
        "{}",
        theme::success(&t!("hive-source-removed", "name" => name))
    ))
}

fn cmd_source_reload(name: Option<&str>) -> CmdResult {
    let name = name.ok_or_else(|| {
        t!(
            "hive-source-missing-name",
            "command" => "reload"
        )
    })?;

    use hive_core::DaemonRequest;

    let (client, runtime) = require_daemon_client()?;

    runtime
        .block_on(client.send(DaemonRequest::ReloadSource {
            name: name.to_string(),
        }))
        .map_err(|e| t!("error-reload-source", "error" => e.to_string()))?;

    Ok(format!(
        "{}",
        theme::success(&t!("hive-source-reloaded", "name" => name))
    ))
}

fn cmd_source_enable(name: Option<&str>) -> CmdResult {
    let name = name.ok_or_else(|| {
        t!(
            "hive-source-missing-name",
            "command" => "enable"
        )
    })?;

    use hive_core::DaemonRequest;

    let (client, runtime) = require_daemon_client()?;

    runtime
        .block_on(client.send(DaemonRequest::EnableSource {
            name: name.to_string(),
        }))
        .map_err(|e| t!("error-enable-source", "error" => e.to_string()))?;

    Ok(format!(
        "{}",
        theme::success(&t!("hive-source-enabled", "name" => name))
    ))
}

fn cmd_source_disable(name: Option<&str>) -> CmdResult {
    let name = name.ok_or_else(|| {
        t!(
            "hive-source-missing-name",
            "command" => "disable"
        )
    })?;

    use hive_core::DaemonRequest;

    let (client, runtime) = require_daemon_client()?;

    runtime
        .block_on(client.send(DaemonRequest::DisableSource {
            name: name.to_string(),
        }))
        .map_err(|e| t!("error-disable-source", "error" => e.to_string()))?;

    Ok(format!(
        "{}",
        theme::success(&t!("hive-source-disabled", "name" => name))
    ))
}

fn cmd_doctor() -> CmdResult {
    use hive_core::dns::collect_tlds;
    use hive_core::daemon_defaults::DNS_BIND;

    // Collect proxy hosts from hive.yaml; gracefully skip if no config found
    let hosts: Vec<String> = {
        let current_dir = std::env::current_dir()
            .map_err(|e| t!("error-get-current-dir", "error" => e.to_string()))?;
        let root = hive_core::find_project_root(&current_dir).unwrap_or(current_dir);
        let parser = HiveConfigParser::new(&root);
        if parser.config_exists() {
            parser
                .parse()
                .map(|config| {
                    let mut v: Vec<String> = config
                        .services
                        .values()
                        .filter_map(|s| s.proxy.as_ref())
                        .flat_map(|p| p.endpoints())
                        .filter_map(|ep| ep.host.clone())
                        .collect();
                    v.sort();
                    v.dedup();
                    v
                })
                .unwrap_or_default()
        } else {
            Vec::new()
        }
    };

    if hosts.is_empty() {
        out_info!("{}", t!("doctor-no-hosts-found"));
        return Ok(String::new());
    }

    // Extract TLDs and the DNS port from the bind address
    let tlds = collect_tlds(&hosts);
    let dns_port: u16 = DNS_BIND
        .rsplit(':')
        .next()
        .and_then(|p| p.parse().ok())
        .unwrap_or(15353);

    // Check /etc/resolver/<tld> files (macOS) or show info (Linux/other)
    #[cfg(target_os = "macos")]
    {
        let mut missing_tlds: Vec<String> = tlds
            .iter()
            .filter(|tld| !std::path::Path::new(&format!("/etc/resolver/{}", tld)).exists())
            .cloned()
            .collect();
        missing_tlds.sort();

        for tld in tlds.iter().filter(|t| !missing_tlds.contains(t)) {
            out_success!(
                "  {} /etc/resolver/{}  {}",
                theme::success(theme::icons::SUCCESS),
                tld,
                theme::muted(&t!("doctor-resolver-ok"))
            );
        }
        for tld in &missing_tlds {
            out_warn!(
                "  {} /etc/resolver/{}  {}",
                theme::warning(theme::icons::WARNING),
                tld,
                theme::muted(&t!("doctor-resolver-missing"))
            );
        }
        out_info!("");

        if !missing_tlds.is_empty() {
            let sp = spinner(&t!("doctor-creating-resolvers"));
            let mut fail_info: Option<(String, String)> = None;
            for tld in &missing_tlds {
                let path = format!("/etc/resolver/{}", tld);
                let content = format!("nameserver 127.0.0.1\nport {}\n", dns_port);
                if let Err(e) = sudo_write_file(&path, &content) {
                    fail_info = Some((t!("doctor-resolver-failed", "tld" => tld.as_str()), e));
                    break;
                }
            }
            if let Some((msg, e)) = fail_info {
                sp.fail(Some(&msg), Some(&e));
            } else {
                sp.success(Some(&t!("doctor-resolvers-created")));
            }

            // Flush DNS cache after creating resolver files
            let sp = spinner(&t!("doctor-flushing-dns"));
            match flush_dns_cache() {
                Ok(()) => sp.success(Some(&t!("doctor-dns-flushed"))),
                Err(e) => sp.fail(Some(&t!("doctor-dns-flush-failed")), Some(&e)),
            }
        }
    }

    #[cfg(not(target_os = "macos"))]
    {
        out_info!("{}", t!("doctor-non-macos-hint"));
        let _ = dns_port; // suppress unused warning
        let _ = tlds;
    }

    Ok(String::new())
}

fn sudo_write_file(path: &str, content: &str) -> std::result::Result<(), String> {
    use std::io::Write;

    // Ensure parent directory exists
    if let Some(parent) = std::path::Path::new(path).parent() {
        if !parent.exists() {
            std::process::Command::new("sudo")
                .args(["mkdir", "-p", &parent.to_string_lossy()])
                .status()
                .map_err(|e| e.to_string())?;
        }
    }

    let mut child = std::process::Command::new("sudo")
        .args(["tee", path])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .spawn()
        .map_err(|e| e.to_string())?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(content.as_bytes()).map_err(|e| e.to_string())?;
    }
    let status = child.wait().map_err(|e| e.to_string())?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("sudo tee {} exited with {}", path, status))
    }
}

fn flush_dns_cache() -> std::result::Result<(), String> {
    std::process::Command::new("sudo")
        .args(["dscacheutil", "-flushcache"])
        .status()
        .map_err(|e| e.to_string())?;
    std::process::Command::new("sudo")
        .args(["killall", "-HUP", "mDNSResponder"])
        .status()
        .map_err(|e| e.to_string())?;
    Ok(())
}
