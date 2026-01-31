//! Browser Debug Plugin (v3)
//!
//! Browser debugging - inspect network requests and console logs from browser tabs
//!
//! TODO: Complete v3 migration - implement full functionality from v2.bak

use lib_plugin_abi_v3::*;
use lib_plugin_abi_v3::cli::{CliCommand, CliCommands, CliContext, CliResult};

pub struct BrowserDebugPlugin;

#[async_trait]
impl Plugin for BrowserDebugPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            id: "adi.browser-debug".to_string(),
            name: "Browser Debug".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            plugin_type: PluginType::Core,
            author: Some("ADI Team".to_string()),
            description: Some("Browser debugging - inspect network requests and console logs from browser tabs ".to_string()),
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
impl CliCommands for BrowserDebugPlugin {
    async fn list_commands(&self) -> Vec<CliCommand> {
        vec![
            CliCommand {
                name: "connect".to_string(),
                description: "Connect to browser debugging session".to_string(),
                usage: "browser-debug connect".to_string(),
                has_subcommands: false,
            },
        ]
    }

    async fn run_command(&self, _ctx: &CliContext) -> Result<CliResult> {
        // TODO: Implement full browser debugging from v2.bak
        Ok(CliResult::error("Browser debug plugin not yet fully migrated to v3. See lib.rs.v2.bak for reference."))
    }
}

#[no_mangle]
pub fn plugin_create() -> Box<dyn Plugin> {
    Box::new(BrowserDebugPlugin)
}

#[no_mangle]
pub fn plugin_create_cli() -> Box<dyn CliCommands> {
    Box::new(BrowserDebugPlugin)
}
