//! ADI Browser Debug Plugin
//!
//! Provides CLI commands for browser debugging:
//! - `adi browser-debug list-tabs` - List available debug tabs
//! - `adi browser-debug network <token>` - Get network requests
//! - `adi browser-debug console <token>` - Get console logs

use abi_stable::std_types::{ROption, RResult, RStr, RString, RVec};
use lib_plugin_abi::{
    PluginContext, PluginInfo, PluginVTable, ServiceDescriptor, ServiceError, ServiceHandle,
    ServiceMethod, ServiceVTable, ServiceVersion,
};
use once_cell::sync::Lazy;
use std::ffi::c_void;
use std::sync::Mutex;

const SERVICE_CLI: &str = "adi.browser-debug.cli";

// Global runtime for async operations
static RUNTIME: Lazy<Mutex<Option<tokio::runtime::Runtime>>> = Lazy::new(|| {
    Mutex::new(Some(
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("Failed to create Tokio runtime"),
    ))
});

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

extern "C" fn plugin_info() -> PluginInfo {
    PluginInfo::new(
        "adi.browser-debug",
        "ADI Browser Debug",
        env!("CARGO_PKG_VERSION"),
        "extension",
    )
    .with_author("ADI Team")
    .with_description("Browser debugging - inspect network and console from ADI-proxied pages")
    .with_min_host_version("0.8.0")
}

extern "C" fn plugin_init(ctx: *mut PluginContext) -> i32 {
    unsafe {
        let host = (*ctx).host();

        // Register CLI service
        let cli_descriptor = ServiceDescriptor::new(
            SERVICE_CLI,
            ServiceVersion::new(1, 0, 0),
            "adi.browser-debug",
        )
        .with_description("CLI commands for browser debugging");

        let cli_handle = ServiceHandle::new(
            SERVICE_CLI,
            ctx as *const c_void,
            &CLI_SERVICE_VTABLE as *const ServiceVTable,
        );

        if let Err(code) = host.register_svc(cli_descriptor, cli_handle) {
            host.error(&format!("Failed to register CLI service: {}", code));
            return code;
        }

        host.info("Browser Debug plugin initialized");
    }
    0
}

extern "C" fn plugin_cleanup(_ctx: *mut PluginContext) {
    // Shutdown runtime
    if let Ok(mut guard) = RUNTIME.lock() {
        if let Some(rt) = guard.take() {
            rt.shutdown_background();
        }
    }
}

