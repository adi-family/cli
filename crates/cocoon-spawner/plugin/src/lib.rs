use cocoon_spawner_core::SpawnerConfig;
use lib_console_output::{out_error, out_info, out_success, theme, KeyValue, Renderable};
use lib_plugin_prelude::*;

fn init_plugin_i18n() {
    lib_plugin_prelude::init_plugin_i18n("en-US", include_str!("../locales/en-US/messages.ftl"));
}

pub struct CocoonSpawnerPlugin;

impl CocoonSpawnerPlugin {
    pub fn new() -> Self {
        Self
    }
}

impl Default for CocoonSpawnerPlugin {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Plugin for CocoonSpawnerPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata::new(
            "adi.cocoon-spawner",
            t!("plugin-name"),
            env!("CARGO_PKG_VERSION"),
        )
        .with_type(PluginType::Core)
        .with_author(t!("plugin-author"))
        .with_description(t!("plugin-description"))
    }

    async fn init(&mut self, _ctx: &PluginContext) -> Result<()> {
        init_plugin_i18n();
        Ok(())
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    fn provides(&self) -> Vec<&'static str> {
        vec![SERVICE_CLI_COMMANDS, SERVICE_DAEMON_SERVICE]
    }
}

#[async_trait]
impl CliCommands for CocoonSpawnerPlugin {
    async fn list_commands(&self) -> Vec<CliCommand> {
        vec![
            Self::__sdk_cmd_meta_run(),
            Self::__sdk_cmd_meta_status(),
            Self::__sdk_cmd_meta_list(),
        ]
    }

    async fn run_command(&self, ctx: &CliContext) -> Result<CliResult> {
        match ctx.subcommand.as_deref() {
            Some("run") => self.__sdk_cmd_handler_run(ctx).await,
            Some("status") => self.__sdk_cmd_handler_status(ctx).await,
            Some("list") => self.__sdk_cmd_handler_list(ctx).await,
            Some("") | Some("help") | None => Ok(CliResult::success(self.help())),
            Some(cmd) => Ok(CliResult::error(t!(
                "error-unknown-command",
                "cmd" => cmd
            ))),
        }
    }
}

impl CocoonSpawnerPlugin {
    fn help(&self) -> String {
        format!(
            "{}\n\n{}\n  {}\n  {}\n  {}",
            t!("spawner-help-title"),
            t!("spawner-help-usage-section"),
            t!("spawner-help-run-usage"),
            t!("spawner-help-status-usage"),
            t!("spawner-help-list-usage"),
        )
    }

    #[command(name = "run", description = "Start spawner in foreground")]
    async fn run(&self) -> CmdResult {
        run_with_runtime(async {
            let config = SpawnerConfig::from_env().map_err(|e| {
                out_error!("{}", t!("error-config", "error" => e.to_string()));
                e.to_string()
            })?;

            out_info!("{}", t!("spawner-starting"));

            let kv = KeyValue::new()
                .entry(t!("label-hive-id"), &config.hive_id)
                .entry(
                    t!("label-kinds"),
                    config
                        .kinds
                        .iter()
                        .map(|k| k.kind.id.as_str())
                        .collect::<Vec<_>>()
                        .join(", "),
                )
                .entry(t!("label-max"), config.max_concurrent.to_string());
            kv.print();

            cocoon_spawner_core::run(config)
                .await
                .map_err(|e| e.to_string())?;

            out_success!("{}", t!("spawner-stopped"));
            Ok("Spawner stopped".to_string())
        })
    }

    #[command(name = "status", description = "Show spawner status")]
    async fn status(&self) -> CmdResult {
        let kv = KeyValue::new().entry(
            t!("label-status"),
            theme::muted(t!("spawner-status-stopped")).to_string(),
        );
        kv.print();
        Ok("Status shown".to_string())
    }

    #[command(name = "list", description = "List active cocoons")]
    async fn list(&self) -> CmdResult {
        out_info!("{}", t!("spawner-list-empty"));
        Ok("Listed".to_string())
    }
}

#[daemon_service]
impl CocoonSpawnerPlugin {
    async fn start(&self, _ctx: DaemonContext) -> Result<()> {
        let _ = tracing_subscriber::fmt()
            .with_env_filter(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| "cocoon_spawner_core=info".into()),
            )
            .with_ansi(false)
            .try_init();

        let (tx, rx) = tokio::sync::oneshot::channel::<std::result::Result<(), String>>();

        std::thread::Builder::new()
            .name("cocoon-spawner-daemon".into())
            .spawn(move || {
                let rt = match tokio::runtime::Builder::new_multi_thread()
                    .enable_all()
                    .build()
                {
                    Ok(rt) => rt,
                    Err(e) => {
                        let _ = tx.send(Err(t!(
                            "error-build-tokio-runtime",
                            "error" => e.to_string()
                        )));
                        return;
                    }
                };

                let result = rt.block_on(async {
                    let config = SpawnerConfig::from_env().map_err(|e| e.to_string())?;
                    cocoon_spawner_core::run(config)
                        .await
                        .map_err(|e| e.to_string())
                });

                let _ = tx.send(result);
            })
            .map_err(|e| {
                anyhow::anyhow!(t!(
                    "error-spawn-daemon-thread",
                    "error" => e.to_string()
                ))
            })?;

        rx.await
            .map_err(|_| anyhow::anyhow!(t!("error-daemon-thread-terminated")))?
            .map_err(|e| anyhow::anyhow!("{}", e).into())
    }
}

fn run_with_runtime<F: std::future::Future<Output = CmdResult> + Send + 'static>(
    fut: F,
) -> CmdResult {
    std::thread::spawn(move || {
        tokio::runtime::Runtime::new()
            .map_err(|e| format!("Failed to create runtime: {e}"))?
            .block_on(fut)
    })
    .join()
    .map_err(|_| "Async task panicked".to_string())?
}

#[no_mangle]
pub extern "C" fn plugin_abi_version() -> u32 {
    lib_plugin_abi_v3::PLUGIN_API_VERSION
}

#[no_mangle]
pub fn plugin_create() -> Box<dyn Plugin> {
    Box::new(CocoonSpawnerPlugin::new())
}

#[no_mangle]
pub fn plugin_create_cli() -> Box<dyn CliCommands> {
    Box::new(CocoonSpawnerPlugin::new())
}
