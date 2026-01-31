//! ADI Coolify Plugin
//!
//! Provides CLI commands for Coolify deployment management.
//!
//! ## Configuration
//!
//! Config is loaded from multiple sources (highest priority first):
//! 1. Environment variables: `ADI_PLUGIN_ADI_COOLIFY_<KEY>`
//! 2. Project config: `.adi/plugins/adi.coolify.toml`
//! 3. User config: `~/.config/adi/plugins/adi.coolify.toml`
//! 4. Defaults
//!
//! ### Config Keys
//! - `url` - Coolify instance URL (default: http://in.the-ihor.com)
//! - `api_key` - API token (required, encrypted at rest)
//!
//! ### Environment Variables
//! - `ADI_PLUGIN_ADI_COOLIFY_URL`
//! - `ADI_PLUGIN_ADI_COOLIFY_API_KEY`
//!
//! ## Security
//!
//! The `api_key` is marked as a secret and will be encrypted when stored
//! in config files using ChaCha20-Poly1305 encryption.

use abi_stable::std_types::{ROption, RResult, RStr, RString, RVec};
use lib_plugin_abi::{
    PluginContext, PluginError, PluginInfo, PluginVTable, ServiceDescriptor, ServiceError,
    ServiceHandle, ServiceMethod, ServiceVTable, ServiceVersion,
};
use lib_plugin_host::PluginConfigManager;

use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::ffi::c_void;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

const SERVICE_CLI: &str = "adi.coolify.cli";
const PLUGIN_ID: &str = "adi.coolify";
const DEFAULT_COOLIFY_URL: &str = "http://in.the-ihor.com";

/// Config keys
const KEY_URL: &str = "url";
const KEY_API_KEY: &str = "api_key";

/// Secret keys (will be encrypted at rest)
const SECRET_KEYS: &[&str] = &[KEY_API_KEY];

static CONFIG_MANAGER: OnceCell<Arc<PluginConfigManager>> = OnceCell::new();
static PROJECT_PATH: OnceCell<RwLock<Option<PathBuf>>> = OnceCell::new();

fn get_config_manager() -> Option<&'static Arc<PluginConfigManager>> {
    CONFIG_MANAGER.get()
}

fn get_url() -> String {
    get_config_manager()
        .and_then(|m| m.get(KEY_URL))
        .unwrap_or_else(|| DEFAULT_COOLIFY_URL.to_string())
}

