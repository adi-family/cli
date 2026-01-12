//! ADI Linter Plugin
//!
//! Provides CLI commands for code linting with configurable rules.

use abi_stable::std_types::{ROption, RResult, RStr, RString, RVec};
use lib_plugin_abi::{
    PluginContext, PluginError, PluginInfo, PluginVTable, ServiceDescriptor, ServiceError,
    ServiceHandle, ServiceMethod, ServiceVTable, ServiceVersion,
};

use adi_linter_core::{
    config::LinterConfig,
    output::{format_to_string, OutputFormat},
    runner::{Runner, RunnerConfig},
    AutofixConfig, AutofixEngine,
};
use once_cell::sync::OnceCell;
use serde_json::json;
use std::ffi::c_void;
use std::path::PathBuf;

/// Plugin-specific CLI service ID
const SERVICE_CLI: &str = "adi.linter.cli";

/// Tokio runtime for async operations
static RUNTIME: OnceCell<tokio::runtime::Runtime> = OnceCell::new();

fn get_runtime() -> &'static tokio::runtime::Runtime {
    RUNTIME.get_or_init(|| tokio::runtime::Runtime::new().expect("Failed to create tokio runtime"))
}

// === Plugin VTable Implementation ===

extern "C" fn plugin_info() -> PluginInfo {
    PluginInfo::new(
        "adi.linter",
        "ADI Linter",
        env!("CARGO_PKG_VERSION"),
        "core",
    )
    .with_author("ADI Team")
    .with_description("Language-agnostic code linting with configurable rules")
    .with_min_host_version("0.8.0")
}

extern "C" fn plugin_init(ctx: *mut PluginContext) -> i32 {
    unsafe {
        let host = (*ctx).host();

        // Register CLI commands service
        let cli_descriptor =
            ServiceDescriptor::new(SERVICE_CLI, ServiceVersion::new(1, 0, 0), "adi.linter")
                .with_description("CLI commands for code linting");

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

        host.info("ADI Linter plugin initialized");
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
            let commands = get_commands_info();
            RResult::ROk(RString::from(
                serde_json::to_string(&commands).unwrap_or_default(),
            ))
        }
        "llm_extract" => {
            let info = get_llm_extract_info();
            RResult::ROk(RString::from(
                serde_json::to_string_pretty(&info).unwrap_or_default(),
            ))
        }
        _ => RResult::RErr(ServiceError::method_not_found(method.as_str())),
    }
}

fn get_commands_info() -> serde_json::Value {
    json!([
        {
            "name": "run",
            "description": "Run linting on files in the current project",
            "usage": "run [files...] [--format <pretty|json|sarif>] [--fail-on <error|warning|info|hint>]",
            "examples": [
                "adi lint run",
                "adi lint run src/main.rs",
                "adi lint run --format json",
                "adi lint run --fail-on warning"
            ],
            "options": [
                {"name": "--format", "type": "string", "choices": ["pretty", "json", "sarif"], "default": "pretty", "description": "Output format"},
                {"name": "--fail-on", "type": "string", "choices": ["error", "warning", "info", "hint"], "default": "error", "description": "Minimum severity to fail"},
                {"name": "--sequential", "type": "boolean", "default": false, "description": "Disable parallel execution"}
            ]
        },
        {
            "name": "fix",
            "description": "Apply auto-fixes for linting issues",
            "usage": "fix [files...] [--dry-run] [--interactive] [--max-iterations <n>]",
            "examples": [
                "adi lint fix",
                "adi lint fix --dry-run",
                "adi lint fix src/ --max-iterations 5"
            ],
            "options": [
                {"name": "--dry-run", "type": "boolean", "default": false, "description": "Show fixes without applying"},
                {"name": "--interactive", "type": "boolean", "default": false, "description": "Confirm each fix"},
                {"name": "--max-iterations", "type": "number", "default": 10, "description": "Max fix iterations"}
            ]
        },
        {
            "name": "list",
            "description": "List all configured linters and their patterns",
            "usage": "list [--format <text|json>]",
            "examples": [
                "adi lint list",
                "adi lint list --format json"
            ],
            "options": [
                {"name": "--format", "type": "string", "choices": ["text", "json"], "default": "text"}
            ]
        },
        {
            "name": "config",
            "description": "Show current linter configuration file",
            "usage": "config",
            "examples": ["adi lint config"]
        },
        {
            "name": "init",
            "description": "Create default linter configuration in .adi/linters/",
            "usage": "init [--force]",
            "examples": [
                "adi lint init",
                "adi lint init --force"
            ],
            "options": [
                {"name": "--force", "type": "boolean", "default": false, "description": "Overwrite existing config"}
            ]
        }
    ])
}

