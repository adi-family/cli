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

// Service traits
pub mod cli;
pub mod http;
pub mod mcp;

// Orchestration traits
pub mod runner;
pub mod health;
pub mod env;
pub mod proxy;
pub mod obs;
pub mod rollout;

// Error handling
mod error;
pub use error::{PluginError, Result};

// Common utilities
pub mod utils;
