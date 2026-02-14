//! ADI Browser Debug MCP Server
//!
//! Provides MCP tools for browser debugging:
//! - browser_debug_list_tabs: List available debug tabs
//! - browser_debug_get_network: Get network requests from a tab
//! - browser_debug_get_console: Get console logs from a tab

use browser_debug_core::{
    client::BrowserDebugClient, ConsoleFilters, ConsoleLevel, NetworkFilters,
};
use lib_mcp_core::{
    server::{McpRouter, McpServerBuilder},
    transport::stdio::StdioTransport,
    CallToolResult, Tool, ToolInputSchema,
};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

use lib_env_parse::{env_vars, env_or, env_opt};

env_vars! {
    SignalingUrl => "SIGNALING_URL",
    AccessToken => "ACCESS_TOKEN",
}

struct AppState {
    client: Option<BrowserDebugClient>,
    signaling_url: String,
    access_token: String,
}

impl AppState {
    fn new() -> Self {
        let signaling_url = env_or(EnvVar::SignalingUrl.as_str(), "wss://adi.the-ihor.com/api/signaling/ws");
        let access_token = env_opt(EnvVar::AccessToken.as_str()).unwrap_or_default();

        Self {
            client: None,
            signaling_url,
            access_token,
        }
    }

    async fn ensure_connected(&mut self) -> Result<&BrowserDebugClient, String> {
        if self.client.is_none() {
            let client = BrowserDebugClient::connect(&self.signaling_url, &self.access_token)
                .await
                .map_err(|e| format!("Failed to connect: {}", e))?;
            self.client = Some(client);
        }
        Ok(self.client.as_ref().unwrap())
    }
}

#[tokio::main]
async fn main() -> lib_mcp_core::Result<()> {
    // Initialize logging to stderr (MCP uses stdout for protocol)
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("adi_browser_debug=info".parse().unwrap()),
        )
        .with_writer(std::io::stderr)
        .init();

    info!("Starting ADI Browser Debug MCP Server");

    let state = Arc::new(RwLock::new(AppState::new()));
    let server = build_server(state);

    let mut router = McpRouter::new(server);
    router.run(StdioTransport::new()).await
}

