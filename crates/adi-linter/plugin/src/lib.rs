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
    RUNTIME.get_or_init(|| {
        tokio::runtime::Runtime::new().expect("Failed to create tokio runtime")
    })
}

// === Plugin VTable Implementation ===

extern "C" fn plugin_info() -> PluginInfo {
    PluginInfo::new("adi.linter", "ADI Linter", env!("CARGO_PKG_VERSION"), "core")
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
            let commands = json!([
                {"name": "run", "description": "Run linting on files", "usage": "run [files...] [--format <pretty|json|sarif>] [--fail-on <error|warning|info|hint>]"},
                {"name": "fix", "description": "Apply auto-fixes", "usage": "fix [files...] [--dry-run] [--interactive] [--max-iterations <n>]"},
                {"name": "list", "description": "List configured linters", "usage": "list [--format <text|json>]"},
                {"name": "config", "description": "Show configuration", "usage": "config"},
                {"name": "init", "description": "Initialize linter configuration", "usage": "init [--force]"}
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
        "run" | "" => cmd_run(&cwd, &positional, &options_value),
        "fix" => cmd_fix(&cwd, &positional, &options_value),
        "list" => cmd_list(&cwd, &options_value),
        "config" => cmd_config(&cwd),
        "init" => cmd_init(&cwd, &options_value),
        _ => Err(format!("Unknown command: {}", subcommand)),
    }
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
        return Ok("[!] No linters configured. Run `adi lint init` to create a config.".to_string());
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
        return Ok("[!] No linters configured. Run `adi lint init` to create a config.".to_string());
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
    let config_path = project_path.join(".adi").join("linter.toml");

    if config_path.exists() {
        let content =
            std::fs::read_to_string(&config_path).map_err(|e| format!("Read error: {}", e))?;
        return Ok(format!(
            "Config: {}\n\n{}",
            config_path.display(),
            content
        ));
    }

    let alt_path = project_path.join("linter.toml");
    if alt_path.exists() {
        let content =
            std::fs::read_to_string(&alt_path).map_err(|e| format!("Read error: {}", e))?;
        return Ok(format!("Config: {}\n\n{}", alt_path.display(), content));
    }

    Ok("[!] No config found. Run `adi lint init` to create one.".to_string())
}

fn cmd_init(project_path: &PathBuf, options: &serde_json::Value) -> Result<String, String> {
    let force = options
        .get("force")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let config_dir = project_path.join(".adi");
    let config_path = config_dir.join("linter.toml");

    if config_path.exists() && !force {
        return Ok(format!(
            "[!] Config already exists at {}\n  Use --force to overwrite",
            config_path.display()
        ));
    }

    std::fs::create_dir_all(&config_dir).map_err(|e| format!("Create dir error: {}", e))?;

    let default_config = r#"# ADI Linter Configuration

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

# Example command linter
# [[rules.command]]
# id = "no-todo"
# category = "code-quality"
# type = "regex-forbid"
# pattern = "TODO|FIXME"
# message = "Unresolved TODO comment"
# glob = "**/*.rs"
# severity = "warning"

# Example external linter
# [[rules.exec]]
# id = "shellcheck"
# category = "correctness"
# exec = "shellcheck -f json {file}"
# glob = "**/*.sh"
# output = "json"
"#;

    std::fs::write(&config_path, default_config).map_err(|e| format!("Write error: {}", e))?;

    Ok(format!(
        "[+] Created {}\n  Edit the file to add your linting rules.",
        config_path.display()
    ))
}
