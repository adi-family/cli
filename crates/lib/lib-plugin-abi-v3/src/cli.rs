//! CLI commands service trait

use crate::{Plugin, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;

/// CLI commands service trait
///
/// Plugins implementing this trait can provide CLI commands that extend
/// the `adi` CLI with custom functionality.
///
/// # Example
///
/// ```rust
/// use lib_plugin_abi_v3::*;
///
/// pub struct TasksPlugin;
///
/// #[async_trait]
/// impl CliCommands for TasksPlugin {
///     async fn list_commands(&self) -> Vec<CliCommand> {
///         vec![
///             CliCommand {
///                 name: "list".to_string(),
///                 description: "List all tasks".to_string(),
///                 usage: "tasks list [--status <status>]".to_string(),
///                 has_subcommands: false,
///             },
///             CliCommand {
///                 name: "create".to_string(),
///                 description: "Create a new task".to_string(),
///                 usage: "tasks create <title>".to_string(),
///                 has_subcommands: false,
///             },
///         ]
///     }
///
///     async fn run_command(&self, ctx: &CliContext) -> Result<CliResult> {
///         match ctx.subcommand.as_deref() {
///             Some("list") => {
///                 // List tasks logic
///                 Ok(CliResult::success("Task 1\nTask 2\n"))
///             }
///             Some("create") => {
///                 // Create task logic
///                 Ok(CliResult::success("Task created!"))
///             }
///             _ => Ok(CliResult::error("Unknown command")),
///         }
///     }
/// }
/// ```
#[async_trait]
pub trait CliCommands: Plugin {
    /// List all CLI commands provided by this plugin
    async fn list_commands(&self) -> Vec<CliCommand>;

    /// Execute a CLI command
    async fn run_command(&self, ctx: &CliContext) -> Result<CliResult>;
}

/// CLI command metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CliCommand {
    /// Command name (e.g., "list")
    pub name: String,

    /// Human-readable description
    pub description: String,

    /// Usage string (e.g., "tasks list [--filter <filter>]")
    pub usage: String,

    /// Whether this command has subcommands
    pub has_subcommands: bool,
}

/// CLI execution context
#[derive(Debug, Clone)]
pub struct CliContext {
    /// Command name (e.g., "tasks")
    pub command: String,

    /// Subcommand name (e.g., "list")
    pub subcommand: Option<String>,

    /// Positional arguments
    pub args: Vec<String>,

    /// Parsed flags and options
    pub options: HashMap<String, Value>,

    /// Current working directory
    pub cwd: PathBuf,

    /// Environment variables
    pub env: HashMap<String, String>,
}

impl CliContext {
    /// Get a string argument by index
    pub fn arg(&self, index: usize) -> Option<&str> {
        self.args.get(index).map(|s| s.as_str())
    }

    /// Get an option value
    pub fn option<T>(&self, key: &str) -> Option<T>
    where
        T: serde::de::DeserializeOwned,
    {
        self.options
            .get(key)
            .and_then(|v| serde_json::from_value(v.clone()).ok())
    }

    /// Check if a flag is set
    pub fn has_flag(&self, key: &str) -> bool {
        self.options
            .get(key)
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
    }
}

/// CLI command result
#[derive(Debug, Clone)]
pub struct CliResult {
    /// Exit code (0 = success)
    pub exit_code: i32,

    /// Standard output
    pub stdout: String,

    /// Standard error
    pub stderr: String,
}

impl CliResult {
    /// Create a successful result with output
    pub fn success(output: impl Into<String>) -> Self {
        Self {
            exit_code: 0,
            stdout: output.into(),
            stderr: String::new(),
        }
    }

    /// Create an error result with message
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            exit_code: 1,
            stdout: String::new(),
            stderr: message.into(),
        }
    }

    /// Create a custom result
    pub fn custom(exit_code: i32, stdout: impl Into<String>, stderr: impl Into<String>) -> Self {
        Self {
            exit_code,
            stdout: stdout.into(),
            stderr: stderr.into(),
        }
    }
}