fn get_api_key() -> Option<String> {
    get_config_manager().and_then(|m| m.get(KEY_API_KEY))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ServiceInfo {
    id: String,
    name: String,
    uuid: String,
}

fn get_services() -> Vec<ServiceInfo> {
    vec![
        ServiceInfo {
            id: "auth".to_string(),
            name: "Auth API".to_string(),
            uuid: "ngg488ogoc80c8wogowkckow".to_string(),
        },
        ServiceInfo {
            id: "platform".to_string(),
            name: "Platform API".to_string(),
            uuid: "cosw4cw0gscso88w8sskgk8g".to_string(),
        },
        ServiceInfo {
            id: "signaling".to_string(),
            name: "Signaling Server".to_string(),
            uuid: "t0k0owcw00w00s4w4o0c000w".to_string(),
        },
        ServiceInfo {
            id: "web".to_string(),
            name: "Web UI".to_string(),
            uuid: "tkg84kg0o0ok8gkcs8wcggck".to_string(),
        },
        ServiceInfo {
            id: "analytics-ingestion".to_string(),
            name: "Analytics Ingestion".to_string(),
            uuid: "TODO_COOLIFY_UUID".to_string(),
        },
        ServiceInfo {
            id: "analytics".to_string(),
            name: "Analytics API".to_string(),
            uuid: "TODO_COOLIFY_UUID".to_string(),
        },
        ServiceInfo {
            id: "registry".to_string(),
            name: "Plugin Registry".to_string(),
            uuid: "TODO_COOLIFY_UUID".to_string(),
        },
    ]
}

fn find_service(id: &str) -> Option<ServiceInfo> {
    get_services().into_iter().find(|s| s.id == id)
}

fn api_call(method: &str, endpoint: &str) -> Result<serde_json::Value, String> {
    let api_key = get_api_key().ok_or_else(|| {
        "API key not configured. Set via:\n  \
         - Environment: ADI_PLUGIN_ADI_COOLIFY_API_KEY=<key>\n  \
         - User config: adi coolify config set api_key <key>\n  \
         - Project config: adi coolify config set api_key <key> --project"
            .to_string()
    })?;

    let url = format!("{}/api/v1{}", get_url(), endpoint);
    let client = reqwest::blocking::Client::new();

    let response = match method {
        "GET" => client
            .get(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .send(),
        "POST" => client
            .post(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .send(),
        _ => return Err(format!("Unsupported method: {}", method)),
    };

    match response {
        Ok(resp) => {
            let text = resp.text().map_err(|e| e.to_string())?;
            serde_json::from_str(&text).map_err(|e| format!("JSON parse error: {} - {}", e, text))
        }
        Err(e) => Err(format!("Request failed: {}", e)),
    }
}

fn status_icon(status: &str) -> &'static str {
    match status {
        "running:healthy" | "running" | "finished" | "success" => "●",
        "running:unhealthy" | "running:unknown" => "◐",
        "queued" => "○",
        "in_progress" | "building" => "◐",
        s if s.starts_with("exited") => "✗",
        "failed" | "error" | "cancelled" | "stopped" => "✗",
        _ => "?",
    }
}

fn status_label(status: &str) -> &str {
    match status {
        "running:healthy" => "healthy",
        "running:unhealthy" => "unhealthy",
        "running:unknown" => "unknown",
        "in_progress" => "building",
        _ => status,
    }
}

extern "C" fn plugin_info() -> PluginInfo {
    PluginInfo::new(PLUGIN_ID, "ADI Coolify", env!("CARGO_PKG_VERSION"), "core")
        .with_author("ADI Team")
        .with_description("Coolify deployment management")
        .with_min_host_version("0.8.0")
}

extern "C" fn plugin_init(ctx: *mut PluginContext) -> i32 {
    let _ = PROJECT_PATH.set(RwLock::new(None));

    unsafe {
        let host = (*ctx).host();

        // Get data directory from host
        let data_dir = PathBuf::from((*host).get_data_dir().as_str());

        // Initialize config manager with secrets support
        let mut defaults = HashMap::new();
        defaults.insert(KEY_URL.to_string(), DEFAULT_COOLIFY_URL.to_string());

        let manager = PluginConfigManager::new(PLUGIN_ID, data_dir)
            .with_defaults(defaults)
            .with_secrets(SECRET_KEYS.iter().copied());
        manager.load();

        let _ = CONFIG_MANAGER.set(Arc::new(manager));

        let cli_descriptor =
            ServiceDescriptor::new(SERVICE_CLI, ServiceVersion::new(1, 0, 0), PLUGIN_ID)
                .with_description("CLI commands for Coolify deployment");

        let cli_handle = ServiceHandle::new(
            SERVICE_CLI,
            ctx as *const c_void,
            &CLI_SERVICE_VTABLE as *const ServiceVTable,
        );

        if let Err(code) = (*host).register_svc(cli_descriptor, cli_handle) {
            (*host).error(&format!(
                "Failed to register CLI commands service: {}",
                code
            ));
            return code;
        }

        (*host).info("ADI Coolify plugin initialized");
    }

    0
}

extern "C" fn plugin_cleanup(_ctx: *mut PluginContext) {}

extern "C" fn handle_message(
    _ctx: *mut PluginContext,
    msg_type: RStr<'_>,
    msg_data: RStr<'_>,
) -> RResult<RString, PluginError> {
    match msg_type.as_str() {
        "set_project_path" => {
            if let Some(path_lock) = PROJECT_PATH.get() {
                if let Ok(mut path) = path_lock.write() {
                    *path = Some(PathBuf::from(msg_data.as_str()));
                    return RResult::ROk(RString::from("ok"));
                }
            }
            RResult::RErr(PluginError::new(
                1,
                "Failed to set project path".to_string(),
            ))
        }
        _ => RResult::RErr(PluginError::new(
            -1,
            format!("Unknown message type: {}", msg_type.as_str()),
        )),
    }
}

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
                {"name": "status", "description": "Show status of all services", "usage": "status"},
                {"name": "deploy", "description": "Deploy a service", "usage": "deploy <service|all> [--force]"},
                {"name": "watch", "description": "Watch deployment progress", "usage": "watch <service>"},
                {"name": "logs", "description": "Show deployment logs", "usage": "logs <service>"},
                {"name": "list", "description": "List recent deployments", "usage": "list <service> [count]"},
                {"name": "services", "description": "List available services", "usage": "services"},
                {"name": "config", "description": "Show current configuration", "usage": "config"},
                {"name": "config set", "description": "Set a config value", "usage": "config set <key> <value> [--user|--project]"}
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
    let cmd_args: Vec<&str> = args.iter().skip(1).map(|s| s.as_str()).collect();

    let mut options = HashMap::new();
    let mut i = 0;
    while i < cmd_args.len() {
        if cmd_args[i].starts_with("--") {
            let key = cmd_args[i].trim_start_matches("--");
            if i + 1 < cmd_args.len() && !cmd_args[i + 1].starts_with("--") {
                options.insert(key.to_string(), cmd_args[i + 1].to_string());
                i += 2;
            } else {
                options.insert(key.to_string(), "true".to_string());
                i += 1;
            }
        } else {
            i += 1;
        }
    }

    let positional: Vec<&str> = cmd_args
        .iter()
        .filter(|a| !a.starts_with("--"))
        .copied()
        .collect();

    match subcommand {
        "status" => cmd_status(),
        "deploy" => cmd_deploy(&positional, &options),
        "watch" => cmd_watch(&positional),
        "logs" => cmd_logs(&positional),
        "list" => cmd_list(&positional),
        "services" => cmd_services(),
        "config" => {
            if positional.first().map(|s| *s) == Some("set") {
                cmd_config_set(&positional[1..], &options)
            } else {
                cmd_config_show()
            }
        }
        "" => Ok(get_help()),
        _ => Err(format!("Unknown command: {}", subcommand)),
    }
}

fn get_help() -> String {
    format!(
        r#"ADI Coolify - Deployment Management

Commands:
  status              Show status of all services
  deploy <svc|all>    Deploy a service (use 'all' for all services)
  deploy <svc> -f     Force rebuild (no cache)
  watch <svc>         Watch deployment progress
  logs <svc>          Show deployment logs
  list <svc> [n]      List recent deployments (default: 5)
  services            List available services
  config              Show current configuration
  config set <k> <v>  Set config value (--user or --project)

Services:
  auth                Auth API (adi-auth)
  platform            Platform API (adi-platform-api)
  signaling           Signaling Server (tarminal-signaling-server)
  web                 Web UI (infra-service-web)
  analytics-ingestion Analytics Ingestion (adi-analytics-ingestion)
  analytics           Analytics API (adi-analytics-api)
  registry            Plugin Registry (adi-plugin-registry-http)

Configuration:
  Config is loaded from (highest priority first):
  1. Environment: ADI_PLUGIN_ADI_COOLIFY_<KEY>
  2. Project: .adi/plugins/{}.toml
  3. User: ~/.config/adi/plugins/{}.toml

  Keys: url, api_key

Usage: adi coolify <command> [args]"#,
        PLUGIN_ID, PLUGIN_ID
    )
}

fn cmd_config_show() -> Result<String, String> {
    let manager = get_config_manager();
    let url = get_url();
    let api_key = get_api_key();
    let api_key_display = api_key
        .as_ref()
        .map(|k| {
            if k.len() > 8 {
                format!("{}...{}", &k[..4], &k[k.len() - 4..])
            } else {
                "****".to_string()
            }
        })
        .unwrap_or_else(|| "(not set)".to_string());

    let user_config_path = manager
        .and_then(|m| m.user_config_path())
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "(unavailable)".to_string());

    let project_config_path = PROJECT_PATH
        .get()
        .and_then(|p| p.read().ok())
        .and_then(|p| p.clone())
        .map(|p| p.join(".adi/plugins").join(format!("{}.toml", PLUGIN_ID)))
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "(no project)".to_string());

    Ok(format!(
        r#"ADI Coolify Configuration

Current Values:
  url       = {}
  api_key   = {} (secret, encrypted at rest)

Config Files:
  User:    {}
  Project: {}

Environment Variables:
  ADI_PLUGIN_ADI_COOLIFY_URL
  ADI_PLUGIN_ADI_COOLIFY_API_KEY

Set config:
  adi coolify config set api_key <value>           # user-level (encrypted)
  adi coolify config set api_key <value> --project # project-level (encrypted)
  adi coolify config set url <value>               # user-level

Encryption:
  Secrets are encrypted using ChaCha20-Poly1305.
  Master key stored at: ~/.config/adi/secrets.key"#,
        url, api_key_display, user_config_path, project_config_path
    ))
}

