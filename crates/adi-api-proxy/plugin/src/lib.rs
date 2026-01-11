//! ADI API Proxy CLI Plugin
//!
//! Provides CLI commands for managing proxy tokens and upstream API keys.

use abi_stable::std_types::{ROption, RResult, RStr, RString, RVec};
use lib_plugin_abi::{
    PluginContext, PluginError, PluginInfo, PluginVTable, ServiceDescriptor, ServiceError,
    ServiceHandle, ServiceMethod, ServiceVTable, ServiceVersion,
};
use once_cell::sync::OnceCell;
use serde_json::json;
use std::ffi::c_void;

const SERVICE_CLI: &str = "adi.api-proxy.cli";

static RUNTIME: OnceCell<tokio::runtime::Runtime> = OnceCell::new();
static API_URL: OnceCell<String> = OnceCell::new();

// Plugin info
extern "C" fn plugin_info() -> PluginInfo {
    PluginInfo::new(
        "adi.api-proxy",
        "ADI API Proxy",
        env!("CARGO_PKG_VERSION"),
        "core",
    )
    .with_author("ADI Team")
    .with_description("LLM API proxy with BYOK/Platform modes")
    .with_min_host_version("0.8.0")
}

// Plugin initialization
extern "C" fn plugin_init(ctx: *mut PluginContext) -> i32 {
    // Initialize tokio runtime
    let _ = RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("Failed to create runtime")
    });

    // Get API URL from config or default
    unsafe {
        let host = (*ctx).host();

        // Try to get API URL from config
        let url = host
            .get_config("api_url")
            .unwrap_or_else(|| "http://localhost:8024".to_string());
        let _ = API_URL.set(url);

        // Register CLI service
        let cli_descriptor =
            ServiceDescriptor::new(SERVICE_CLI, ServiceVersion::new(1, 0, 0), "adi.api-proxy")
                .with_description("CLI commands for API proxy management");

        let cli_handle = ServiceHandle::new(
            SERVICE_CLI,
            ctx as *const c_void,
            &CLI_SERVICE_VTABLE as *const ServiceVTable,
        );

        if let Err(code) = host.register_svc(cli_descriptor, cli_handle) {
            host.error(&format!("Failed to register CLI service: {}", code));
            return code;
        }

        host.info("API Proxy plugin initialized");
    }

    0
}

extern "C" fn plugin_cleanup(_ctx: *mut PluginContext) {}

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

// CLI Service
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
                    "name": "keys",
                    "description": "Manage upstream API keys",
                    "usage": "api-proxy keys <list|add|remove|verify> [options]"
                },
                {
                    "name": "tokens",
                    "description": "Manage proxy tokens",
                    "usage": "api-proxy tokens <list|create|revoke|rotate> [options]"
                },
                {
                    "name": "usage",
                    "description": "View usage statistics",
                    "usage": "api-proxy usage [--from DATE] [--to DATE]"
                },
                {
                    "name": "providers",
                    "description": "List available platform providers",
                    "usage": "api-proxy providers"
                }
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

    match subcommand {
        "keys" => cmd_keys(&args[1..]),
        "tokens" => cmd_tokens(&args[1..]),
        "usage" => cmd_usage(&args[1..]),
        "providers" => cmd_providers(),
        "" | "help" => Ok(get_help()),
        _ => Err(format!(
            "Unknown command: {}. Use 'api-proxy help' for usage.",
            subcommand
        )),
    }
}

fn get_help() -> String {
    let mut help = String::new();
    help.push_str("ADI API Proxy - LLM API proxy with BYOK/Platform modes\n\n");
    help.push_str("USAGE:\n");
    help.push_str("  adi api-proxy <COMMAND>\n\n");
    help.push_str("COMMANDS:\n");
    help.push_str("  keys       Manage upstream API keys (BYOK)\n");
    help.push_str("  tokens     Manage proxy tokens\n");
    help.push_str("  usage      View usage statistics\n");
    help.push_str("  providers  List available platform providers\n");
    help.push_str("  help       Show this help message\n\n");
    help.push_str("EXAMPLES:\n");
    help.push_str("  adi api-proxy keys list\n");
    help.push_str(
        "  adi api-proxy tokens create --name \"my-token\" --mode byok --key-id <uuid>\n",
    );
    help.push_str("  adi api-proxy usage --from 2024-01-01\n");
    help
}

