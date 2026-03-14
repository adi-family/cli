use lib_plugin_prelude::*;

pub struct DebugScreenPlugin;

#[async_trait]
impl Plugin for DebugScreenPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata::new(
            "adi.debug-screen",
            "Debug Screen",
            env!("CARGO_PKG_VERSION"),
        )
        .with_type(PluginType::Core)
        .with_author("ADI Team")
        .with_description("Debug screen and operations log")
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
    Box::new(DebugScreenPlugin)
}