fn build_server(
    state: Arc<RwLock<AppState>>,
) -> impl lib_mcp_core::server::McpHandler + Send + Sync + 'static {
    // List tabs tool
    let list_tabs_state = state.clone();
    let list_tabs = move |_args: std::collections::HashMap<String, serde_json::Value>| {
        let state = list_tabs_state.clone();
        async move {
            let mut state_guard = state.write().await;
            let client = state_guard
                .ensure_connected()
                .await
                .map_err(|e| lib_mcp_core::Error::Internal(e))?;

            let tabs = client.list_tabs().await.map_err(|e| {
                lib_mcp_core::Error::Internal(format!("Failed to list tabs: {}", e))
            })?;

            if tabs.is_empty() {
                return Ok(CallToolResult::text(
                    "No browser debug tabs available.\n\n\
                     To debug a browser tab:\n\
                     1. Install the ADI Browser Debugger Chrome extension\n\
                     2. Browse to a page served through an ADI-proxied cocoon\n\
                     3. The extension will automatically detect the debug token",
                ));
            }

            let output = tabs
                .iter()
                .map(|tab| {
                    format!(
                        "Token: {}\nURL: {}\nTitle: {}\nCocoon: {} ({})\n",
                        tab.token,
                        tab.url,
                        tab.title,
                        tab.cocoon_id,
                        tab.cocoon_name.as_deref().unwrap_or("unnamed")
                    )
                })
                .collect::<Vec<_>>()
                .join("\n---\n");

            Ok(CallToolResult::text(format!(
                "Found {} debug tab(s):\n\n{}",
                tabs.len(),
                output
            )))
        }
    };

    // Get network tool
    let get_network_state = state.clone();
    let get_network = move |args: std::collections::HashMap<String, serde_json::Value>| {
        let state = get_network_state.clone();
        async move {
            let token = args
                .get("token")
                .and_then(|v| v.as_str())
                .ok_or_else(|| lib_mcp_core::Error::InvalidParams("token is required".into()))?;

            let filters = NetworkFilters {
                url_pattern: args
                    .get("url_pattern")
                    .and_then(|v| v.as_str().map(String::from)),
                method: args.get("method").and_then(|v| {
                    v.as_array().map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    })
                }),
                status_min: args
                    .get("status_min")
                    .and_then(|v| v.as_u64().map(|n| n as u16)),
                status_max: args
                    .get("status_max")
                    .and_then(|v| v.as_u64().map(|n| n as u16)),
                since: args.get("since").and_then(|v| v.as_i64()),
                limit: args.get("limit").and_then(|v| v.as_u64().map(|n| n as u32)),
            };

            let has_filters = filters.url_pattern.is_some()
                || filters.method.is_some()
                || filters.status_min.is_some()
                || filters.status_max.is_some()
                || filters.since.is_some()
                || filters.limit.is_some();

            let mut state_guard = state.write().await;
            let client = state_guard
                .ensure_connected()
                .await
                .map_err(|e| lib_mcp_core::Error::Internal(e))?;

            let requests = client
                .get_network(token, if has_filters { Some(filters) } else { None })
                .await
                .map_err(|e| {
                    lib_mcp_core::Error::Internal(format!("Failed to get network: {}", e))
                })?;

            if requests.is_empty() {
                return Ok(CallToolResult::text("No network requests captured yet."));
            }

            let output = requests
                .iter()
                .map(|req| {
                    let mut s = format!(
                        "{} {} - {}",
                        req.method,
                        req.url,
                        req.status
                            .map(|s| s.to_string())
                            .unwrap_or_else(|| "pending".to_string())
                    );
                    if let Some(ms) = req.duration_ms {
                        s.push_str(&format!(" ({}ms)", ms));
                    }
                    if let Some(err) = &req.error {
                        s.push_str(&format!(" [ERROR: {}]", err));
                    }
                    s
                })
                .collect::<Vec<_>>()
                .join("\n");

            Ok(CallToolResult::text(format!(
                "Network requests ({}):\n\n{}",
                requests.len(),
                output
            )))
        }
    };

    // Get console tool
    let get_console_state = state.clone();
    let get_console = move |args: std::collections::HashMap<String, serde_json::Value>| {
        let state = get_console_state.clone();
        async move {
            let token = args
                .get("token")
                .and_then(|v| v.as_str())
                .ok_or_else(|| lib_mcp_core::Error::InvalidParams("token is required".into()))?;

            let level_filter: Option<Vec<ConsoleLevel>> = args.get("level").and_then(|v| {
                v.as_array().map(|arr| {
                    arr.iter()
                        .filter_map(|v| match v.as_str()? {
                            "log" => Some(ConsoleLevel::Log),
                            "debug" => Some(ConsoleLevel::Debug),
                            "info" => Some(ConsoleLevel::Info),
                            "warn" => Some(ConsoleLevel::Warn),
                            "error" => Some(ConsoleLevel::Error),
                            _ => None,
                        })
                        .collect()
                })
            });

            let filters = ConsoleFilters {
                level: level_filter,
                message_pattern: args
                    .get("message_pattern")
                    .and_then(|v| v.as_str().map(String::from)),
                since: args.get("since").and_then(|v| v.as_i64()),
                limit: args.get("limit").and_then(|v| v.as_u64().map(|n| n as u32)),
            };

            let has_filters = filters.level.is_some()
                || filters.message_pattern.is_some()
                || filters.since.is_some()
                || filters.limit.is_some();

            let mut state_guard = state.write().await;
            let client = state_guard
                .ensure_connected()
                .await
                .map_err(|e| lib_mcp_core::Error::Internal(e))?;

            let entries = client
                .get_console(token, if has_filters { Some(filters) } else { None })
                .await
                .map_err(|e| {
                    lib_mcp_core::Error::Internal(format!("Failed to get console: {}", e))
                })?;

            if entries.is_empty() {
                return Ok(CallToolResult::text("No console entries captured yet."));
            }

            let output = entries
                .iter()
                .map(|entry| {
                    let level = match entry.level {
                        ConsoleLevel::Log => "LOG",
                        ConsoleLevel::Debug => "DEBUG",
                        ConsoleLevel::Info => "INFO",
                        ConsoleLevel::Warn => "WARN",
                        ConsoleLevel::Error => "ERROR",
                    };
                    let mut s = format!("[{}] {}", level, entry.message);
                    if let Some(source) = &entry.source {
                        s.push_str(&format!(" ({}:{})", source, entry.line.unwrap_or(0)));
                    }
                    s
                })
                .collect::<Vec<_>>()
                .join("\n");

            Ok(CallToolResult::text(format!(
                "Console entries ({}):\n\n{}",
                entries.len(),
                output
            )))
        }
    };

    McpServerBuilder::new("adi-browser-debug", env!("CARGO_PKG_VERSION"))
        .instructions(
            "Browser debugging tools for ADI. Use these to inspect network requests \
             and console logs from browser tabs running ADI-proxied applications.\n\n\
             Workflow:\n\
             1. Call browser_debug_list_tabs to see available debug sessions\n\
             2. Copy the token from a tab you want to inspect\n\
             3. Call browser_debug_get_network or browser_debug_get_console with that token",
        )
        .tool(
            Tool::new("browser_debug_list_tabs", ToolInputSchema::new()).with_description(
                "List all browser tabs with debug tokens available for inspection",
            ),
            list_tabs,
        )
        .tool(
            Tool::new(
                "browser_debug_get_network",
                ToolInputSchema::new()
                    .string_property("token", "Debug token from browser_debug_list_tabs", true)
                    .string_property("url_pattern", "Filter by URL regex pattern", false)
                    .property(
                        "method",
                        serde_json::json!({
                            "type": "array",
                            "items": { "type": "string" },
                            "description": "Filter by HTTP methods, e.g. [\"GET\", \"POST\"]"
                        }),
                        false,
                    )
                    .property(
                        "status_min",
                        serde_json::json!({
                            "type": "integer",
                            "description": "Minimum status code (inclusive)"
                        }),
                        false,
                    )
                    .property(
                        "status_max",
                        serde_json::json!({
                            "type": "integer",
                            "description": "Maximum status code (inclusive)"
                        }),
                        false,
                    )
                    .property(
                        "since",
                        serde_json::json!({
                            "type": "integer",
                            "description": "Only requests after this timestamp (ms)"
                        }),
                        false,
                    )
                    .property(
                        "limit",
                        serde_json::json!({
                            "type": "integer",
                            "description": "Max number of requests to return"
                        }),
                        false,
                    ),
            )
            .with_description("Get network requests from a browser tab"),
            get_network,
        )
        .tool(
            Tool::new(
                "browser_debug_get_console",
                ToolInputSchema::new()
                    .string_property("token", "Debug token from browser_debug_list_tabs", true)
                    .property(
                        "level",
                        serde_json::json!({
                            "type": "array",
                            "items": {
                                "type": "string",
                                "enum": ["log", "debug", "info", "warn", "error"]
                            },
                            "description": "Filter by log levels"
                        }),
                        false,
                    )
                    .string_property("message_pattern", "Filter by message regex pattern", false)
                    .property(
                        "since",
                        serde_json::json!({
                            "type": "integer",
                            "description": "Only entries after this timestamp (ms)"
                        }),
                        false,
                    )
                    .property(
                        "limit",
                        serde_json::json!({
                            "type": "integer",
                            "description": "Max number of entries to return"
                        }),
                        false,
                    ),
            )
            .with_description("Get console logs from a browser tab"),
            get_console,
        )
        .with_logging()
        .build()
}
