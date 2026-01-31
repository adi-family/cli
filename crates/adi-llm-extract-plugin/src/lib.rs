//! ADI LLM Extract Plugin
//!
//! Extracts LLM-friendly documentation from ADI plugins.
//! Outputs markdown suitable for claude.md or similar AI context files.

use abi_stable::std_types::{ROption, RResult, RStr, RString, RVec};
use lib_plugin_abi::{
    PluginContext, PluginError, PluginInfo, PluginVTable, ServiceDescriptor, ServiceError,
    ServiceHandle, ServiceMethod, ServiceVTable, ServiceVersion,
};
use lib_plugin_manifest::PluginManifest;
use serde_json::json;
use std::ffi::c_void;
use std::path::PathBuf;

const SERVICE_CLI: &str = "adi.llm-extract.cli";

// === Plugin VTable Implementation ===

extern "C" fn plugin_info() -> PluginInfo {
    PluginInfo::new(
        "adi.llm-extract",
        "ADI LLM Extract",
        env!("CARGO_PKG_VERSION"),
        "core",
    )
    .with_author("ADI Team")
    .with_description("Extract LLM-friendly documentation from plugins")
    .with_min_host_version("0.8.0")
}

extern "C" fn plugin_init(ctx: *mut PluginContext) -> i32 {
    unsafe {
        let host = (*ctx).host();

        let cli_descriptor =
            ServiceDescriptor::new(SERVICE_CLI, ServiceVersion::new(1, 0, 0), "adi.llm-extract")
                .with_description("CLI commands for LLM documentation extraction");

        let cli_handle = ServiceHandle::new(
            SERVICE_CLI,
            ctx as *const c_void,
            &CLI_SERVICE_VTABLE as *const ServiceVTable,
        );

        if let Err(code) = host.register_svc(cli_descriptor, cli_handle) {
            host.error(&format!("Failed to register CLI service: {}", code));
            return code;
        }

        host.info("ADI LLM Extract plugin initialized");
    }
    0
}

extern "C" fn plugin_cleanup(_ctx: *mut PluginContext) {}

extern "C" fn handle_message(
    _ctx: *mut PluginContext,
    msg_type: RStr<'_>,
    _msg_data: RStr<'_>,
) -> RResult<RString, PluginError> {
    RResult::RErr(PluginError::new(
        -1,
        format!("Unknown message type: {}", msg_type.as_str()),
    ))
}

// === Plugin Entry Point ===

static PLUGIN_VTABLE: PluginVTable = PluginVTable {
    info: plugin_info,
    init: plugin_init,
    update: ROption::RNone,
    cleanup: plugin_cleanup,
    handle_message: ROption::RSome(handle_message),
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
                {
                    "name": "extract",
                    "description": "Extract LLM documentation from a plugin",
                    "usage": "extract <plugin-id> [--format <json|md>]",
                    "examples": [
                        "adi llm-extract extract adi.tasks",
                        "adi llm-extract extract adi.linter --format json"
                    ]
                },
                {
                    "name": "all",
                    "description": "Extract docs from all installed plugins",
                    "usage": "all [--format <json|md>]",
                    "examples": [
                        "adi llm-extract all",
                        "adi llm-extract all --format json"
                    ]
                }
            ]);
            RResult::ROk(RString::from(
                serde_json::to_string(&commands).unwrap_or_default(),
            ))
        }
        "llm_extract" => {
            let info = get_self_llm_info();
            RResult::ROk(RString::from(
                serde_json::to_string_pretty(&info).unwrap_or_default(),
            ))
        }
        _ => RResult::RErr(ServiceError::method_not_found(method.as_str())),
    }
}

extern "C" fn cli_list_methods(_handle: *const c_void) -> RVec<ServiceMethod> {
    vec![
        ServiceMethod::new("run_command").with_description("Run a CLI command"),
        ServiceMethod::new("list_commands").with_description("List available commands"),
        ServiceMethod::new("llm_extract").with_description("Get LLM-friendly documentation"),
    ]
    .into_iter()
    .collect()
}

fn get_self_llm_info() -> serde_json::Value {
    json!({
        "plugin": {
            "id": "adi.llm-extract",
            "name": "ADI LLM Extract",
            "description": "Extract LLM-friendly documentation from plugins for AI context files",
            "categories": ["documentation", "llm", "tooling"]
        },
        "cli": {
            "command": "llm-extract",
            "aliases": ["llm"],
            "usage": "adi llm-extract <command> [options]"
        },
        "commands": [
            {
                "name": "extract",
                "description": "Extract LLM documentation from a plugin",
                "usage": "extract <plugin-id> [--format <json|md>]",
                "examples": ["adi llm-extract extract adi.tasks"]
            },
            {
                "name": "all",
                "description": "Extract docs from all installed plugins",
                "usage": "all [--format <json|md>]",
                "examples": ["adi llm-extract all"]
            }
        ]
    })
}

fn run_cli_command(context_json: &str) -> Result<String, String> {
    let context: serde_json::Value =
        serde_json::from_str(context_json).map_err(|e| format!("Invalid context: {}", e))?;

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

    // Parse options
    let mut options = serde_json::Map::new();
    let mut positional = Vec::new();
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
            positional.push(cmd_args[i]);
            i += 1;
        }
    }

    let options_value = serde_json::Value::Object(options);

    match subcommand {
        "extract" => cmd_extract(&positional, &options_value),
        "all" => cmd_all(&options_value),
        "" => Ok(get_help()),
        _ => Err(format!("Unknown command: {}", subcommand)),
    }
}

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
    let home = std::env::var("HOME").map_err(|_| "HOME not set")?;
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
