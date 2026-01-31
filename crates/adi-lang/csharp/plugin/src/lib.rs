//! C# Language Support Plugin (v3)
//!
//! Language support for C# with tree-sitter parsing 

mod cli_impl;
// Add other modules as needed

use lib_plugin_abi_v3::*;
use lib_plugin_abi_v3::cli::{CliCommand, CliCommands, CliContext, CliResult};

pub struct CSharpLangPlugin;

#[async_trait]
impl Plugin for CSharpLangPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            id: "adi.lang.csharp".to_string(),
            name: "C# Language Support".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            plugin_type: PluginType::Core,
            author: Some("ADI Team".to_string()),
            description: Some("Language support for C# with tree-sitter parsing ".to_string()),
        }
    }

    async fn init(&mut self, _ctx: &PluginContext) -> Result<()> {
        Ok(())
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }
}

#[async_trait]
impl CliCommands for CSharpLangPlugin {
    async fn list_commands(&self) -> Vec<CliCommand> {
        // TODO: Update with actual commands
        vec![]
    }

    async fn run_command(&self, ctx: &CliContext) -> Result<CliResult> {
        // Convert context to JSON format expected by cli_impl::run_command
        let context_json = serde_json::json!({
            "command": &ctx.command,
            "args": &ctx.args,
            "cwd": &ctx.cwd,
        });

        match cli_impl::run_command(&context_json.to_string()) {
            Ok(output) => Ok(CliResult::success(output)),
            Err(e) => Ok(CliResult::error(e.to_string())),
        }
    }
}

#[no_mangle]
pub fn plugin_create() -> Box<dyn Plugin> {
    Box::new(CSharpLangPlugin)
}
