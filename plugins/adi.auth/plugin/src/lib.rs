use lib_plugin_prelude::*;

#[derive(CliArgs)]
pub struct StartArgs {
    #[arg(long, default = 8012)]
    pub port: u16,
}

pub struct AuthPlugin;

impl AuthPlugin {
    pub fn new() -> Self {
        Self
    }
}

impl Default for AuthPlugin {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Plugin for AuthPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata::new("adi.auth", t!("plugin-name"), env!("CARGO_PKG_VERSION"))
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
impl CliCommands for AuthPlugin {
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

impl AuthPlugin {
    fn help(&self) -> String {
        format!(
            "{}\n\n{}\n  start    {}\n\n{}",
            t!("auth-help-title"),
            t!("auth-help-commands"),
            t!("cmd-start-help"),
            t!("auth-help-usage"),
        )
    }

    #[command(name = "start", description = "cmd-start-help")]
    async fn start(&self, args: StartArgs) -> CmdResult {
        auth_http::run_server(args.port)
            .map_err(|e| format!("Auth server failed: {e}"))?;
        Ok("Auth server stopped".to_string())
    }
}

#[no_mangle]
pub fn plugin_create() -> Box<dyn Plugin> {
    Box::new(AuthPlugin::new())
}

#[no_mangle]
pub fn plugin_create_cli() -> Box<dyn CliCommands> {
    Box::new(AuthPlugin::new())
}
