//! ADI Agent Loop Plugin
//!
//! Provides CLI commands for autonomous LLM agents.

use agent_loop_core::{
    AgentLoop, ApprovalDecision, ApprovalHandler, LoopConfig, Message, MockLlmProvider,
    PermissionRule, ToolCall,
};
use async_trait::async_trait;
use console::style;
use dialoguer::{theme::ColorfulTheme, Select};
use lib_plugin_abi_v3::{
    cli::{CliCommand, CliCommands, CliContext, CliResult},
    Plugin, PluginContext, PluginMetadata, PluginType, Result as PluginResult, SERVICE_CLI_COMMANDS,
};
use once_cell::sync::OnceCell;
use std::sync::Arc;
use tokio::runtime::Runtime;

/// Global tokio runtime for async operations
static RUNTIME: OnceCell<Runtime> = OnceCell::new();

fn get_runtime() -> &'static Runtime {
    RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("Failed to create tokio runtime")
    })
}

/// Agent Loop Plugin
pub struct AgentLoopPlugin;

impl AgentLoopPlugin {
    pub fn new() -> Self {
        Self
    }
}

impl Default for AgentLoopPlugin {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Plugin for AgentLoopPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            id: "adi.agent-loop".to_string(),
            name: "ADI Agent Loop".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            plugin_type: PluginType::Core,
            author: Some("ADI Team".to_string()),
            description: Some("Autonomous LLM agent with tool execution".to_string()),
            category: None,
        }
    }

    async fn init(&mut self, _ctx: &PluginContext) -> PluginResult<()> {
        let _ = get_runtime();
        Ok(())
    }

    async fn shutdown(&self) -> PluginResult<()> {
        Ok(())
    }

    fn provides(&self) -> Vec<&'static str> {
        vec![SERVICE_CLI_COMMANDS]
    }
}

#[async_trait]
impl CliCommands for AgentLoopPlugin {
    async fn list_commands(&self) -> Vec<CliCommand> {
        vec![
            CliCommand {
                name: "run".to_string(),
                description: "Run agent with a task".to_string(),
                usage: "run <task> [--max-iterations <n>] [--yes] [--file <path>] [--system-prompt <prompt>]".to_string(),
                has_subcommands: false,
            },
            CliCommand {
                name: "config".to_string(),
                description: "Manage configuration".to_string(),
                usage: "config [show|set <key> <value>]".to_string(),
                has_subcommands: true,
            },
            CliCommand {
                name: "tools".to_string(),
                description: "List available tools".to_string(),
                usage: "tools [list]".to_string(),
                has_subcommands: true,
            },
        ]
    }

    async fn run_command(&self, ctx: &CliContext) -> PluginResult<CliResult> {
        let subcommand = ctx.subcommand.as_deref().unwrap_or("");
        let args: Vec<&str> = ctx.args.iter().map(|s| s.as_str()).collect();
        let options = ctx.options_as_json();

        let result = match subcommand {
            "run" => cmd_run(&args, &options),
            "config" => cmd_config(&args),
            "tools" => cmd_tools(&args),
            "" | "help" => Ok(get_help()),
            _ => Err(format!("Unknown command: {}", subcommand)),
        };

        match result {
            Ok(output) => Ok(CliResult::success(output)),
            Err(e) => Ok(CliResult::error(e)),
        }
    }
}

/// Create the plugin instance (v3 entry point)
#[no_mangle]
pub fn plugin_create() -> Box<dyn Plugin> {
    Box::new(AgentLoopPlugin::new())
}

/// Create the CLI commands interface
#[no_mangle]
pub fn plugin_create_cli() -> Box<dyn CliCommands> {
    Box::new(AgentLoopPlugin::new())
}

// === Command Implementations ===

fn get_help() -> String {
    r#"ADI Agent Loop - Autonomous LLM agent with tool execution

Commands:
  run      Run agent with a task
  config   Manage configuration
  tools    List available tools

Usage: adi agent-loop <command> [args]"#
        .to_string()
}