fn cmd_config_set(args: &[&str], options: &HashMap<String, String>) -> Result<String, String> {
    if args.len() < 2 {
        return Err("Usage: config set <key> <value> [--user|--project]".to_string());
    }

    let key = args[0];
    let value = args[1];

    // Validate key
    if key != KEY_URL && key != KEY_API_KEY {
        return Err(format!(
            "Unknown config key: '{}'. Valid keys: url, api_key",
            key
        ));
    }

    let manager = get_config_manager().ok_or("Config manager not initialized")?;

    let is_project = options.contains_key("project") || options.contains_key("p");
    let is_secret = manager.is_secret(key);
    let display_value = if is_secret { "(encrypted)" } else { value };

    if is_project {
        // Update manager's project path if needed
        let project_path = PROJECT_PATH
            .get()
            .and_then(|p| p.read().ok())
            .and_then(|p| p.clone())
            .ok_or("No project directory set. Run from a project directory.")?;

        // Create a new manager with project path for this operation
        let temp_manager = PluginConfigManager::new(PLUGIN_ID, manager.data_dir().clone())
            .with_project_dir(project_path.clone())
            .with_secrets(SECRET_KEYS.iter().copied());

        temp_manager
            .set_project(key, value)
            .map_err(|e| format!("Failed to save config: {}", e))?;

        let config_path = project_path
            .join(".adi/plugins")
            .join(format!("{}.toml", PLUGIN_ID));

        Ok(format!(
            "Set {} = {} in project config{}\nFile: {}",
            key,
            display_value,
            if is_secret { " (encrypted)" } else { "" },
            config_path.display()
        ))
    } else {
        manager
            .set_user(key, value)
            .map_err(|e| format!("Failed to save config: {}", e))?;

        let config_path = manager
            .user_config_path()
            .ok_or("Could not determine user config directory")?;

        Ok(format!(
            "Set {} = {} in user config{}\nFile: {}",
            key,
            display_value,
            if is_secret { " (encrypted)" } else { "" },
            config_path.display()
        ))
    }
}

