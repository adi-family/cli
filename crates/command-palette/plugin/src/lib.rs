use lib_plugin_prelude::*;

pub struct CommandPalettePlugin;

#[async_trait]
impl Plugin for CommandPalettePlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata::new(
            "adi.command-palette",
            "Command Palette",
            env!("CARGO_PKG_VERSION"),
        )
        .with_type(PluginType::Core)
        .with_author("ADI Team")
        .with_description("Command palette for quick command search and execution")
    }

    async fn init(&mut self, _ctx: &PluginContext) -> Result<()> {
        Ok(())
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    fn provides(&self) -> Vec<&'static str> {
        vec![]
    }
}

#[no_mangle]
pub fn plugin_create() -> Box<dyn Plugin> {
    Box::new(CommandPalettePlugin)
}