// === CLI Service ===
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
            let commands = serde_json::json!([
                {
                    "name": "list-tabs",
                    "description": "List all browser tabs with debug tokens available for inspection",
                    "usage": "list-tabs"
                },
                {
                    "name": "network",
                    "description": "Get network requests from a browser tab",
                    "usage": "network <token> [--url <pattern>] [--method <methods>] [--status <min>-<max>] [--limit <n>]"
                },
                {
                    "name": "console",
                    "description": "Get console logs from a browser tab",
                    "usage": "console <token> [--level <levels>] [--message <pattern>] [--limit <n>]"
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
        "list-tabs" => cmd_list_tabs(),
        "network" => cmd_network(&args[1..]),
        "console" => cmd_console(&args[1..]),
        "" | "help" | "-h" | "--help" => Ok(get_help()),
        _ => Err(format!(
            "Unknown command: {}\n\nRun 'adi browser-debug help' for usage.",
            subcommand
        )),
    }
}

fn get_help() -> String {
    r#"ADI Browser Debug - Inspect network and console from browser tabs

USAGE:
    adi browser-debug <COMMAND> [OPTIONS]

COMMANDS:
    list-tabs     List all browser tabs with debug tokens available
    network       Get network requests from a browser tab
    console       Get console logs from a browser tab
    help          Show this help message

EXAMPLES:
    # List available debug tabs
    adi browser-debug list-tabs

    # Get network requests from a tab
    adi browser-debug network <token>

    # Get only error console logs
    adi browser-debug console <token> --level error,warn

    # Get failed network requests
    adi browser-debug network <token> --status 400-599

SETUP:
    1. Install the ADI Browser Debugger Chrome extension
    2. Browse to a page served through an ADI-proxied cocoon
    3. The extension will automatically detect and register the debug session

ENVIRONMENT:
    SIGNALING_URL    Signaling server URL (default: wss://adi.the-ihor.com/api/signaling/ws)
    ACCESS_TOKEN     Authentication token"#
        .to_string()
}

fn get_client() -> Result<adi_browser_debug_core::client::BrowserDebugClient, String> {
    let signaling_url = std::env::var("SIGNALING_URL")
        .unwrap_or_else(|_| "wss://adi.the-ihor.com/api/signaling/ws".to_string());
    let access_token = std::env::var("ACCESS_TOKEN").unwrap_or_default();

    let rt_guard = RUNTIME
        .lock()
        .map_err(|e| format!("Runtime lock error: {}", e))?;
    let rt = rt_guard
        .as_ref()
        .ok_or_else(|| "Runtime not available".to_string())?;

    rt.block_on(async {
        adi_browser_debug_core::client::BrowserDebugClient::connect(&signaling_url, &access_token)
            .await
            .map_err(|e| format!("Connection error: {}", e))
    })
}

fn cmd_list_tabs() -> Result<String, String> {
    let client = get_client()?;

    let rt_guard = RUNTIME
        .lock()
        .map_err(|e| format!("Runtime lock error: {}", e))?;
    let rt = rt_guard
        .as_ref()
        .ok_or_else(|| "Runtime not available".to_string())?;

    let tabs = rt.block_on(async {
        client
            .list_tabs()
            .await
            .map_err(|e| format!("Failed to list tabs: {}", e))
    })?;

    if tabs.is_empty() {
        return Ok("No browser debug tabs available.\n\n\
             To debug a browser tab:\n\
             1. Install the ADI Browser Debugger Chrome extension\n\
             2. Browse to a page served through an ADI-proxied cocoon\n\
             3. The extension will automatically detect the debug token"
            .to_string());
    }

    let mut output = format!("Found {} debug tab(s):\n\n", tabs.len());
    for tab in &tabs {
        output.push_str(&format!(
            "Token: {}\n  URL: {}\n  Title: {}\n  Cocoon: {} ({})\n\n",
            tab.token,
            tab.url,
            tab.title,
            tab.cocoon_id,
            tab.cocoon_name.as_deref().unwrap_or("unnamed")
        ));
    }

    Ok(output)
}

fn cmd_network(args: &[String]) -> Result<String, String> {
    if args.is_empty() {
        return Err(
            "Missing token argument.\n\nUsage: adi browser-debug network <token> [options]"
                .to_string(),
        );
    }

    let token = &args[0];
    let mut filters = adi_browser_debug_core::NetworkFilters::default();

    // Parse options
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--url" | "-u" => {
                i += 1;
                if i < args.len() {
                    filters.url_pattern = Some(args[i].clone());
                }
            }
            "--method" | "-m" => {
                i += 1;
                if i < args.len() {
                    filters.method = Some(
                        args[i]
                            .split(',')
                            .map(|s| s.trim().to_uppercase())
                            .collect(),
                    );
                }
            }
            "--status" | "-s" => {
                i += 1;
                if i < args.len() {
                    let parts: Vec<&str> = args[i].split('-').collect();
                    if let Some(min) = parts.first().and_then(|s| s.parse().ok()) {
                        filters.status_min = Some(min);
                    }
                    if let Some(max) = parts.get(1).and_then(|s| s.parse().ok()) {
                        filters.status_max = Some(max);
                    }
                }
            }
            "--limit" | "-l" => {
                i += 1;
                if i < args.len() {
                    filters.limit = args[i].parse().ok();
                }
            }
            _ => {}
        }
        i += 1;
    }

    let client = get_client()?;

    let rt_guard = RUNTIME
        .lock()
        .map_err(|e| format!("Runtime lock error: {}", e))?;
    let rt = rt_guard
        .as_ref()
        .ok_or_else(|| "Runtime not available".to_string())?;

    let has_filters = filters.url_pattern.is_some()
        || filters.method.is_some()
        || filters.status_min.is_some()
        || filters.status_max.is_some()
        || filters.limit.is_some();

    let requests = rt.block_on(async {
        client
            .get_network(token, if has_filters { Some(filters) } else { None })
            .await
            .map_err(|e| format!("Failed to get network: {}", e))
    })?;

    if requests.is_empty() {
        return Ok("No network requests captured yet.".to_string());
    }

    let mut output = format!("Network requests ({}):\n\n", requests.len());
    for req in &requests {
        let status = req
            .status
            .map(|s| s.to_string())
            .unwrap_or_else(|| "pending".to_string());
        let duration = req
            .duration_ms
            .map(|ms| format!(" ({}ms)", ms))
            .unwrap_or_default();
        let error = req
            .error
            .as_ref()
            .map(|e| format!(" [ERROR: {}]", e))
            .unwrap_or_default();

        output.push_str(&format!(
            "{} {} - {}{}{}\n",
            req.method, req.url, status, duration, error
        ));
    }

    Ok(output)
}

