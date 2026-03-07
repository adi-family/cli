use lib_plugin_prelude::*;

pub struct CocoonControlCenterPlugin;

impl CocoonControlCenterPlugin {
    pub fn new() -> Self {
        Self
    }
}

impl Default for CocoonControlCenterPlugin {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Plugin for CocoonControlCenterPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata::new(
            "adi.cocoon-control-center",
            t!("plugin-name"),
            env!("CARGO_PKG_VERSION"),
        )
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
        vec![]
    }
}

#[async_trait]
impl CliCommands for CocoonControlCenterPlugin {
    async fn list_commands(&self) -> Vec<CliCommand> {
        vec![]
    }

    async fn run_command(&self, ctx: &CliContext) -> Result<CliResult> {
        match ctx.subcommand.as_deref() {
            Some(cmd) => Ok(CliResult::error(format!("Unknown command: {cmd}"))),
            None => Ok(CliResult::success(String::new())),
        }
    }
}

#[no_mangle]
pub fn plugin_create() -> Box<dyn Plugin> {
    Box::new(CocoonControlCenterPlugin::new())
}

#[no_mangle]
pub fn plugin_create_cli() -> Box<dyn CliCommands> {
    Box::new(CocoonControlCenterPlugin::new())
}
