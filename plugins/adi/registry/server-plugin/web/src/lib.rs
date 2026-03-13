mod server;

use lib_plugin_abi_v3::{
    async_trait,
    cli::{CliCommand, CliCommands, CliContext, CliResult},
    Plugin, PluginContext, PluginMetadata, PluginType, Result as PluginResult,
    SERVICE_CLI_COMMANDS,
};

pub struct WebRegistryPlugin;

impl WebRegistryPlugin {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Plugin for WebRegistryPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            id: "cli.adi.web-registry-server".to_string(),
            name: "Web Registry Server".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            plugin_type: PluginType::Extension,
            author: Some("ADI Team".to_string()),
            description: Some("Web plugin registry server".to_string()),
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
impl CliCommands for WebRegistryPlugin {
    async fn list_commands(&self) -> Vec<CliCommand> {
        vec![CliCommand {
            name: "start".to_string(),
            description: "Start a web plugin registry server (Ctrl+C to stop)".to_string(),
            args: vec![],
            has_subcommands: false,
        }]
    }

    async fn run_command(&self, ctx: &CliContext) -> PluginResult<CliResult> {
        let subcommand = ctx.subcommand.as_deref().unwrap_or("");
        match subcommand {
            "start" => {
                let port: u16 = ctx
                    .option::<u16>("port")
                    .or_else(|| ctx.option::<String>("port").and_then(|s| s.parse().ok()))
                    .or_else(|| ctx.args.first().and_then(|s| s.parse().ok()))
                    .unwrap_or(8020);

                if let Err(e) = server::run_server(port) {
                    return Ok(CliResult::error(format!("Web registry server failed: {e}")));
                }
                Ok(CliResult::success("Web registry server stopped"))
            }
            _ => Ok(CliResult::error(format!(
                "Unknown command: {subcommand}\nUsage: adi web-registry-server start [--port PORT]"
            ))),
        }
    }
}

#[no_mangle]
pub extern "C" fn plugin_abi_version() -> u32 {
    lib_plugin_abi_v3::PLUGIN_API_VERSION
}

#[no_mangle]
pub fn plugin_create() -> Box<dyn Plugin> {
    Box::new(WebRegistryPlugin::new())
}

#[no_mangle]
pub fn plugin_create_cli() -> Box<dyn CliCommands> {
    Box::new(WebRegistryPlugin::new())
}
