//! ADI LLM Extract Plugin
//!
//! Extracts LLM-friendly documentation from ADI plugins.
//! Outputs markdown suitable for claude.md or similar AI context files.

use lib_plugin_abi_v3::{
    async_trait,
    cli::{CliCommand, CliCommands, CliContext, CliResult},
    Plugin, PluginContext, PluginMetadata, PluginType, Result as PluginResult, SERVICE_CLI_COMMANDS,
};
use lib_plugin_manifest::PluginManifest;
use serde_json::json;
use std::path::PathBuf;

use lib_env_parse::{env_vars, env_opt};

env_vars! {
    Home => "HOME",
}

/// LLM Extract Plugin
pub struct LlmExtractPlugin;

impl LlmExtractPlugin {
    pub fn new() -> Self {
        Self
    }
}

impl Default for LlmExtractPlugin {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Plugin for LlmExtractPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            id: "adi.llm-extract".to_string(),
            name: "ADI LLM Extract".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            plugin_type: PluginType::Core,
            author: Some("ADI Team".to_string()),
            description: Some("Extract LLM-friendly documentation from plugins".to_string()),
            category: None,
        }
    }

    async fn init(&mut self, _ctx: &PluginContext) -> PluginResult<()> {
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
impl CliCommands for LlmExtractPlugin {
    async fn list_commands(&self) -> Vec<CliCommand> {
        vec![
            CliCommand {
                name: "extract".to_string(),
                description: "Extract LLM documentation from a plugin".to_string(),
                usage: "extract <plugin-id> [--format <json|md>]".to_string(),
                has_subcommands: false,
            },
            CliCommand {
                name: "all".to_string(),
                description: "Extract docs from all installed plugins".to_string(),
                usage: "all [--format <json|md>]".to_string(),
                has_subcommands: false,
            },
        ]
    }

    async fn run_command(&self, ctx: &CliContext) -> PluginResult<CliResult> {
        let subcommand = ctx.subcommand.as_deref().unwrap_or("");
        let args: Vec<&str> = ctx.args.iter().map(|s| s.as_str()).collect();
        let options = ctx.options_as_json();

        let result = match subcommand {
            "extract" => cmd_extract(&args, &options),
            "all" => cmd_all(&options),
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
    Box::new(LlmExtractPlugin::new())
}

/// Create the CLI commands interface
#[no_mangle]
pub fn plugin_create_cli() -> Box<dyn CliCommands> {
    Box::new(LlmExtractPlugin::new())
}

// === Command Implementations ===

fn get_help() -> String {
    r#"ADI LLM Extract - Extract plugin documentation for LLM consumption

Commands:
  extract <plugin-id>   Extract LLM docs from a plugin
  all                   Extract docs from all installed plugins

Options:
  --format <json|md>    Output format (default: md)

Examples:
  adi llm-extract extract adi.tasks
  adi llm-extract all --format json"#
        .to_string()
}

fn cmd_extract(positional: &[&str], options: &serde_json::Value) -> Result<String, String> {
    let plugin_id = positional
        .first()
        .ok_or("Missing plugin ID. Usage: extract <plugin-id>")?;

    let format = options
        .get("format")
        .and_then(|v| v.as_str())
        .unwrap_or("md");

    let manifest = find_plugin_manifest(plugin_id)?;
    let llm_info = extract_llm_info(&manifest, plugin_id)?;

    match format {
        "json" => serde_json::to_string_pretty(&llm_info).map_err(|e| e.to_string()),
        _ => Ok(format_as_markdown(&llm_info)),
    }
}

fn cmd_all(options: &serde_json::Value) -> Result<String, String> {
    let format = options
        .get("format")
        .and_then(|v| v.as_str())
        .unwrap_or("md");

    let plugins_dir = get_plugins_dir()?;
    let mut all_info = Vec::new();

    if let Ok(entries) = std::fs::read_dir(&plugins_dir) {
        for entry in entries.flatten() {
            let manifest_path = entry.path().join("plugin.toml");
            if manifest_path.exists() {
                if let Ok(manifest) = PluginManifest::from_file(&manifest_path) {
                    if let Ok(info) = extract_llm_info(&manifest, &manifest.plugin.id) {
                        all_info.push(info);
                    }
                }
            }
        }
    }

    if all_info.is_empty() {
        return Ok("[!] No plugins found".to_string());
    }

    match format {
        "json" => serde_json::to_string_pretty(&all_info).map_err(|e| e.to_string()),
        _ => {
            let mut output = String::new();
            for info in &all_info {
                output.push_str(&format_as_markdown(info));
                output.push_str("\n---\n\n");
            }
            Ok(output.trim_end_matches("\n---\n\n").to_string())
        }
    }
}

fn get_plugins_dir() -> Result<PathBuf, String> {
    let home = env_opt(EnvVar::Home.as_str()).ok_or("HOME not set")?;
    Ok(PathBuf::from(home).join(".adi").join("plugins"))
}

fn find_plugin_manifest(plugin_id: &str) -> Result<PluginManifest, String> {
    let plugins_dir = get_plugins_dir()?;
    let plugin_dir = plugins_dir.join(plugin_id);
    let manifest_path = plugin_dir.join("plugin.toml");

    if !manifest_path.exists() {
        return Err(format!("Plugin not found: {}", plugin_id));
    }

    PluginManifest::from_file(&manifest_path)
        .map_err(|e| format!("Failed to parse manifest: {}", e))
}

fn extract_llm_info(
    manifest: &PluginManifest,
    plugin_id: &str,
) -> Result<serde_json::Value, String> {
    let mut info = json!({
        "plugin": {
            "id": manifest.plugin.id,
            "name": manifest.plugin.name,
            "description": manifest.plugin.description
        }
    });

    // Add CLI info if present
    if let Some(cli) = &manifest.cli {
        info["cli"] = json!({
            "command": cli.command,
            "aliases": cli.aliases,
            "usage": format!("adi {} <command> [options]", cli.command)
        });
    }

    // Add services
    if !manifest.provides.is_empty() {
        info["services"] = json!(manifest
            .provides
            .iter()
            .map(|s| json!({
                "id": s.id,
                "version": s.version,
                "description": s.description
            }))
            .collect::<Vec<_>>());
    }

    // Placeholder for commands - full implementation would invoke plugin
    info["commands"] = json!([]);
    info["_note"] = json!(format!(
        "Run `adi run {} list_commands` for detailed command info",
        plugin_id
    ));

    Ok(info)
}

fn format_as_markdown(info: &serde_json::Value) -> String {
    let mut md = String::new();

    // Header
    let id = info["plugin"]["id"].as_str().unwrap_or("unknown");
    let name = info["plugin"]["name"].as_str().unwrap_or("Unknown");
    let desc = info["plugin"]["description"].as_str().unwrap_or("");

    md.push_str(&format!("## {} ({})\n", id, name));

    // Categories as tags
    if let Some(cats) = info["plugin"]["categories"].as_array() {
        let tags: Vec<&str> = cats.iter().filter_map(|c| c.as_str()).collect();
        if !tags.is_empty() {
            md.push_str(&format!("{}\n", tags.join(", ")));
        }
    }
    md.push('\n');

    // Description
    if !desc.is_empty() {
        md.push_str(&format!("{}\n\n", desc));
    }

    // CLI usage
    if let Some(cli) = info.get("cli") {
        let usage = cli["usage"].as_str().unwrap_or("");

        md.push_str("### Usage\n");
        md.push_str(&format!("```\n{}\n```\n\n", usage));

        if let Some(aliases) = cli["aliases"].as_array() {
            let alias_strs: Vec<&str> = aliases.iter().filter_map(|a| a.as_str()).collect();
            if !alias_strs.is_empty() {
                md.push_str(&format!("Aliases: `{}`\n\n", alias_strs.join("`, `")));
            }
        }
    }

    // Commands
    if let Some(commands) = info.get("commands").and_then(|c| c.as_array()) {
        if !commands.is_empty() {
            md.push_str("### Commands\n");
            for cmd in commands {
                let name = cmd["name"].as_str().unwrap_or("");
                let desc = cmd["description"].as_str().unwrap_or("");
                let usage = cmd["usage"].as_str().unwrap_or("");

                md.push_str(&format!("- `{}` - {}\n", name, desc));
                if !usage.is_empty() && usage != name {
                    md.push_str(&format!("  Usage: `{}`\n", usage));
                }
            }
            md.push('\n');
        }
    }

    // Services
    if let Some(services) = info.get("services").and_then(|s| s.as_array()) {
        if !services.is_empty() {
            md.push_str("### Services\n");
            for svc in services {
                let id = svc["id"].as_str().unwrap_or("");
                let desc = svc["description"].as_str().unwrap_or("");
                md.push_str(&format!("- `{}` - {}\n", id, desc));
            }
            md.push('\n');
        }
    }

    md
}