fn cmd_status() -> Result<String, String> {
    let url = get_url();
    let mut output = format!(
        "ADI Deployment Status\nCoolify: {}\n\n{:<12} {:<20} {:<20}\n{}\n",
        url,
        "SERVICE",
        "NAME",
        "STATUS",
        "─".repeat(56)
    );

    for service in get_services() {
        let status = match api_call("GET", &format!("/applications/{}", service.uuid)) {
            Ok(info) => info
                .get("status")
                .and_then(|s| s.as_str())
                .unwrap_or("unknown")
                .to_string(),
            Err(_) => "error".to_string(),
        };

        let icon = status_icon(&status);
        let label = status_label(&status);
        output.push_str(&format!(
            "{:<12} {:<20} {} {}\n",
            service.id, service.name, icon, label
        ));
    }

    Ok(output.trim_end().to_string())
}

fn cmd_deploy(args: &[&str], options: &HashMap<String, String>) -> Result<String, String> {
    if args.is_empty() {
        return Err("Service name required. Usage: deploy <service|all> [--force]".to_string());
    }

    let service_id = args[0];
    let force = options.contains_key("force") || options.contains_key("f");
    let force_param = if force { "&force=true" } else { "" };

    let services_to_deploy: Vec<ServiceInfo> = if service_id == "all" {
        get_services()
    } else {
        match find_service(service_id) {
            Some(s) => vec![s],
            None => {
                let available: Vec<String> = get_services().iter().map(|s| s.id.clone()).collect();
                return Err(format!(
                    "Unknown service '{}'. Available: {}",
                    service_id,
                    available.join(", ")
                ));
            }
        }
    };

    let mut output = String::from("Deploying services...\n\n");
    let mut deployments = Vec::new();

    for service in &services_to_deploy {
        output.push_str(&format!("  {}: ", service.name));

        let endpoint = format!("/deploy?uuid={}{}", service.uuid, force_param);
        match api_call("GET", &endpoint) {
            Ok(result) => {
                if let Some(deps) = result.get("deployments").and_then(|d| d.as_array()) {
                    if let Some(first) = deps.first() {
                        if let Some(uuid) = first.get("deployment_uuid").and_then(|u| u.as_str()) {
                            output.push_str(&format!("Started ({})\n", uuid));
                            deployments.push((service.clone(), uuid.to_string()));
                            continue;
                        }
                    }
                }
                let error = result
                    .get("message")
                    .or_else(|| result.get("error"))
                    .and_then(|e| e.as_str())
                    .unwrap_or("Unknown error");
                output.push_str(&format!("Failed: {}\n", error));
            }
            Err(e) => {
                output.push_str(&format!("Failed: {}\n", e));
            }
        }
    }

    if !deployments.is_empty() {
        output.push_str("\nDeployment UUIDs:\n");
        for (service, uuid) in &deployments {
            output.push_str(&format!("  {}: {}\n", service.name, uuid));
        }
        output.push_str("\nUse 'adi coolify watch <service>' to monitor progress\n");
    }

    Ok(output.trim_end().to_string())
}