fn get_llm_extract_info() -> serde_json::Value {
    json!({
        "plugin": {
            "id": "adi.linter",
            "name": "ADI Linter",
            "description": "Language-agnostic code linting with configurable rules, auto-fix support, and multiple output formats",
            "categories": ["linting", "code-quality", "static-analysis"],
            "summary": "ADI Linter provides configurable code linting for any language. Define rules in .adi/linters/ using individual rule files with regex patterns, external tools (shellcheck, eslint), or custom commands. Supports auto-fixing, parallel execution, and SARIF output for CI integration.",
            "use_cases": [
                "Enforce code style and patterns across a project",
                "Run multiple linters with unified output",
                "Auto-fix common issues",
                "Generate SARIF reports for CI/CD pipelines",
                "Custom regex-based rules without external tools"
            ]
        },
        "cli": {
            "command": "lint",
            "aliases": ["l"],
            "usage": "adi lint <command> [options]"
        },
        "commands": get_commands_info(),
        "services": [
            {
                "id": "adi.linter.cli",
                "version": "1.0.0",
                "description": "CLI commands for code linting"
            }
        ],
        "config": {
            "directory": ".adi/linters/",
            "config_file": "config.toml",
            "description": "Configuration directory for linter rules. Global settings in config.toml, individual rules in separate .toml files.",
            "sections": {
                "[linter]": {
                    "description": "Global linter settings",
                    "options": {
                        "parallel": {"type": "bool", "default": true, "description": "Run linters in parallel"},
                        "fail_fast": {"type": "bool", "default": false, "description": "Stop on first error"},
                        "timeout": {"type": "u64", "default": 30, "description": "Timeout per linter in seconds"},
                        "max_workers": {"type": "usize", "default": "auto", "description": "Maximum parallel workers"}
                    }
                },
                "[autofix]": {
                    "description": "Auto-fix settings",
                    "options": {
                        "enabled": {"type": "bool", "default": true, "description": "Enable auto-fix"},
                        "max_iterations": {"type": "usize", "default": 10, "description": "Max fix iterations"},
                        "interactive": {"type": "bool", "default": false, "description": "Prompt before each fix"}
                    }
                },
                "[categories]": {
                    "description": "Per-category configuration",
                    "format": "category_name = { enabled = true, fail_on = \"warning\", priority = 1000 }",
                    "built_in_categories": [
                        "security", "correctness", "error-handling", "architecture",
                        "performance", "code-quality", "best-practices", "testing",
                        "documentation", "naming", "style"
                    ]
                },
                "[[rules.exec]]": {
                    "description": "External linter rules (runs subprocess)",
                    "options": {
                        "id": {"type": "string", "required": true, "description": "Unique rule ID"},
                        "exec": {"type": "string", "required": true, "description": "Command template with {file}, {dir}, {basename}, {ext}"},
                        "category": {"type": "string", "description": "Single category"},
                        "categories": {"type": "array", "description": "Multiple categories"},
                        "glob": {"type": "string|array", "description": "File patterns to match"},
                        "output": {"type": "string", "choices": ["json", "exitcode", "lines"], "default": "json"},
                        "input": {"type": "string", "choices": ["filepath", "stdin", "both"], "default": "filepath"},
                        "severity": {"type": "string", "choices": ["hint", "info", "warning", "error"], "default": "warning"},
                        "timeout": {"type": "u64", "description": "Override global timeout"},
                        "fix.exec": {"type": "string", "description": "Fix command template"}
                    }
                },
                "[[rules.command]]": {
                    "description": "Built-in rules (no subprocess)",
                    "types": {
                        "regex-forbid": {"required": ["pattern", "message"], "description": "Error if regex matches"},
                        "regex-require": {"required": ["pattern", "message"], "description": "Error if regex NOT matches"},
                        "max-line-length": {"required": ["max"], "description": "Error if line exceeds max"},
                        "max-file-size": {"required": ["max"], "description": "Error if file exceeds max bytes"},
                        "contains": {"required": ["text", "message"], "description": "Error if text found"},
                        "not-contains": {"required": ["text", "message"], "description": "Error if text NOT found"}
                    },
                    "options": {
                        "id": {"type": "string", "required": true},
                        "type": {"type": "string", "required": true},
                        "pattern": {"type": "string", "description": "Regex pattern"},
                        "message": {"type": "string", "description": "Error message"},
                        "category": {"type": "string"},
                        "glob": {"type": "string|array"},
                        "severity": {"type": "string", "choices": ["hint", "info", "warning", "error"]},
                        "fix.pattern": {"type": "string", "description": "Pattern to replace"},
                        "fix.replacement": {"type": "string", "description": "Replacement text (supports $1, $2)"}
                    }
                }
            },
            "example": r#"[linter]
parallel = true
fail_fast = false
timeout = 60

[autofix]
enabled = true
max_iterations = 10

[categories]
security = { enabled = true, fail_on = "warning" }
style = { enabled = true, priority = 50 }

# External linter: shellcheck
[[rules.exec]]
id = "shellcheck"
exec = "shellcheck -f json {file}"
category = "correctness"
glob = "**/*.sh"
output = "json"

# Built-in rule: no TODOs
[[rules.command]]
id = "no-todo"
type = "regex-forbid"
pattern = "TODO|FIXME"
message = "Unresolved TODO/FIXME comment"
category = "code-quality"
glob = ["**/*.rs", "**/*.ts"]
severity = "warning"

# Built-in rule: line length
[[rules.command]]
id = "max-line"
type = "max-line-length"
max = 120
category = "style"
glob = "**/*"
severity = "info"

# Built-in rule with auto-fix
[[rules.command]]
id = "no-unwrap"
type = "regex-forbid"
pattern = "\\.unwrap\\(\\)"
message = "Avoid .unwrap(), use ? or proper error handling"
category = "error-handling"
glob = "**/*.rs"
severity = "warning"
fix = { pattern = "\\.unwrap\\(\\)", replacement = "?" }"#
        }
    })
}

