use lib_plugin_prelude::*;

pub struct EmbedProxyPlugin;

impl EmbedProxyPlugin {
    pub fn new() -> Self {
        Self
    }
}

impl Default for EmbedProxyPlugin {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Plugin for EmbedProxyPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata::new("adi.embed-proxy", "Embed Proxy", env!("CARGO_PKG_VERSION"))
            .with_type(PluginType::Extension)
            .with_author("ADI Team")
            .with_description("Embedding API proxy management with BYOK/Platform modes")
    }

    async fn init(&mut self, _ctx: &PluginContext) -> Result<()> {
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
impl CliCommands for EmbedProxyPlugin {
    async fn list_commands(&self) -> Vec<CliCommand> {
        vec![]
    }

    async fn run_command(&self, ctx: &CliContext) -> Result<CliResult> {
        match ctx.subcommand.as_deref() {
            Some(cmd) => Ok(CliResult::error(format!("Unknown command: {cmd}"))),
            None => Ok(CliResult::success("Embed Proxy plugin (web UI only)".to_string())),
        }
    }
}

#[no_mangle]
pub fn plugin_create() -> Box<dyn Plugin> {
    Box::new(EmbedProxyPlugin::new())
}

#[no_mangle]
pub fn plugin_create_cli() -> Box<dyn CliCommands> {
    Box::new(EmbedProxyPlugin::new())
}
