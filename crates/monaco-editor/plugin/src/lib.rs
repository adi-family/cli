use lib_plugin_prelude::*;

pub struct MonacoEditorPlugin;

#[async_trait]
impl Plugin for MonacoEditorPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata::new(
            "adi.monaco-editor",
            "Monaco Editor",
            env!("CARGO_PKG_VERSION"),
        )
        .with_type(PluginType::Core)
        .with_author("ADI Team")
        .with_description("Web-based code editor powered by Monaco")
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
    Box::new(MonacoEditorPlugin)
}