extern "C" fn cli_list_methods(_handle: *const c_void) -> RVec<ServiceMethod> {
    vec![
        ServiceMethod::new("run_command").with_description("Run a CLI command"),
        ServiceMethod::new("list_commands").with_description("List available commands"),
        ServiceMethod::new("llm_extract").with_description("Get LLM-friendly plugin documentation"),
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

    let cwd = context
        .get("cwd")
        .and_then(|v| v.as_str())
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

    let subcommand = args.first().map(|s| s.as_str()).unwrap_or("");
    let cmd_args: Vec<&str> = args.iter().skip(1).map(|s| s.as_str()).collect();

    // Parse options from remaining args (--key value format)
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
        "run" => cmd_run(&cwd, &positional, &options_value),
        "fix" => cmd_fix(&cwd, &positional, &options_value),
        "list" => cmd_list(&cwd, &options_value),
        "config" => cmd_config(&cwd),
        "init" => cmd_init(&cwd, &options_value),
        "" | "help" | "--help" | "-h" => Ok(get_help()),
        _ => Err(format!(
            "Unknown command: {}. Use 'adi lint help' for usage.",
            subcommand
        )),
    }
}

fn get_help() -> String {
    let mut help = String::new();
    help.push_str("ADI Linter - Language-agnostic code linting\n\n");
    help.push_str("USAGE:\n");
    help.push_str("  adi lint <COMMAND> [OPTIONS]\n\n");
    help.push_str("COMMANDS:\n");
    help.push_str("  run     Run linting on files (default command)\n");
    help.push_str("  fix     Apply auto-fixes for linting issues\n");
    help.push_str("  list    List all configured linters\n");
    help.push_str("  config  Show current linter configuration\n");
    help.push_str("  init    Create default linter configuration\n");
    help.push_str("  help    Show this help message\n\n");
    help.push_str("OPTIONS:\n");
    help.push_str("  --format <pretty|json|sarif>  Output format (default: pretty)\n");
    help.push_str(
        "  --fail-on <level>             Minimum severity to fail (error|warning|info|hint)\n",
    );
    help.push_str("  --sequential                  Disable parallel execution\n\n");
    help.push_str("EXAMPLES:\n");
    help.push_str("  adi lint run                  # Lint all files in project\n");
    help.push_str("  adi lint run src/main.rs      # Lint specific file\n");
    help.push_str("  adi lint fix --dry-run        # Preview fixes without applying\n");
    help.push_str("  adi lint init                 # Create config in .adi/linters/\n");
    help
}

