//! ADI Tools Plugin (v3 ABI)
//!
//! Provides CLI commands for tool discovery and search.
//! LLM agents get a single meta-command to search tools by intent
//! and pull full usage docs only when needed.

use adi_tools_core::{discover_all, discover_tool_from_path, fetch_help, Config, ToolSearch};
use lib_plugin_abi_v3::{
    async_trait,
    cli::{CliCommand, CliCommands, CliContext, CliResult},
    Plugin, PluginContext, PluginMetadata, PluginType, Result as PluginResult,
    SERVICE_CLI_COMMANDS,
};
use std::sync::{Arc, Mutex};

type CmdResult = std::result::Result<String, String>;

pub struct ToolsPlugin {
    search: Arc<Mutex<Option<ToolSearch>>>,
    config: Config,
}

impl ToolsPlugin {
    pub fn new() -> Self {
        Self {
            search: Arc::new(Mutex::new(None)),
            config: Config::default(),
        }
    }
}

impl Default for ToolsPlugin {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Plugin for ToolsPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            id: "adi.tools".to_string(),
            name: "Tool Index".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            plugin_type: PluginType::Core,
            author: Some("ADI Team".to_string()),
            description: Some("Searchable index of CLI tools for LLM agents".to_string()),
            category: None,
        }
    }

    async fn init(&mut self, _ctx: &PluginContext) -> PluginResult<()> {
        // Try to open existing index
        let search = ToolSearch::open(&self.config).ok();
        *self.search.lock().unwrap() = search;
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
impl CliCommands for ToolsPlugin {
    async fn list_commands(&self) -> Vec<CliCommand> {
        vec![
            CliCommand {
                name: "find".to_string(),
                description: "Search tools by intent".to_string(),
                usage: "find <query> [--limit <n>]".to_string(),
                has_subcommands: false,
            },
            CliCommand {
                name: "help".to_string(),
                description: "Show full usage for a tool".to_string(),
                usage: "help <tool-id>".to_string(),
                has_subcommands: false,
            },
            CliCommand {
                name: "list".to_string(),
                description: "List all indexed tools".to_string(),
                usage: "list [--source <plugin|tooldir|system>] [--format <text|json>]".to_string(),
                has_subcommands: false,
            },
            CliCommand {
                name: "run".to_string(),
                description: "Run a tool".to_string(),
                usage: "run <tool-id> [args...]".to_string(),
                has_subcommands: false,
            },
            CliCommand {
                name: "index".to_string(),
                description: "Re-index all tools".to_string(),
                usage: "index".to_string(),
                has_subcommands: false,
            },
            CliCommand {
                name: "add".to_string(),
                description: "Add a tool to index".to_string(),
                usage: "add <path>".to_string(),
                has_subcommands: false,
            },
            CliCommand {
                name: "remove".to_string(),
                description: "Remove a tool from index".to_string(),
                usage: "remove <tool-id>".to_string(),
                has_subcommands: false,
            },
            CliCommand {
                name: "stats".to_string(),
                description: "Show index statistics".to_string(),
                usage: "stats".to_string(),
                has_subcommands: false,
            },
        ]
    }

    async fn run_command(&self, ctx: &CliContext) -> PluginResult<CliResult> {
        let subcommand = ctx.subcommand.as_deref().unwrap_or("");

        let result = match subcommand {
            "find" => {
                let guard = self.search.lock().unwrap();
                if let Some(ref search) = *guard {
                    cmd_find(search, ctx)
                } else {
                    Err("Tool index not initialized. Run: adi tools index".to_string())
                }
            }
            "help" => {
                let guard = self.search.lock().unwrap();
                if let Some(ref search) = *guard {
                    cmd_help(search, ctx)
                } else {
                    Err("Tool index not initialized".to_string())
                }
            }
            "list" => {
                let guard = self.search.lock().unwrap();
                if let Some(ref search) = *guard {
                    cmd_list(search, ctx)
                } else {
                    Err("Tool index not initialized".to_string())
                }
            }
            "run" => {
                let guard = self.search.lock().unwrap();
                if let Some(ref search) = *guard {
                    cmd_run(search, ctx)
                } else {
                    drop(guard);
                    cmd_run_direct(ctx)
                }
            }
            "index" => cmd_index(&self.search, &self.config),
            "add" => cmd_add(&self.search, &self.config, ctx),
            "remove" => {
                let guard = self.search.lock().unwrap();
                if let Some(ref search) = *guard {
                    cmd_remove(search, ctx)
                } else {
                    Err("Tool index not initialized".to_string())
                }
            }
            "stats" => {
                let guard = self.search.lock().unwrap();
                if let Some(ref search) = *guard {
                    cmd_stats(search)
                } else {
                    Err("Tool index not initialized".to_string())
                }
            }
            "" => Ok(get_help()),
            _ => Err(format!("Unknown command: {}", subcommand)),
        };

        match result {
            Ok(output) => Ok(CliResult::success(output)),
            Err(e) => Ok(CliResult::error(e)),
        }
    }
}

#[no_mangle]
pub fn plugin_create() -> Box<dyn Plugin> {
    Box::new(ToolsPlugin::new())
}

#[no_mangle]
pub fn plugin_create_cli() -> Box<dyn CliCommands> {
    Box::new(ToolsPlugin::new())
}

fn get_help() -> String {
    r#"ADI Tools - Searchable CLI Tool Index

Commands:
  find    Search tools by intent (semantic + keyword)
  help    Show full usage for a tool
  list    List all indexed tools
  run     Run a tool
  index   Re-index all tools
  add     Add a tool to index
  remove  Remove a tool from index
  stats   Show index statistics

Usage: adi tools <command> [args]

Examples:
  adi tools find "list docker containers"
  adi tools help docker-ps
  adi tools list --source plugin
  adi tools run git-status
  adi tools index"#
        .to_string()
}

fn cmd_find(search: &ToolSearch, ctx: &CliContext) -> CmdResult {
    let query = ctx
        .arg(0)
        .ok_or_else(|| "Missing query. Usage: find <query> [--limit <n>]".to_string())?;

    let limit: usize = ctx.option("limit").unwrap_or(10);

    let results = search.find(query, limit).map_err(|e| e.to_string())?;

    if results.is_empty() {
        return Ok(format!("No tools found for: {}", query));
    }

    let mut output = String::new();
    for result in results {
        output.push_str(&format!(
            "{}: {}\n",
            result.tool.name, result.tool.description
        ));
    }
    output.push_str("---\n");
    output.push_str("Use: adi tools help <name> for full usage");

    Ok(output)
}

fn cmd_help(search: &ToolSearch, ctx: &CliContext) -> CmdResult {
    let tool_id = ctx
        .arg(0)
        .ok_or_else(|| "Missing tool ID. Usage: help <tool-id>".to_string())?;

    let tool = search
        .get(tool_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Tool not found: {}", tool_id))?;

    // Fetch fresh --help
    let usage = fetch_help(&tool).map_err(|e| e.to_string())?;

    Ok(usage.help_text)
}

fn cmd_list(search: &ToolSearch, ctx: &CliContext) -> CmdResult {
    let source_filter: Option<String> = ctx.option("source");
    let format: Option<String> = ctx.option("format");
    let format = format.as_deref().unwrap_or("text");

    let tools = search.list().map_err(|e| e.to_string())?;

    let filtered: Vec<_> = if let Some(ref source) = source_filter {
        tools
            .into_iter()
            .filter(|t| {
                matches!(
                    (&t.source, source.as_str()),
                    (adi_tools_core::ToolSource::Plugin { .. }, "plugin")
                        | (adi_tools_core::ToolSource::ToolDir { .. }, "tooldir")
                        | (adi_tools_core::ToolSource::System { .. }, "system")
                )
            })
            .collect()
    } else {
        tools
    };

    if format == "json" {
        return serde_json::to_string_pretty(&filtered).map_err(|e| e.to_string());
    }

    if filtered.is_empty() {
        return Ok("No tools indexed. Run: adi tools index".to_string());
    }

    let mut output = String::new();
    for tool in filtered {
        let source = match &tool.source {
            adi_tools_core::ToolSource::Plugin { .. } => "[plugin]",
            adi_tools_core::ToolSource::ToolDir { .. } => "[tool]",
            adi_tools_core::ToolSource::System { .. } => "[system]",
        };
        output.push_str(&format!(
            "{} {} - {}\n",
            source, tool.name, tool.description
        ));
    }

    Ok(output.trim_end().to_string())
}

fn cmd_run(search: &ToolSearch, ctx: &CliContext) -> CmdResult {
    let tool_id = ctx
        .arg(0)
        .ok_or_else(|| "Missing tool ID. Usage: run <tool-id> [args...]".to_string())?;

    let tool = search
        .get(tool_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Tool not found: {}", tool_id))?;

    // Get remaining args
    let args: Vec<String> = (1..).map_while(|i| ctx.arg(i).map(|s| s.to_string())).collect();

    match &tool.source {
        adi_tools_core::ToolSource::Plugin { command, .. } => {
            // Run: adi <command> [args...]
            let mut cmd_args = vec![command.clone()];
            cmd_args.extend(args);

            let output = std::process::Command::new("adi")
                .args(&cmd_args)
                .output()
                .map_err(|e| format!("Failed to run adi {}: {}", command, e))?;

            if output.status.success() {
                Ok(String::from_utf8_lossy(&output.stdout).to_string())
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                let stdout = String::from_utf8_lossy(&output.stdout);
                Err(format!("{}{}", stdout, stderr))
            }
        }
        adi_tools_core::ToolSource::ToolDir { path, .. }
        | adi_tools_core::ToolSource::System { path } => {
            let output = std::process::Command::new(path)
                .args(&args)
                .output()
                .map_err(|e| format!("Failed to run {}: {}", path.display(), e))?;

            if output.status.success() {
                Ok(String::from_utf8_lossy(&output.stdout).to_string())
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                let stdout = String::from_utf8_lossy(&output.stdout);
                Err(format!("{}{}", stdout, stderr))
            }
        }
    }
}

fn cmd_run_direct(ctx: &CliContext) -> CmdResult {
    let tool_id = ctx
        .arg(0)
        .ok_or_else(|| "Missing tool ID. Usage: run <tool-id> [args...]".to_string())?;

    let args: Vec<String> = (1..).map_while(|i| ctx.arg(i).map(|s| s.to_string())).collect();

    let output = std::process::Command::new(tool_id)
        .args(&args)
        .output()
        .map_err(|e| format!("Failed to run {}: {}", tool_id, e))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

fn cmd_index(
    search_lock: &Arc<Mutex<Option<ToolSearch>>>,
    config: &Config,
) -> CmdResult {
    // Discover all tools
    let tools = discover_all(config).map_err(|e| e.to_string())?;

    // Open or create search index
    let search = ToolSearch::open(config).map_err(|e| e.to_string())?;

    // Clear existing and index all tools
    search.storage().clear().map_err(|e| e.to_string())?;
    
    let count = tools.len();
    for tool in tools {
        search
            .storage()
            .upsert_tool(&tool)
            .map_err(|e| e.to_string())?;
    }

    // Update shared state
    *search_lock.lock().unwrap() = Some(search);

    Ok(format!("Indexed {} tools", count))
}

fn cmd_add(
    search_lock: &Arc<Mutex<Option<ToolSearch>>>,
    config: &Config,
    ctx: &CliContext,
) -> CmdResult {
    let path_str = ctx
        .arg(0)
        .ok_or_else(|| "Missing path. Usage: add <path>".to_string())?;

    let path = std::path::Path::new(path_str);
    if !path.exists() {
        return Err(format!("Path not found: {}", path.display()));
    }

    let tool = discover_tool_from_path(path).map_err(|e| e.to_string())?;

    let mut guard = search_lock.lock().unwrap();
    
    // Initialize if needed
    if guard.is_none() {
        let search = ToolSearch::open(config).map_err(|e| e.to_string())?;
        *guard = Some(search);
    }
    
    if let Some(ref search) = *guard {
        search
            .storage()
            .upsert_tool(&tool)
            .map_err(|e| e.to_string())?;
        Ok(format!("Added tool: {} - {}", tool.name, tool.description))
    } else {
        Err("Failed to initialize tool index".to_string())
    }
}

fn cmd_remove(search: &ToolSearch, ctx: &CliContext) -> CmdResult {
    let tool_id = ctx
        .arg(0)
        .ok_or_else(|| "Missing tool ID. Usage: remove <tool-id>".to_string())?;

    // Check if tool exists
    let tool = search
        .get(tool_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Tool not found: {}", tool_id))?;

    search
        .storage()
        .delete_tool(tool_id)
        .map_err(|e| e.to_string())?;

    Ok(format!("Removed tool: {}", tool.name))
}

fn cmd_stats(search: &ToolSearch) -> CmdResult {
    let tools = search.list().map_err(|e| e.to_string())?;

    let mut plugin_count = 0;
    let mut tooldir_count = 0;
    let mut system_count = 0;

    for tool in &tools {
        match &tool.source {
            adi_tools_core::ToolSource::Plugin { .. } => plugin_count += 1,
            adi_tools_core::ToolSource::ToolDir { .. } => tooldir_count += 1,
            adi_tools_core::ToolSource::System { .. } => system_count += 1,
        }
    }

    let mut output = String::from("Tool Index Statistics\n\n");
    output.push_str(&format!("  Total tools:     {}\n", tools.len()));
    output.push_str(&format!("  From plugins:    {}\n", plugin_count));
    output.push_str(&format!("  From tools dir:  {}\n", tooldir_count));
    output.push_str(&format!("  From system:     {}\n", system_count));

    Ok(output.trim_end().to_string())
}
