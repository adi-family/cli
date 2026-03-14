pub mod server;
pub mod ws;

use lib_plugin_prelude::*;

pub struct SignalingPlugin;

impl SignalingPlugin {
    pub fn new() -> Self {
        Self
    }

    fn help(&self) -> String {
        format!(
            "{}\n\n{}\n  \
             start    {}\n  \
             status   {}\n  \
             pair     {}\n  \
             devices  {}\n\n\
             {}",
            t!("plugin-help-title"),
            t!("plugin-help-commands"),
            t!("cmd-start-help"),
            t!("cmd-status-help"),
            t!("cmd-pair-help"),
            t!("cmd-devices-help"),
            t!("plugin-help-usage"),
        )
    }
}

impl Default for SignalingPlugin {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Plugin for SignalingPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata::new("adi.signaling", t!("plugin-name"), env!("CARGO_PKG_VERSION"))
            .with_type(PluginType::Core)
            .with_author(t!("plugin-author"))
            .with_description(t!("plugin-description"))
    }

    async fn init(&mut self, _ctx: &PluginContext) -> Result<()> {
        lib_plugin_prelude::init_plugin_i18n(
            "en-US",
            include_str!("../locales/en-US/messages.ftl"),
        );
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
impl CliCommands for SignalingPlugin {
    async fn list_commands(&self) -> Vec<CliCommand> {
        vec![
            CliCommand {
                name: "start".to_string(),
                description: t!("cmd-start-help"),
                args: vec![CliArg::optional("--port", CliArgType::String)],
                has_subcommands: false,
            },
            Self::__sdk_cmd_meta_status(),
            Self::__sdk_cmd_meta_pair(),
            Self::__sdk_cmd_meta_devices(),
        ]
    }

    async fn run_command(&self, ctx: &CliContext) -> Result<CliResult> {
        match ctx.subcommand.as_deref() {
            Some("start") => {
                let port: u16 = ctx
                    .option::<u16>("port")
                    .or_else(|| ctx.option::<String>("port").and_then(|s| s.parse().ok()))
                    .or_else(|| ctx.args.first().and_then(|s| s.parse().ok()))
                    .unwrap_or(8080);

                if let Err(e) = server::run_server(port) {
                    return Ok(CliResult::error(format!("Signaling server failed: {e}")));
                }
                Ok(CliResult::success("Signaling server stopped"))
            }
            Some("status") => self.__sdk_cmd_handler_status(ctx).await,
            Some("pair") => self.__sdk_cmd_handler_pair(ctx).await,
            Some("devices") => self.__sdk_cmd_handler_devices(ctx).await,
            Some(cmd) => Ok(CliResult::error(format!("Unknown command: {}", cmd))),
            None => Ok(CliResult::success(self.help())),
        }
    }
}

impl SignalingPlugin {
    #[command(name = "status", description = "cmd-status-help")]
    async fn status(&self) -> CmdResult {
        Ok(t!("not-implemented"))
    }

    #[command(name = "pair", description = "cmd-pair-help")]
    async fn pair(&self) -> CmdResult {
        Ok(t!("not-implemented"))
    }

    #[command(name = "devices", description = "cmd-devices-help")]
    async fn devices(&self) -> CmdResult {
        Ok(t!("not-implemented"))
    }
}

#[no_mangle]
pub fn plugin_create() -> Box<dyn Plugin> {
    Box::new(SignalingPlugin::new())
}

#[no_mangle]
pub fn plugin_create_cli() -> Box<dyn CliCommands> {
    Box::new(SignalingPlugin::new())
}
