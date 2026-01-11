//! ADI Agent Loop Plugin
//!
//! Provides CLI commands for autonomous LLM agents.

use abi_stable::std_types::{ROption, RResult, RStr, RString, RVec};
use adi_agent_loop_core::{
    AgentLoop, ApprovalDecision, ApprovalHandler, LoopConfig, Message, MockLlmProvider,
    PermissionRule, ToolCall,
};
use async_trait::async_trait;
use lib_plugin_abi::{
    PluginContext, PluginInfo, PluginVTable, ServiceDescriptor, ServiceError, ServiceHandle,
    ServiceMethod, ServiceVTable, ServiceVersion,
};

/// Plugin-specific CLI service ID
const SERVICE_CLI: &str = "adi.agent-loop.cli";
use console::style;
use dialoguer::{theme::ColorfulTheme, Select};
use serde_json::json;
use std::ffi::c_void;
use std::sync::Arc;

// === Plugin VTable Implementation ===

extern "C" fn plugin_info() -> PluginInfo {
    PluginInfo::new(
        "adi.agent-loop",
        "ADI Agent Loop",
        env!("CARGO_PKG_VERSION"),
        "core",
    )
    .with_author("ADI Team")
    .with_description("Autonomous LLM agent with tool execution")
    .with_min_host_version("0.8.0")
}

extern "C" fn plugin_init(ctx: *mut PluginContext) -> i32 {
    unsafe {
        let host = (*ctx).host();

        // Register CLI commands service
        let cli_descriptor =
            ServiceDescriptor::new(SERVICE_CLI, ServiceVersion::new(1, 0, 0), "adi.agent-loop")
                .with_description("CLI commands for agent operations");

        let cli_handle = ServiceHandle::new(
            SERVICE_CLI,
            ctx as *const c_void,
            &CLI_SERVICE_VTABLE as *const ServiceVTable,
        );

        if let Err(code) = host.register_svc(cli_descriptor, cli_handle) {
            host.error(&format!(
                "Failed to register CLI commands service: {}",
                code
            ));
            return code;
        }

        host.info("ADI Agent Loop plugin initialized");
    }

    0
}

extern "C" fn plugin_cleanup(_ctx: *mut PluginContext) {}

// === Plugin Entry Point ===

static PLUGIN_VTABLE: PluginVTable = PluginVTable {
    info: plugin_info,
    init: plugin_init,
    update: ROption::RNone,
    cleanup: plugin_cleanup,
    handle_message: ROption::RNone,
};

#[no_mangle]
pub extern "C" fn plugin_entry() -> *const PluginVTable {
    &PLUGIN_VTABLE
}

// === CLI Service VTable ===

static CLI_SERVICE_VTABLE: ServiceVTable = ServiceVTable {
    invoke: cli_invoke,
    list_methods: cli_list_methods,
};

extern "C" fn cli_invoke(
    _handle: *const c_void,
    method: RStr<'_>,
    args: RStr<'_>,
) -> RResult<RString, ServiceError> {
    match method.as_str() {
        "run_command" => {
            let result = run_cli_command(args.as_str());
            match result {
                Ok(output) => RResult::ROk(RString::from(output)),
                Err(e) => RResult::RErr(ServiceError::invocation_error(e)),
            }
        }
        "list_commands" => {
            let commands = json!([
                {"name": "run", "description": "Run agent with a task", "usage": "run <task> [--max-iterations <n>] [--yes] [--file <path>] [--system-prompt <prompt>]"},
                {"name": "config", "description": "Manage configuration", "usage": "config [show|set <key> <value>]"},
                {"name": "tools", "description": "List available tools", "usage": "tools [list]"}
            ]);
            RResult::ROk(RString::from(
                serde_json::to_string(&commands).unwrap_or_default(),
            ))
        }
        _ => RResult::RErr(ServiceError::method_not_found(method.as_str())),
    }
}

extern "C" fn cli_list_methods(_handle: *const c_void) -> RVec<ServiceMethod> {
    vec![
        ServiceMethod::new("run_command").with_description("Run a CLI command"),
        ServiceMethod::new("list_commands").with_description("List available commands"),
    ]
    .into_iter()
    .collect()
}

fn run_cli_command(context_json: &str) -> Result<String, String> {
    let context: serde_json::Value =
        serde_json::from_str(context_json).map_err(|e| format!("Invalid context: {}", e))?;

    // Parse command and args from context
    let args: Vec<String> = context
        .get("args")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    let subcommand = args.first().map(|s| s.as_str()).unwrap_or("");
    let cmd_args: Vec<&str> = args.iter().skip(1).map(|s| s.as_str()).collect();

    // Parse options from remaining args (--key value format)
    let mut options = serde_json::Map::new();
    let mut i = 0;
    while i < cmd_args.len() {
        if cmd_args[i].starts_with("--") {
            let key = cmd_args[i].trim_start_matches("--");
            if i + 1 < cmd_args.len() && !cmd_args[i + 1].starts_with("--") {
                options.insert(key.to_string(), json!(cmd_args[i + 1]));
                i += 2;
            } else {
                options.insert(key.to_string(), json!(true));
                i += 1;
            }
        } else {
            i += 1;
        }
    }

    // Get positional args (non-option args after subcommand)
    let positional: Vec<&str> = cmd_args
        .iter()
        .filter(|a| !a.starts_with("--"))
        .copied()
        .collect();

    let options_value = serde_json::Value::Object(options);

    match subcommand {
        "run" => cmd_run(&positional, &options_value),
        "config" => cmd_config(&positional),
        "tools" => cmd_tools(&positional),
        "" => {
            let help = "ADI Agent Loop - Autonomous LLM agent with tool execution\n\n\
                        Commands:\n  \
                        run      Run agent with a task\n  \
                        config   Manage configuration\n  \
                        tools    List available tools\n\n\
                        Usage: adi run adi.agent-loop <command> [args]";
            Ok(help.to_string())
        }
        _ => Err(format!("Unknown command: {}", subcommand)),
    }
}

// === Command Implementations ===

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
    let rt =
        tokio::runtime::Runtime::new().map_err(|e| format!("Failed to create runtime: {}", e))?;

    let result = rt.block_on(async {
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
    ) -> adi_agent_loop_core::Result<ApprovalDecision> {
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
