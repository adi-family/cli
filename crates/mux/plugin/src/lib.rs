use lib_plugin_prelude::*;
use std::path::PathBuf;

#[derive(CliArgs)]
pub struct StartArgs {
    #[arg(long, default = 8025)]
    pub port: u16,
    #[arg(long)]
    pub config: Option<String>,
}

pub struct MuxPlugin;

impl MuxPlugin {
    pub fn new() -> Self {
        Self
    }
}

impl Default for MuxPlugin {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Plugin for MuxPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata::new("adi.mux", t!("plugin-name"), env!("CARGO_PKG_VERSION"))
            .with_type(PluginType::Extension)
            .with_author(t!("plugin-author"))
            .with_description(t!("plugin-description"))
    }

    async fn init(&mut self, ctx: &PluginContext) -> Result<()> {
        PluginCtx::init(ctx);
        init_plugin_i18n("en-US", include_str!("../locales/en-US/messages.ftl"));
        Ok(())
    }

    fn provides(&self) -> Vec<&'static str> {
        vec![SERVICE_CLI_COMMANDS]
    }
}

#[async_trait]
impl CliCommands for MuxPlugin {
    async fn list_commands(&self) -> Vec<CliCommand> {
        vec![Self::__sdk_cmd_meta_start()]
    }

    async fn run_command(&self, ctx: &CliContext) -> Result<CliResult> {
        match ctx.subcommand.as_deref() {
            Some("start") => self.__sdk_cmd_handler_start(ctx).await,
            Some(cmd) => Ok(CliResult::error(format!("Unknown command: {cmd}"))),
            None => Ok(CliResult::success(self.help())),
        }
    }
}

impl MuxPlugin {
    fn help(&self) -> String {
        format!(
            "{}\n\n{}\n  start    {}\n\n{}",
            t!("mux-help-title"),
            t!("mux-help-commands"),
            t!("cmd-start-help"),
            t!("mux-help-usage"),
        )
    }

    #[command(name = "start", description = "cmd-start-help")]
    async fn start(&self, args: StartArgs) -> CmdResult {
        let config = args.config.map(PathBuf::from);
        mux_http::run_server(args.port, config)
            .map_err(|e| format!("Mux server failed: {e}"))?;
        Ok("Mux server stopped".to_string())
    }
}

#[no_mangle]
pub fn plugin_create() -> Box<dyn Plugin> {
    Box::new(MuxPlugin::new())
}

#[no_mangle]
pub fn plugin_create_cli() -> Box<dyn CliCommands> {
    Box::new(MuxPlugin::new())
}
