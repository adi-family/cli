//! MCP tools and resources implementation for v3 ABI.

use adi_indexer_core::SymbolId;
use lib_plugin_abi_v3::{
    mcp::{McpResource, McpResourceContent, McpTool, McpToolResult},
    PluginError, Result as PluginResult,
};
use serde_json::{json, Value};
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::PluginState;

// ============================================================================
// MCP TOOLS
// ============================================================================

/// List all available MCP tools
pub fn list_tools() -> Vec<McpTool> {
    vec![
        McpTool {
            name: "search".to_string(),
            description: "Semantic search for code symbols using natural language. Returns symbols ranked by relevance.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "Natural language search query" },
                    "limit": { "type": "integer", "default": 10, "minimum": 1, "maximum": 100 }
                },
                "required": ["query"]
            }),
        },
        McpTool {
            name: "search_symbols".to_string(),
            description: "Full-text search for symbols by name.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "query": { "type": "string" },
                    "limit": { "type": "integer", "default": 10 }
                },
                "required": ["query"]
            }),
        },
        McpTool {
            name: "search_files".to_string(),
            description: "Full-text search for files by path or name.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "query": { "type": "string" },
                    "limit": { "type": "integer", "default": 10 }
                },
                "required": ["query"]
            }),
        },
        McpTool {
            name: "get_symbol".to_string(),
            description: "Get detailed information about a specific symbol by its ID.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "id": { "type": "integer", "description": "Symbol ID" }
                },
                "required": ["id"]
            }),
        },
        McpTool {
            name: "get_file".to_string(),
            description: "Get file information including all symbols defined in it.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "File path relative to project root" }
                },
                "required": ["path"]
            }),
        },
        McpTool {
            name: "get_callers".to_string(),
            description: "Find all symbols that call/reference a given symbol.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "id": { "type": "integer", "description": "Symbol ID to find callers for" }
                },
                "required": ["id"]
            }),
        },
        McpTool {
            name: "get_callees".to_string(),
            description: "Find all symbols that a given symbol calls/references.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "id": { "type": "integer", "description": "Symbol ID to find callees for" }
                },
                "required": ["id"]
            }),
        },
        McpTool {
            name: "get_symbol_usage".to_string(),
            description: "Get complete usage statistics for a symbol.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "id": { "type": "integer", "description": "Symbol ID" }
                },
                "required": ["id"]
            }),
        },
        McpTool {
            name: "get_tree".to_string(),
            description: "Get the complete project structure as a hierarchical tree.".to_string(),
            input_schema: json!({ "type": "object", "properties": {} }),
        },
        McpTool {
            name: "index".to_string(),
            description: "Index or re-index the project.".to_string(),
            input_schema: json!({ "type": "object", "properties": {} }),
        },
        McpTool {
            name: "status".to_string(),
            description: "Get current indexing status.".to_string(),
            input_schema: json!({ "type": "object", "properties": {} }),
        },
        McpTool {
            name: "set_project_path".to_string(),
            description: "Set the project path for indexing.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Path to the project root" }
                },
                "required": ["path"]
            }),
        },
    ]
}

/// Call an MCP tool
pub async fn call_tool(
    state: &Arc<RwLock<PluginState>>,
    name: &str,
    args: Value,
) -> PluginResult<McpToolResult> {
    match name {
        "set_project_path" => {
            let path = args
                .get("path")
                .and_then(|v| v.as_str())
                .ok_or_else(|| PluginError::CommandFailed("Missing path".to_string()))?;

            let mut state = state.write().await;
            state.project_path = std::path::PathBuf::from(path);

            // Initialize the indexer for the new path
            // Get the plugin manager from thread-local storage for plugin support
            let plugin_manager = match lib_plugin_host::current_plugin_manager() {
                Some(pm) => pm,
                None => {
                    return Err(PluginError::CommandFailed(
                        "Plugin manager not available".to_string(),
                    ));
                }
            };

            match adi_indexer_core::Adi::open_with_plugins(
                state.project_path.as_path(),
                plugin_manager,
            )
            .await
            {
                Ok(adi) => {
                    state.indexer = Some(Arc::new(adi));
                    Ok(McpToolResult::text("ok"))
                }
                Err(e) => Err(PluginError::CommandFailed(format!(
                    "Failed to open indexer: {}",
                    e
                ))),
            }
        }
        _ => {
            // All other tools require an initialized indexer
            let state = state.read().await;
            let indexer = state
                .indexer
                .as_ref()
                .ok_or_else(|| {
                    PluginError::CommandFailed(
                        "Indexer not initialized. Call set_project_path first.".to_string(),
                    )
                })?
                .clone();

            call_tool_impl(&indexer, name, &args).await
        }
    }
}

