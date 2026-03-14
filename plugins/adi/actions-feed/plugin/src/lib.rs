use lib_plugin_prelude::*;

pub struct ActionsFeedPlugin;

#[async_trait]
impl Plugin for ActionsFeedPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata::new(
            "adi.actions-feed",
            "Actions Feed",
            env!("CARGO_PKG_VERSION"),
        )
        .with_type(PluginType::Core)
        .with_author("ADI Team")
        .with_description("Action feed system for cross-plugin notifications")
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
    Box::new(ActionsFeedPlugin)
}