// === Command Implementations ===

fn cmd_run(
    project_path: &PathBuf,
    files: &[&str],
    options: &serde_json::Value,
) -> Result<String, String> {
    let format = options
        .get("format")
        .and_then(|v| v.as_str())
        .unwrap_or("pretty");
    let fail_on = options
        .get("fail-on")
        .and_then(|v| v.as_str())
        .unwrap_or("error");
    let parallel = !options
        .get("sequential")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let config = LinterConfig::load_from_project(project_path).map_err(|e| e.to_string())?;
    let registry = config.build_registry().map_err(|e| e.to_string())?;

    if registry.is_empty() {
        return Ok(
            "[!] No linters configured. Run `adi lint init` to create a config.".to_string(),
        );
    }

    let runner_config = RunnerConfig::new(project_path).parallel(parallel);
    let runner = Runner::new(registry, runner_config);

    let file_paths: Option<Vec<PathBuf>> = if files.is_empty() {
        None
    } else {
        Some(files.iter().map(PathBuf::from).collect())
    };

    let result = get_runtime()
        .block_on(runner.run(file_paths))
        .map_err(|e| e.to_string())?;

    let output_format = match format {
        "json" => OutputFormat::Json,
        "sarif" => OutputFormat::Sarif,
        _ => OutputFormat::Pretty,
    };

    let output = format_to_string(&result, output_format).map_err(|e| e.to_string())?;

    // Check for failures
    let fail_severity = match fail_on {
        "warning" => adi_linter_core::Severity::Warning,
        "info" => adi_linter_core::Severity::Info,
        "hint" => adi_linter_core::Severity::Hint,
        _ => adi_linter_core::Severity::Error,
    };

    let has_failures = result
        .diagnostics
        .iter()
        .any(|d| d.severity >= fail_severity);

    if has_failures {
        // Return output with exit marker for host to handle
        Ok(format!("__EXIT_CODE:1__\n{}", output))
    } else {
        Ok(output)
    }
}