fn cmd_console(args: &[String]) -> Result<String, String> {
    if args.is_empty() {
        return Err(
            "Missing token argument.\n\nUsage: adi browser-debug console <token> [options]"
                .to_string(),
        );
    }

    let token = &args[0];
    let mut filters = adi_browser_debug_core::ConsoleFilters::default();

    // Parse options
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--level" | "-l" => {
                i += 1;
                if i < args.len() {
                    let levels: Vec<adi_browser_debug_core::ConsoleLevel> = args[i]
                        .split(',')
                        .filter_map(|s| match s.trim().to_lowercase().as_str() {
                            "log" => Some(adi_browser_debug_core::ConsoleLevel::Log),
                            "debug" => Some(adi_browser_debug_core::ConsoleLevel::Debug),
                            "info" => Some(adi_browser_debug_core::ConsoleLevel::Info),
                            "warn" => Some(adi_browser_debug_core::ConsoleLevel::Warn),
                            "error" => Some(adi_browser_debug_core::ConsoleLevel::Error),
                            _ => None,
                        })
                        .collect();
                    if !levels.is_empty() {
                        filters.level = Some(levels);
                    }
                }
            }
            "--message" | "-m" => {
                i += 1;
                if i < args.len() {
                    filters.message_pattern = Some(args[i].clone());
                }
            }
            "--limit" | "-n" => {
                i += 1;
                if i < args.len() {
                    filters.limit = args[i].parse().ok();
                }
            }
            _ => {}
        }
        i += 1;
    }

    let client = get_client()?;

    let rt_guard = RUNTIME
        .lock()
        .map_err(|e| format!("Runtime lock error: {}", e))?;
    let rt = rt_guard
        .as_ref()
        .ok_or_else(|| "Runtime not available".to_string())?;

    let has_filters =
        filters.level.is_some() || filters.message_pattern.is_some() || filters.limit.is_some();

    let entries = rt.block_on(async {
        client
            .get_console(token, if has_filters { Some(filters) } else { None })
            .await
            .map_err(|e| format!("Failed to get console: {}", e))
    })?;

    if entries.is_empty() {
        return Ok("No console entries captured yet.".to_string());
    }

    let mut output = format!("Console entries ({}):\n\n", entries.len());
    for entry in &entries {
        let level = match entry.level {
            adi_browser_debug_core::ConsoleLevel::Log => "LOG",
            adi_browser_debug_core::ConsoleLevel::Debug => "DEBUG",
            adi_browser_debug_core::ConsoleLevel::Info => "INFO",
            adi_browser_debug_core::ConsoleLevel::Warn => "WARN",
            adi_browser_debug_core::ConsoleLevel::Error => "ERROR",
        };
        let source = entry
            .source
            .as_ref()
            .map(|s| format!(" ({}:{})", s, entry.line.unwrap_or(0)))
            .unwrap_or_default();

        output.push_str(&format!("[{}] {}{}\n", level, entry.message, source));
    }

    Ok(output)
}