fn cmd_watch(args: &[&str]) -> Result<String, String> {
    if args.is_empty() {
        return Err("Service name required. Usage: watch <service>".to_string());
    }

    let service = find_service(args[0]).ok_or_else(|| format!("Unknown service: {}", args[0]))?;

    let endpoint = format!("/applications/{}/deployments?take=1", service.uuid);
    let result = api_call("GET", &endpoint)?;

    let deployments = result.as_array().ok_or("Invalid response format")?;
    if deployments.is_empty() {
        return Ok(format!("No deployments found for {}", service.name));
    }

    let latest = &deployments[0];
    let status = latest
        .get("status")
        .and_then(|s| s.as_str())
        .unwrap_or("unknown");
    let commit = latest
        .get("commit")
        .and_then(|c| c.as_str())
        .map(|c| &c[..c.len().min(7)])
        .unwrap_or("none");
    let uuid = latest
        .get("deployment_uuid")
        .and_then(|u| u.as_str())
        .unwrap_or("unknown");

    let icon = status_icon(status);
    let label = status_label(status);

    Ok(format!(
        "Watching {} deployments...\n\nLatest deployment:\n  UUID: {}\n  Status: {} {}\n  Commit: {}\n\nNote: For live watching, use: adi workflow deploy (select watch)\nService ID: {}",
        service.name, uuid, icon, label, commit, service.id
    ))
}

fn cmd_logs(args: &[&str]) -> Result<String, String> {
    if args.is_empty() {
        return Err("Service name required. Usage: logs <service>".to_string());
    }

    let service = find_service(args[0]).ok_or_else(|| format!("Unknown service: {}", args[0]))?;

    let endpoint = format!("/applications/{}/deployments?take=1", service.uuid);
    let result = api_call("GET", &endpoint)?;

    let deployments = result.as_array().ok_or("Invalid response format")?;
    if deployments.is_empty() {
        return Ok(format!("No deployments found for {}", service.name));
    }

    let deploy_uuid = deployments[0]
        .get("deployment_uuid")
        .and_then(|u| u.as_str())
        .ok_or("No deployment UUID")?;

    let deploy_info = api_call("GET", &format!("/deployments/{}", deploy_uuid))?;
    let logs = deploy_info
        .get("logs")
        .and_then(|l| l.as_str())
        .unwrap_or("No logs available");

    Ok(format!(
        "Deployment logs for {}\nDeployment: {}\n\n{}",
        service.name, deploy_uuid, logs
    ))
}

fn cmd_list(args: &[&str]) -> Result<String, String> {
    if args.is_empty() {
        return Err("Service name required. Usage: list <service> [count]".to_string());
    }

    let service = find_service(args[0]).ok_or_else(|| format!("Unknown service: {}", args[0]))?;

    let take: u32 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(5);

    let endpoint = format!("/applications/{}/deployments?take={}", service.uuid, take);
    let result = api_call("GET", &endpoint)?;

    let deployments = result.as_array().ok_or("Invalid response format")?;
    if deployments.is_empty() {
        return Ok(format!("No deployments found for {}", service.name));
    }

    let mut output = format!(
        "Recent deployments for {}\n\n{:<12} {:<15} {}\n{}\n",
        service.name,
        "STATUS",
        "COMMIT",
        "CREATED",
        "─".repeat(48)
    );

    for deploy in deployments {
        let status = deploy
            .get("status")
            .and_then(|s| s.as_str())
            .unwrap_or("unknown");
        let commit = deploy
            .get("commit")
            .and_then(|c| c.as_str())
            .map(|c| &c[..c.len().min(7)])
            .unwrap_or("none");
        let created = deploy
            .get("created_at")
            .and_then(|c| c.as_str())
            .map(|c| &c[..c.len().min(16)])
            .unwrap_or("unknown");

        let icon = status_icon(status);
        output.push_str(&format!(
            "{} {:<10} {:<15} {}\n",
            icon, status, commit, created
        ));
    }

    Ok(output.trim_end().to_string())
}

fn cmd_services() -> Result<String, String> {
    let mut output = String::from("Available Services\n\n");
    output.push_str(&format!("{:<20} {:<25} {}\n", "ID", "NAME", "UUID"));
    output.push_str(&format!("{}\n", "─".repeat(70)));

    for service in get_services() {
        output.push_str(&format!(
            "{:<20} {:<25} {}\n",
            service.id, service.name, service.uuid
        ));
    }

    Ok(output.trim_end().to_string())
}