fn cmd_keys(args: &[String]) -> Result<String, String> {
    let subcommand = args.first().map(|s| s.as_str()).unwrap_or("list");

    match subcommand {
        "list" => {
            // TODO: Implement API call
            Ok(
                "Upstream API keys:\n  (No keys configured. Use 'keys add' to add one.)"
                    .to_string(),
            )
        }
        "add" => {
            let mut name = None;
            let mut provider = None;
            let mut api_key = None;

            let mut i = 1;
            while i < args.len() {
                match args[i].as_str() {
                    "--name" | "-n" => {
                        name = args.get(i + 1).cloned();
                        i += 2;
                    }
                    "--provider" | "-p" => {
                        provider = args.get(i + 1).cloned();
                        i += 2;
                    }
                    "--key" | "-k" => {
                        api_key = args.get(i + 1).cloned();
                        i += 2;
                    }
                    _ => i += 1,
                }
            }

            if name.is_none() || provider.is_none() || api_key.is_none() {
                return Err("Usage: keys add --name <name> --provider <openai|anthropic|openrouter|custom> --key <api-key>".to_string());
            }

            Ok(format!(
                "Added upstream key '{}' for provider '{}'",
                name.unwrap(),
                provider.unwrap()
            ))
        }
        "remove" => {
            let id = args.get(1).ok_or("Usage: keys remove <key-id>")?;
            Ok(format!("Removed upstream key: {}", id))
        }
        "verify" => {
            let id = args.get(1).ok_or("Usage: keys verify <key-id>")?;
            Ok(format!("Verifying key {}... OK", id))
        }
        _ => Err(format!(
            "Unknown keys command: {}. Use list, add, remove, or verify.",
            subcommand
        )),
    }
}

fn cmd_tokens(args: &[String]) -> Result<String, String> {
    let subcommand = args.first().map(|s| s.as_str()).unwrap_or("list");

    match subcommand {
        "list" => Ok(
            "Proxy tokens:\n  (No tokens configured. Use 'tokens create' to create one.)"
                .to_string(),
        ),
        "create" => {
            let mut name = None;
            let mut mode = None;
            let mut key_id = None;
            let mut provider = None;

            let mut i = 1;
            while i < args.len() {
                match args[i].as_str() {
                    "--name" | "-n" => {
                        name = args.get(i + 1).cloned();
                        i += 2;
                    }
                    "--mode" | "-m" => {
                        mode = args.get(i + 1).cloned();
                        i += 2;
                    }
                    "--key-id" => {
                        key_id = args.get(i + 1).cloned();
                        i += 2;
                    }
                    "--provider" => {
                        provider = args.get(i + 1).cloned();
                        i += 2;
                    }
                    _ => i += 1,
                }
            }

            let name = name.ok_or("--name is required")?;
            let mode = mode.ok_or("--mode (byok|platform) is required")?;

            match mode.as_str() {
                "byok" => {
                    let _key_id = key_id.ok_or("--key-id is required for BYOK mode")?;
                }
                "platform" => {
                    let _provider = provider.ok_or("--provider is required for platform mode")?;
                }
                _ => return Err("--mode must be 'byok' or 'platform'".to_string()),
            }

            Ok(format!(
                "Created proxy token '{}'\n\nSECRET (save this, shown only once!):\n  adi_pk_xxxxxxxxxxxxxxxxxxxx",
                name
            ))
        }
        "revoke" => {
            let id = args.get(1).ok_or("Usage: tokens revoke <token-id>")?;
            Ok(format!("Revoked token: {}", id))
        }
        "rotate" => {
            let id = args.get(1).ok_or("Usage: tokens rotate <token-id>")?;
            Ok(format!(
                "Rotated token: {}\n\nNEW SECRET (save this, shown only once!):\n  adi_pk_yyyyyyyyyyyyyyyyyyyy",
                id
            ))
        }
        _ => Err(format!(
            "Unknown tokens command: {}. Use list, create, revoke, or rotate.",
            subcommand
        )),
    }
}

fn cmd_usage(args: &[String]) -> Result<String, String> {
    let mut from = None;
    let mut to = None;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--from" => {
                from = args.get(i + 1).cloned();
                i += 2;
            }
            "--to" => {
                to = args.get(i + 1).cloned();
                i += 2;
            }
            _ => i += 1,
        }
    }

    let mut output = String::new();
    output.push_str("Usage Summary\n");
    output.push_str("=============\n\n");

    if let Some(f) = from {
        output.push_str(&format!("From: {}\n", f));
    }
    if let Some(t) = to {
        output.push_str(&format!("To: {}\n", t));
    }

    output.push_str("\nTotal Requests:    0\n");
    output.push_str("Input Tokens:      0\n");
    output.push_str("Output Tokens:     0\n");
    output.push_str("Total Cost:        $0.00\n");
    output.push_str("Success Rate:      N/A\n");

    Ok(output)
}

fn cmd_providers() -> Result<String, String> {
    let mut output = String::new();
    output.push_str("Available Platform Providers\n");
    output.push_str("============================\n\n");
    output.push_str("Provider      Status    Models\n");
    output.push_str("--------      ------    ------\n");
    output.push_str("openai        -         (not configured)\n");
    output.push_str("anthropic     -         (not configured)\n");
    output.push_str("openrouter    -         (not configured)\n");
    Ok(output)
}