fn cmd_run(args: &[&str], options: &serde_json::Value) -> Result<String, String> {
    if args.is_empty() {
        return Err("Missing task. Usage: run <task> [--max-iterations <n>] [--yes] [--file <path>] [--system-prompt <prompt>]".to_string());
    }

    let task = args[0];
    let max_iterations = options
        .get("max-iterations")
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse().ok())
        .unwrap_or(50usize);
    let auto_approve = options
        .get("yes")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let file_path = options.get("file").and_then(|v| v.as_str());
    let system_prompt = options.get("system-prompt").and_then(|v| v.as_str());

    // Read task from file if specified, otherwise use provided task
    let task_content = if let Some(path) = file_path {
        std::fs::read_to_string(path).map_err(|e| format!("Failed to read file {}: {}", path, e))?
    } else {
        task.to_string()
    };

    // Run the agent using tokio runtime
    let runtime = get_runtime();

    let result = runtime.block_on(async {
        let config = LoopConfig {
            max_iterations,
            ..Default::default()
        };

        let provider = Arc::new(MockLlmProvider::with_responses(vec![Message::assistant(
            "This is a demo response. Connect a real LLM provider for actual functionality.",
        )]));

        let mut agent = AgentLoop::new(provider).with_loop_config(config);

        if let Some(prompt) = system_prompt {
            agent = agent.with_system_prompt(prompt.to_string());
        }

        if auto_approve {
            agent.run(&task_content).await
        } else {
            let approver = InteractiveApprover::new();
            agent.run_with_approval(&approver, &task_content).await
        }
    });

    match result {
        Ok(response) => {
            let mut output = String::new();
            output.push_str(&format!("{}\n", response));
            Ok(output)
        }
        Err(e) => Err(format!("Agent error: {}", e)),
    }
}

struct InteractiveApprover;

impl InteractiveApprover {
    fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ApprovalHandler for InteractiveApprover {
    async fn request_approval(
        &self,
        tool_call: &ToolCall,
        rule: Option<&PermissionRule>,
    ) -> agent_loop_core::Result<ApprovalDecision> {
        eprintln!("\n{} Agent wants to run:", style("?").yellow().bold());

        eprintln!(
            "  {}: {}",
            style(&tool_call.name).cyan().bold(),
            serde_json::to_string_pretty(&tool_call.arguments).unwrap_or_default()
        );

        if let Some(r) = rule {
            if let Some(reason) = &r.reason {
                eprintln!("  {} {}", style("Note:").yellow(), reason);
            }
        }

        let options = vec!["Allow", "Allow All (this session)", "Deny", "Abort"];
        let selection = Select::with_theme(&ColorfulTheme::default())
            .items(&options)
            .default(0)
            .interact()
            .unwrap_or(2);

        Ok(match selection {
            0 => ApprovalDecision::Allow,
            1 => ApprovalDecision::AllowAll,
            2 => ApprovalDecision::Deny,
            _ => ApprovalDecision::Abort,
        })
    }
}

fn cmd_config(args: &[&str]) -> Result<String, String> {
    let subcommand = args.first().copied().unwrap_or("show");

    match subcommand {
        "show" => {
            let mut output = String::from("Current configuration:\n\n");
            output.push_str("  model: claude-sonnet-4-20250514\n");
            output.push_str("  max_iterations: 50\n");
            output.push_str("  max_tokens: 100000\n");
            output.push_str("  timeout_ms: 120000\n");
            Ok(output.trim_end().to_string())
        }
        "set" => {
            if args.len() < 3 {
                return Err("Usage: config set <key> <value>".to_string());
            }
            let key = args[1];
            let value = args[2];
            Ok(format!("Set {} = {}", key, value))
        }
        _ => Err(format!(
            "Unknown config subcommand: {}. Use 'show' or 'set'",
            subcommand
        )),
    }
}

fn cmd_tools(args: &[&str]) -> Result<String, String> {
    let subcommand = args.first().copied().unwrap_or("list");

    match subcommand {
        "list" => {
            let mut output = String::from("Available tools:\n\n");
            output.push_str("  (No tools registered - add tools via configuration)\n\n");
            output.push_str("To add tools, edit ~/.config/adi/agent.toml:\n\n");
            output.push_str("  [[tools]]\n");
            output.push_str("  name = \"my_tool\"\n");
            output.push_str("  command = \"my-command\"\n");
            Ok(output.trim_end().to_string())
        }
        _ => Err(format!(
            "Unknown tools subcommand: {}. Use 'list'",
            subcommand
        )),
    }
}
