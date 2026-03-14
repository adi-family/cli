use lib_plugin_prelude::*;

pub struct RouterPlugin;

#[async_trait]
impl Plugin for RouterPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata::new(
            "adi.router",
            "Router",
            env!("CARGO_PKG_VERSION"),
        )
        .with_type(PluginType::Core)
        .with_author("ADI Team")
        .with_description("Client-side SPA router with navigation management")
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
    Box::new(RouterPlugin)
}