fn cmd_fix(
    project_path: &PathBuf,
    files: &[&str],
    options: &serde_json::Value,
) -> Result<String, String> {
    let dry_run = options
        .get("dry-run")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let interactive = options
        .get("interactive")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let max_iterations = options
        .get("max-iterations")
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse().ok())
        .unwrap_or(10usize);

    let config = LinterConfig::load_from_project(project_path).map_err(|e| e.to_string())?;
    let registry = config.build_registry().map_err(|e| e.to_string())?;

    if registry.is_empty() {
        return Ok(
            "[!] No linters configured. Run `adi lint init` to create a config.".to_string(),
        );
    }

    let runner_config = RunnerConfig::new(project_path);
    let runner = Runner::new(registry, runner_config);

    let autofix_config = AutofixConfig {
        max_iterations,
        dry_run,
        interactive,
    };

    let engine = AutofixEngine::new(&runner, autofix_config);

    let file_paths: Option<Vec<PathBuf>> = if files.is_empty() {
        None
    } else {
        Some(files.iter().map(PathBuf::from).collect())
    };

    let result = get_runtime()
        .block_on(engine.run(file_paths))
        .map_err(|e| e.to_string())?;

    let mut output = String::new();

    if dry_run {
        output.push_str(&format!(
            "[i] Dry run: {} fixes would be applied\n",
            result.fixes_count()
        ));
    } else {
        output.push_str(&format!(
            "[+] Applied {} fixes in {} iterations\n",
            result.fixes_count(),
            result.iterations
        ));
    }

    if !result.remaining_diagnostics.is_empty() {
        let fixable_count = result
            .remaining_diagnostics
            .iter()
            .filter(|d| d.is_fixable())
            .count();
        output.push_str(&format!(
            "[!] {} issues remaining ({} fixable)\n",
            result.remaining_count(),
            fixable_count
        ));
    }

    if result.max_iterations_reached {
        output.push_str("[!] Max iterations reached. Run again to continue fixing.\n");
    }

    Ok(output.trim_end().to_string())
}

fn cmd_list(project_path: &PathBuf, options: &serde_json::Value) -> Result<String, String> {
    let format = options
        .get("format")
        .and_then(|v| v.as_str())
        .unwrap_or("text");

    let config = LinterConfig::load_from_project(project_path).map_err(|e| e.to_string())?;
    let registry = config.build_registry().map_err(|e| e.to_string())?;

    if format == "json" {
        let linters: Vec<_> = registry
            .all_linters()
            .map(|l| {
                json!({
                    "id": l.id(),
                    "categories": l.categories().iter().map(|c| c.display_name()).collect::<Vec<_>>(),
                    "priority": l.priority(),
                    "patterns": l.patterns(),
                })
            })
            .collect();
        return serde_json::to_string_pretty(&linters).map_err(|e| e.to_string());
    }

    if registry.is_empty() {
        return Ok("[!] No linters configured".to_string());
    }

    let mut output = String::from("Configured Linters\n\n");

    for linter in registry.by_priority() {
        output.push_str(&format!(
            "  {} {} (priority: {})\n",
            linter.primary_category().icon(),
            linter.id(),
            linter.priority()
        ));

        for pattern in linter.patterns() {
            output.push_str(&format!("      -> {}\n", pattern));
        }
    }

    Ok(output.trim_end().to_string())
}

fn cmd_config(project_path: &PathBuf) -> Result<String, String> {
    let linters_dir = project_path.join(".adi").join("linters");

    if !linters_dir.exists() {
        return Ok("[!] No config found. Run `adi lint init` to create one.".to_string());
    }

    let mut output = format!("Linters directory: {}\n\n", linters_dir.display());

    // Show config.toml
    let config_path = linters_dir.join("config.toml");
    if config_path.exists() {
        let content =
            std::fs::read_to_string(&config_path).map_err(|e| format!("Read error: {}", e))?;
        output.push_str("=== config.toml ===\n");
        output.push_str(&content);
        output.push_str("\n");
    }

    // List rule files
    let entries: Vec<_> = std::fs::read_dir(&linters_dir)
        .map_err(|e| format!("Read dir error: {}", e))?
        .filter_map(|e| e.ok())
        .filter(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            name.ends_with(".toml") && name != "config.toml" && !name.ends_with(".example")
        })
        .collect();

    if !entries.is_empty() {
        output.push_str("=== Active Rules ===\n");
        for entry in entries {
            output.push_str(&format!("  - {}\n", entry.file_name().to_string_lossy()));
        }
    }

    // List example files
    let examples: Vec<_> = std::fs::read_dir(&linters_dir)
        .map_err(|e| format!("Read dir error: {}", e))?
        .filter_map(|e| e.ok())
        .filter(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            name.ends_with(".toml.example")
        })
        .collect();

    if !examples.is_empty() {
        output.push_str("\n=== Example Rules (rename to enable) ===\n");
        for entry in examples {
            output.push_str(&format!("  - {}\n", entry.file_name().to_string_lossy()));
        }
    }

    Ok(output.trim_end().to_string())
}

