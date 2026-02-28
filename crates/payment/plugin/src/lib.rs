use lib_plugin_abi_v3::{
    async_trait,
    cli::{CliCommand, CliCommands, CliContext, CliResult},
    Plugin, PluginContext, PluginMetadata, PluginType, Result as PluginResult,
    SERVICE_CLI_COMMANDS,
};

pub struct PaymentPlugin;

impl PaymentPlugin {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Plugin for PaymentPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            id: "adi.payment".to_string(),
            name: "Payment".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            plugin_type: PluginType::Extension,
            author: Some("ADI Team".to_string()),
            description: Some("Payment HTTP server".to_string()),
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
impl CliCommands for PaymentPlugin {
    async fn list_commands(&self) -> Vec<CliCommand> {
        vec![CliCommand {
            name: "start".to_string(),
            description: "Start the Payment HTTP server (Ctrl+C to stop)".to_string(),
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
                    .unwrap_or(8040);

                if let Err(e) = payment_core::run_server(port) {
                    return Ok(CliResult::error(format!("Payment server failed: {e}")));
                }
                Ok(CliResult::success("Payment server stopped"))
            }
            _ => Ok(CliResult::error(format!(
                "Unknown command: {subcommand}\nUsage: adi run adi.payment start [--port PORT]"
            ))),
        }
    }
}

#[no_mangle]
pub fn plugin_create() -> Box<dyn Plugin> {
    Box::new(PaymentPlugin::new())
}

#[no_mangle]
pub fn plugin_create_cli() -> Box<dyn CliCommands> {
    Box::new(PaymentPlugin::new())
}
