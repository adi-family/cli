use lib_plugin_abi_v3::{
    async_trait,
    cli::{CliCommand, CliCommands, CliContext, CliResult},
    Plugin, PluginContext, PluginMetadata, PluginType, Result as PluginResult,
    SERVICE_CLI_COMMANDS,
};

pub struct CredentialsPlugin;

impl CredentialsPlugin {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Plugin for CredentialsPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            id: "adi.credentials".to_string(),
            name: "Credentials".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            plugin_type: PluginType::Extension,
            author: Some("ADI Team".to_string()),
            description: Some("Credentials HTTP server".to_string()),
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
impl CliCommands for CredentialsPlugin {
    async fn list_commands(&self) -> Vec<CliCommand> {
        vec![CliCommand {
            name: "start".to_string(),
            description: "Start the Credentials HTTP server (Ctrl+C to stop)".to_string(),
            args: vec![],
            has_subcommands: false,
        }]
    }

    async fn run_command(&self, ctx: &CliContext) -> PluginResult<CliResult> {
        let subcommand = ctx.subcommand.as_deref().unwrap_or("");

        match subcommand {
            "start" => {
                Ok(CliResult::error("HTTP server has been removed. Use the credentials core API directly."))
            }
            _ => Ok(CliResult::error(format!(
                "Unknown command: {subcommand}\nUsage: adi run adi.credentials start [--port PORT]"
            ))),
        }
    }
}

#[no_mangle]
pub fn plugin_create() -> Box<dyn Plugin> {
    Box::new(CredentialsPlugin::new())
}

#[no_mangle]
pub fn plugin_create_cli() -> Box<dyn CliCommands> {
    Box::new(CredentialsPlugin::new())
}