fn cmd_init(project_path: &PathBuf, options: &serde_json::Value) -> Result<String, String> {
    let force = options
        .get("force")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let linters_dir = project_path.join(".adi").join("linters");
    let config_path = linters_dir.join("config.toml");

    if config_path.exists() && !force {
        return Ok(format!(
            "[!] Config already exists at {}\n  Use --force to overwrite",
            linters_dir.display()
        ));
    }

    std::fs::create_dir_all(&linters_dir).map_err(|e| format!("Create dir error: {}", e))?;

    // Write config.toml
    let config_content = r#"# ADI Linter Global Configuration
# Individual linter rules are defined in separate .toml files in this directory

[linter]
parallel = true
fail_fast = false
timeout = 30

[autofix]
enabled = true
max_iterations = 10

# Category configuration
[categories]
security = { enabled = true, fail_on = "warning" }
correctness = { enabled = true }
error-handling = { enabled = true }
architecture = { enabled = true }
performance = { enabled = true }
code-quality = { enabled = true }
best-practices = { enabled = true }
testing = { enabled = true }
documentation = { enabled = false }
naming = { enabled = true }
style = { enabled = true, priority = 50 }
"#;

    std::fs::write(&config_path, config_content).map_err(|e| format!("Write error: {}", e))?;

    // Write example rule files
    let no_todo_example = r#"# Example: Regex-based TODO finder
# Rename to no-todo.toml to enable

[rule]
id = "no-todo"
type = "command"
category = "code-quality"
severity = "warning"

[rule.command]
type = "regex-forbid"
pattern = "TODO|FIXME"
message = "Unresolved TODO comment"

[rule.glob]
patterns = ["**/*.rs", "**/*.ts", "**/*.js"]
"#;

    let shellcheck_example = r#"# Example: ShellCheck external linter
# Rename to shellcheck.toml to enable
# Requires: shellcheck installed (brew install shellcheck)

[rule]
id = "shellcheck"
type = "exec"
category = "correctness"
severity = "warning"

[rule.exec]
command = "shellcheck -f json {file}"
output = "json"
timeout = 30

[rule.glob]
patterns = ["**/*.sh", "**/*.bash"]
"#;

    let max_line_example = r#"# Example: Maximum line length checker
# Rename to max-line-length.toml to enable

[rule]
id = "max-line"
type = "command"
category = "style"
severity = "info"

[rule.command]
type = "max-line-length"
max = 120

[rule.glob]
patterns = ["**/*"]
"#;

    std::fs::write(linters_dir.join("no-todo.toml.example"), no_todo_example)
        .map_err(|e| format!("Write error: {}", e))?;
    std::fs::write(
        linters_dir.join("shellcheck.toml.example"),
        shellcheck_example,
    )
    .map_err(|e| format!("Write error: {}", e))?;
    std::fs::write(
        linters_dir.join("max-line-length.toml.example"),
        max_line_example,
    )
    .map_err(|e| format!("Write error: {}", e))?;

    Ok(format!(
        "[+] Created {}\n\n  Files created:\n    - config.toml (global settings)\n    - no-todo.toml.example\n    - shellcheck.toml.example\n    - max-line-length.toml.example\n\n  Rename .example files to .toml to enable rules.",
        linters_dir.display()
    ))
}
