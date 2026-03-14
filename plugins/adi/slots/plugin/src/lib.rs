use lib_plugin_prelude::*;

pub struct SlotsPlugin;

#[async_trait]
impl Plugin for SlotsPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata::new("adi.slots", "ADI Slots", env!("CARGO_PKG_VERSION"))
            .with_type(PluginType::Core)
            .with_author("ADI Team")
            .with_description("Content slots for cross-plugin UI extension points")
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
    Box::new(SlotsPlugin)
}