/// Call a tool with an initialized indexer
async fn call_tool_impl(
    indexer: &adi_indexer_core::Adi,
    tool_name: &str,
    args: &Value,
) -> PluginResult<McpToolResult> {
    match tool_name {
        "search" => {
            let query = args.get("query").and_then(|v| v.as_str()).unwrap_or("");
            let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(10) as usize;
            let limit = limit.clamp(1, 100);

            indexer
                .search(query, limit)
                .await
                .map(|results| {
                    McpToolResult::json(&results).unwrap_or_else(|_| McpToolResult::text("[]"))
                })
                .map_err(|e| PluginError::CommandFailed(e.to_string()))
        }
        "search_symbols" => {
            let query = args.get("query").and_then(|v| v.as_str()).unwrap_or("");
            let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(10) as usize;

            indexer
                .search_symbols(query, limit)
                .await
                .map(|results| {
                    McpToolResult::json(&results).unwrap_or_else(|_| McpToolResult::text("[]"))
                })
                .map_err(|e| PluginError::CommandFailed(e.to_string()))
        }
        "search_files" => {
            let query = args.get("query").and_then(|v| v.as_str()).unwrap_or("");
            let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(10) as usize;

            indexer
                .search_files(query, limit)
                .await
                .map(|results| {
                    McpToolResult::json(&results).unwrap_or_else(|_| McpToolResult::text("[]"))
                })
                .map_err(|e| PluginError::CommandFailed(e.to_string()))
        }
        "get_symbol" => {
            let id = args
                .get("id")
                .and_then(|v| v.as_i64())
                .ok_or_else(|| PluginError::CommandFailed("Missing symbol id".to_string()))?;

            indexer
                .get_symbol(SymbolId(id))
                .map(|symbol| {
                    McpToolResult::json(&symbol).unwrap_or_else(|_| McpToolResult::text("{}"))
                })
                .map_err(|e| PluginError::CommandFailed(e.to_string()))
        }
        "get_file" => {
            let path = args
                .get("path")
                .and_then(|v| v.as_str())
                .ok_or_else(|| PluginError::CommandFailed("Missing file path".to_string()))?;

            indexer
                .get_file(Path::new(path))
                .map(|info| {
                    McpToolResult::json(&info).unwrap_or_else(|_| McpToolResult::text("{}"))
                })
                .map_err(|e| PluginError::CommandFailed(e.to_string()))
        }
        "get_callers" => {
            let id = args
                .get("id")
                .and_then(|v| v.as_i64())
                .ok_or_else(|| PluginError::CommandFailed("Missing symbol id".to_string()))?;

            indexer
                .get_callers(SymbolId(id))
                .map(|callers| {
                    McpToolResult::json(&callers).unwrap_or_else(|_| McpToolResult::text("[]"))
                })
                .map_err(|e| PluginError::CommandFailed(e.to_string()))
        }
        "get_callees" => {
            let id = args
                .get("id")
                .and_then(|v| v.as_i64())
                .ok_or_else(|| PluginError::CommandFailed("Missing symbol id".to_string()))?;

            indexer
                .get_callees(SymbolId(id))
                .map(|callees| {
                    McpToolResult::json(&callees).unwrap_or_else(|_| McpToolResult::text("[]"))
                })
                .map_err(|e| PluginError::CommandFailed(e.to_string()))
        }
        "get_symbol_usage" => {
            let id = args
                .get("id")
                .and_then(|v| v.as_i64())
                .ok_or_else(|| PluginError::CommandFailed("Missing symbol id".to_string()))?;

            indexer
                .get_symbol_usage(SymbolId(id))
                .map(|usage| {
                    McpToolResult::json(&usage).unwrap_or_else(|_| McpToolResult::text("{}"))
                })
                .map_err(|e| PluginError::CommandFailed(e.to_string()))
        }
        "get_tree" => indexer
            .get_tree()
            .map(|tree| McpToolResult::json(&tree).unwrap_or_else(|_| McpToolResult::text("{}")))
            .map_err(|e| PluginError::CommandFailed(e.to_string())),
        "index" => indexer
            .index()
            .await
            .map(|progress| {
                McpToolResult::text(format!(
                    "Indexed {} files with {} symbols. Errors: {}",
                    progress.files_processed,
                    progress.symbols_indexed,
                    if progress.errors.is_empty() {
                        "none".to_string()
                    } else {
                        progress.errors.join(", ")
                    }
                ))
            })
            .map_err(|e| PluginError::CommandFailed(e.to_string())),
        "status" => indexer
            .status()
            .map(|status| {
                McpToolResult::json(&status).unwrap_or_else(|_| McpToolResult::text("{}"))
            })
            .map_err(|e| PluginError::CommandFailed(e.to_string())),
        _ => Err(PluginError::CommandFailed(format!(
            "Unknown tool: {}",
            tool_name
        ))),
    }
}

