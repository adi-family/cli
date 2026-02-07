//! ADI Workflow Plugin (v3)
//!
//! Run workflows defined in TOML files with interactive prompts and templating.

mod cli_impl;
mod discovery;
mod executor;
mod options;
mod parser;
mod prelude;
mod prompts;
mod template;

use lib_plugin_abi_v3::*;
use lib_plugin_abi_v3::cli::{CliCommand, CliCommands, CliContext, CliResult};

pub struct WorkflowPlugin;

#[async_trait]
impl Plugin for WorkflowPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            id: "adi.workflow".to_string(),
            name: "ADI Workflow".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            plugin_type: PluginType::Core,
            author: Some("ADI Team".to_string()),
            description: Some("Run workflows defined in TOML files with interactive prompts".to_string()),
            category: None,
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
impl CliCommands for WorkflowPlugin {
    async fn list_commands(&self) -> Vec<CliCommand> {
        vec![
            CliCommand {
                name: "run".to_string(),
                description: "Run a workflow by name".to_string(),
                usage: "workflow run <name> [--arg=value]".to_string(),
                has_subcommands: false,
            },
            CliCommand {
                name: "list".to_string(),
                description: "List available workflows".to_string(),
                usage: "workflow list".to_string(),
                has_subcommands: false,
            },
            CliCommand {
                name: "show".to_string(),
                description: "Show workflow definition".to_string(),
                usage: "workflow show <name>".to_string(),
                has_subcommands: false,
            },
            CliCommand {
                name: "--completions".to_string(),
                description: "Output completion suggestions (internal use)".to_string(),
                usage: "workflow --completions <position> [args...]".to_string(),
                has_subcommands: false,
            },
        ]
    }

    async fn run_command(&self, ctx: &CliContext) -> Result<CliResult> {
        // Build full args list: [subcommand, ...remaining_args]
        let mut full_args = Vec::new();
        if let Some(ref subcmd) = ctx.subcommand {
            full_args.push(subcmd.clone());
        }
        full_args.extend(ctx.args.iter().cloned());

        // Convert context to JSON format expected by cli_impl::run_command
        let context_json = serde_json::json!({
            "command": &ctx.command,
            "args": &full_args,
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
    Box::new(WorkflowPlugin)
}

#[no_mangle]
pub fn plugin_create_cli() -> Box<dyn CliCommands> {
    Box::new(WorkflowPlugin)
}
