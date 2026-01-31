//! # lib-plugin-abi-v3: Unified Plugin ABI
//!
//! This crate provides a unified plugin ABI for the ADI ecosystem, replacing
//! both `lib-plugin-abi` (FFI-safe) and `lib-plugin-abi-orchestration` (async traits).
//!
//! ## Design Principles
//!
//! - **Native Rust async traits** - No FFI complexity, simple and idiomatic
//! - **Trait composition** - Base `Plugin` trait + service-specific traits
//! - **Type safety** - Strongly typed contexts, no JSON strings
//! - **Zero-cost abstractions** - Direct function calls, no overhead
//!
//! ## Example
//!
//! ```rust
//! use lib_plugin_abi_v3::*;
//! use async_trait::async_trait;
//!
//! pub struct MyPlugin;
//!
//! impl Plugin for MyPlugin {
//!     fn metadata(&self) -> PluginMetadata {
//!         PluginMetadata {
//!             id: "adi.myplugin".to_string(),
//!             name: "My Plugin".to_string(),
//!             version: "1.0.0".to_string(),
//!             plugin_type: PluginType::Extension,
//!             author: Some("Me".to_string()),
//!             description: Some("Does things".to_string()),
//!         }
//!     }
//!
//!     async fn init(&mut self, ctx: &PluginContext) -> Result<()> {
//!         Ok(())
//!     }
//!
//!     async fn shutdown(&self) -> Result<()> {
//!         Ok(())
//!     }
//! }
//!
//! #[async_trait]
//! impl CliCommands for MyPlugin {
//!     async fn list_commands(&self) -> Vec<CliCommand> {
//!         vec![
//!             CliCommand {
//!                 name: "hello".to_string(),
//!                 description: "Say hello".to_string(),
//!                 usage: "myplugin hello".to_string(),
//!                 has_subcommands: false,
//!             }
//!         ]
//!     }
//!
//!     async fn run_command(&self, ctx: &CliContext) -> Result<CliResult> {
//!         Ok(CliResult::success("Hello!"))
//!     }
//! }
//!
//! // Export plugin
//! #[no_mangle]
//! pub fn plugin_create() -> Box<dyn Plugin> {
//!     Box::new(MyPlugin)
//! }
//! ```

// Re-export async_trait for convenience
pub use async_trait::async_trait;

// Core traits and types
mod core;
pub use core::*;

// Service traits (CLI plugins)
pub mod cli;
pub mod http;
pub mod mcp;

// Language analyzer traits (Indexer plugins)
pub mod lang;

// Embedding traits (Embedder plugins)
pub mod embed;

// Orchestration traits (Hive plugins)
pub mod runner;
pub mod health;
pub mod env;
pub mod proxy;
pub mod obs;
pub mod rollout;
pub mod hooks;

// Error handling
mod error;
pub use error::{PluginError, Result};

// Common utilities
pub mod utils;

/// Plugin API version. Bump on breaking changes.
pub const PLUGIN_API_VERSION: u32 = 3;

/// Symbol name that plugins must export.
pub const PLUGIN_ENTRY_SYMBOL: &str = "plugin_create";

/// Type alias for the plugin entry function.
pub type PluginCreateFn = fn() -> Box<dyn Plugin>;

// Service type identifiers for capability discovery
pub const SERVICE_CLI_COMMANDS: &str = "cli.commands";
pub const SERVICE_HTTP_ROUTES: &str = "http.routes";
pub const SERVICE_MCP_TOOLS: &str = "mcp.tools";
pub const SERVICE_MCP_RESOURCES: &str = "mcp.resources";
pub const SERVICE_MCP_PROMPTS: &str = "mcp.prompts";
pub const SERVICE_LANGUAGE_ANALYZER: &str = "indexer.lang";
pub const SERVICE_EMBEDDER: &str = "indexer.embed";
pub const SERVICE_RUNNER: &str = "orchestration.runner";
pub const SERVICE_HEALTH_CHECK: &str = "orchestration.health";
pub const SERVICE_ENV_PROVIDER: &str = "orchestration.env";
pub const SERVICE_PROXY_MIDDLEWARE: &str = "orchestration.proxy";
pub const SERVICE_OBSERVABILITY_SINK: &str = "orchestration.obs";
pub const SERVICE_ROLLOUT_STRATEGY: &str = "orchestration.rollout";