// ============================================================================
// MCP RESOURCES
// ============================================================================

/// List all available MCP resources
pub async fn list_resources(state: &Arc<RwLock<PluginState>>) -> Vec<McpResource> {
    let mut resources = vec![
        McpResource {
            uri: "adi://status".to_string(),
            name: "Index Status".to_string(),
            description: "Current indexing status and statistics".to_string(),
            mime_type: "application/json".to_string(),
        },
        McpResource {
            uri: "adi://tree".to_string(),
            name: "Project Tree".to_string(),
            description: "Hierarchical view of all indexed files and symbols".to_string(),
            mime_type: "application/json".to_string(),
        },
        McpResource {
            uri: "adi://config".to_string(),
            name: "Configuration".to_string(),
            description: "Current ADI configuration".to_string(),
            mime_type: "application/json".to_string(),
        },
    ];

    // Add indexed files as resources
    let state = state.read().await;
    if let Some(indexer) = &state.indexer {
        if let Ok(tree) = indexer.get_tree() {
            for file_node in tree.files.iter().take(100) {
                let path_str = file_node.path.to_string_lossy();
                resources.push(McpResource {
                    uri: format!("adi://file/{}", path_str),
                    name: file_node
                        .path
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_else(|| path_str.to_string()),
                    description: format!(
                        "{} file with {} symbols",
                        file_node.language.as_str(),
                        file_node.symbols.len()
                    ),
                    mime_type: "application/json".to_string(),
                });
            }
        }
    }

    resources
}

/// Read an MCP resource
pub async fn read_resource(
    state: &Arc<RwLock<PluginState>>,
    uri: &str,
) -> PluginResult<McpResourceContent> {
    let state = state.read().await;
    let indexer = state.indexer.as_ref().ok_or_else(|| {
        PluginError::CommandFailed(
            "Indexer not initialized. Call set_project_path first.".to_string(),
        )
    })?;

    match uri {
        "adi://status" => {
            let status = indexer
                .status()
                .map_err(|e| PluginError::CommandFailed(e.to_string()))?;
            let content = serde_json::to_string_pretty(&status)?;
            Ok(McpResourceContent::text(uri, content, "application/json"))
        }
        "adi://tree" => {
            let tree = indexer
                .get_tree()
                .map_err(|e| PluginError::CommandFailed(e.to_string()))?;
            let content = serde_json::to_string_pretty(&tree)?;
            Ok(McpResourceContent::text(uri, content, "application/json"))
        }
        "adi://config" => {
            let config = indexer.config();
            let content = serde_json::to_string_pretty(&config)?;
            Ok(McpResourceContent::text(uri, content, "application/json"))
        }
        _ if uri.starts_with("adi://file/") => {
            let path = uri.strip_prefix("adi://file/").unwrap();
            let file_info = indexer
                .get_file(Path::new(path))
                .map_err(|e| PluginError::CommandFailed(e.to_string()))?;

            // Read actual file content
            let full_path = indexer.project_path().join(path);
            let file_content = std::fs::read_to_string(&full_path).ok();

            let content_obj = if let Some(content) = file_content {
                json!({
                    "file": file_info.file,
                    "symbols": file_info.symbols,
                    "content": content
                })
            } else {
                serde_json::to_value(&file_info)?
            };

            let content = serde_json::to_string_pretty(&content_obj)?;
            Ok(McpResourceContent::text(uri, content, "application/json"))
        }
        _ if uri.starts_with("adi://symbol/") => {
            let id_str = uri.strip_prefix("adi://symbol/").unwrap();
            let id: i64 = id_str
                .parse()
                .map_err(|_| PluginError::CommandFailed("Invalid symbol ID".to_string()))?;

            let symbol = indexer
                .get_symbol(SymbolId(id))
                .map_err(|e| PluginError::CommandFailed(e.to_string()))?;
            let usage = indexer.get_symbol_usage(SymbolId(id)).ok();

            let content_obj = json!({
                "symbol": symbol,
                "usage": usage
            });

            let content = serde_json::to_string_pretty(&content_obj)?;
            Ok(McpResourceContent::text(uri, content, "application/json"))
        }
        _ => Err(PluginError::CommandFailed(format!(
            "Unknown resource URI: {}",
            uri
        ))),
    }
}
