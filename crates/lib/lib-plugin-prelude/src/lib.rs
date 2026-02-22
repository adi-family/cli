//! # lib-plugin-prelude: Plugin Prelude for ADI Ecosystem
//!
//! One-stop import for plugin development. Re-exports all necessary
//! macros and types for writing ADI plugins.
//!
//! ## Usage
//!
//! ```rust,ignore
//! use lib_plugin_prelude::*;
//!
//! #[plugin]
//! pub struct MyPlugin {
//!     counter: u32,
//! }
//!
//! impl Plugin for MyPlugin {
//!     fn metadata(&self) -> PluginMetadata {
//!         PluginMetadata::new("adi.myplugin", "My Plugin", "1.0.0")
//!     }
//!
//!     async fn init(&mut self, ctx: &PluginContext) -> Result<()> {
//!         Ok(())
//!     }
//! }
//!
//! #[async_trait]
//! impl CliCommands for MyPlugin {
//!     async fn list_commands(&self) -> Vec<CliCommand> {
//!         vec![Self::__sdk_cmd_meta_count()]
//!     }
//!
//!     async fn run_command(&self, ctx: &CliContext) -> Result<CliResult> {
//!         match ctx.subcommand.as_deref() {
//!             Some("count") => self.count(ctx.option("by")).await,
//!             _ => Ok(CliResult::error("Unknown command"))
//!         }
//!     }
//! }
//!
//! impl MyPlugin {
//!     #[command(name = "count")]
//!     async fn count(&self, by: Option<u32>) -> std::result::Result<String, String> {
//!         let step = by.unwrap_or(1);
//!         Ok(format!("Counter: {}", self.counter + step))
//!     }
//! }
//! ```

// === Static Plugin Context ===
mod ctx;
pub use ctx::PluginCtx;

// === SDK Macros ===
pub use lib_plugin_sdk::{
    command, daemon_cmd, daemon_service, daemon_sudo, global_command, http_routes, plugin,
    webrtc_handlers,
};

// Re-export derive macro - users write #[derive(CliArgs)]
// This shadows the trait in derive position only
pub use lib_plugin_sdk::CliArgs;

// === Core Plugin Types ===
pub use lib_plugin_abi_v3::{
    // Async support
    async_trait,
    // CLI types - CliArgs trait available for explicit use
    cli::{
        CliArg, CliArgType, CliArgs as CliArgsTrait, CliCommand, CliCommands, CliContext, CliResult,
    },
    // Daemon types
    daemon::{
        DaemonClient, DaemonCommand, DaemonCommandResult, DaemonContext, DaemonService,
        GlobalCommands, ServiceStatus,
    },
    // HTTP types
    http::{HttpMethod, HttpRequest, HttpResponse, HttpRoute, HttpRoutes},
    // WebRTC types
    webrtc::{Message, Peer, WebRtcHandlers},
    // Core plugin traits
    Plugin,
    PluginCategory,
    PluginContext,
    // Error types
    PluginError,
    PluginEvent,
    PluginMetadata,
    PluginType,
    Result,
    // Service identifiers
    SERVICE_CLI_COMMANDS,
    SERVICE_DAEMON_SERVICE,
    SERVICE_GLOBAL_COMMANDS,
    SERVICE_HTTP_ROUTES,
    SERVICE_WEBRTC_HANDLERS,
};

// === Translations ===
pub use lib_i18n_core::t;

/// Convenience initializer for plugin-local i18n.
///
/// cdylib plugins have their own copy of the `lib-i18n-core` static, so the
/// host's `init_global()` does not reach into their address space. Each plugin
/// must initialize its own copy.
pub fn init_plugin_i18n(default_lang: &str, ftl_content: &str) {
    let mut i18n = lib_i18n_core::I18n::new_standalone();
    let _ = i18n.load_embedded(default_lang, ftl_content);
    let _ = i18n.set_language(default_lang);
    lib_i18n_core::init_global(i18n);
}

// === Async Support ===
pub use tokio::sync::RwLock;

/// Command result type alias for convenience
pub type CmdResult = std::result::Result<String, String>;
