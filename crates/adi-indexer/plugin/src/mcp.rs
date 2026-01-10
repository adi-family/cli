//! MCP tools and resources implementation.

use abi_stable::std_types::{RResult, RString, RVec};
use lib_plugin_abi::{PluginContext, ServiceError, ServiceMethod};
use serde_json::{json, Value};

use crate::{PluginState, RUNTIME};

/// Invoke an MCP tool.
pub fn invoke_tool(
    ctx: *mut PluginContext,
    method: &str,
    args: &str,
) -> RResult<RString, ServiceError> {
    let runtime = match RUNTIME.get() {
        Some(rt) => rt,
        None => return RResult::RErr(ServiceError::internal("Runtime not initialized")),
    };

    let result = runtime.block_on(async {
        match method {
            "list_tools" => Ok(list_tools_json()),
            "call_tool" => {
                let params: Value = serde_json::from_str(args)
                    .map_err(|e| ServiceError::invocation_error(format!("Invalid args: {}", e)))?;

                let tool_name = params
                    .get("name")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| ServiceError::invocation_error("Missing tool name"))?;

                let tool_args = params.get("args").cloned().unwrap_or(json!({}));

                call_tool_impl(ctx, tool_name, &tool_args).await
            }
            _ => Err(ServiceError::method_not_found(method)),
        }
    });

    match result {
        Ok(s) => RResult::ROk(RString::from(s)),
        Err(e) => RResult::RErr(e),
    }
}

/// Invoke an MCP resource method.
pub fn invoke_resource(
    ctx: *mut PluginContext,
    method: &str,
    args: &str,
) -> RResult<RString, ServiceError> {
    let runtime = match RUNTIME.get() {
        Some(rt) => rt,
        None => return RResult::RErr(ServiceError::internal("Runtime not initialized")),
    };

    let result = runtime.block_on(async {
        match method {
            "list_resources" => Ok(list_resources_json(ctx)),
            "read_resource" => {
                let params: Value = serde_json::from_str(args)
                    .map_err(|e| ServiceError::invocation_error(format!("Invalid args: {}", e)))?;

                let uri = params
                    .get("uri")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| ServiceError::invocation_error("Missing uri"))?;

                read_resource_impl(ctx, uri).await
            }
            _ => Err(ServiceError::method_not_found(method)),
        }
    });

    match result {
        Ok(s) => RResult::ROk(RString::from(s)),
        Err(e) => RResult::RErr(e),
    }
}

/// List available MCP tools.
pub fn list_tools() -> RVec<ServiceMethod> {
    let tools = vec![
        ServiceMethod::new("list_tools").with_description("List all available tools"),
        ServiceMethod::new("call_tool")
            .with_description("Call a tool by name with arguments")
            .with_parameters_schema(r#"{"type":"object","properties":{"name":{"type":"string"},"args":{"type":"object"}},"required":["name"]}"#),
    ];
    tools.into_iter().collect()
}

/// List available MCP resources.
pub fn list_resources() -> RVec<ServiceMethod> {
    let methods = vec![
        ServiceMethod::new("list_resources").with_description("List all available resources"),
        ServiceMethod::new("read_resource")
            .with_description("Read a resource by URI")
            .with_parameters_schema(
                r#"{"type":"object","properties":{"uri":{"type":"string"}},"required":["uri"]}"#,
            ),
    ];
    methods.into_iter().collect()
}

/// Get tools list as JSON.
fn list_tools_json() -> String {
    let tools = json!([
        {
            "name": "search",
            "description": "Semantic search for code symbols using natural language. Returns symbols ranked by relevance.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "Natural language search query" },
                    "limit": { "type": "integer", "default": 10, "minimum": 1, "maximum": 100 }
                },
                "required": ["query"]
            }
        },
        {
            "name": "search_symbols",
            "description": "Full-text search for symbols by name.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": { "type": "string" },
                    "limit": { "type": "integer", "default": 10 }
                },
                "required": ["query"]
            }
        },
        {
            "name": "search_files",
            "description": "Full-text search for files by path or name.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": { "type": "string" },
                    "limit": { "type": "integer", "default": 10 }
                },
                "required": ["query"]
            }
        },
        {
            "name": "get_symbol",
            "description": "Get detailed information about a specific symbol by its ID.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "id": { "type": "integer", "description": "Symbol ID" }
                },
                "required": ["id"]
            }
        },
        {
            "name": "get_file",
            "description": "Get file information including all symbols defined in it.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "File path relative to project root" }
                },
                "required": ["path"]
            }
        },
        {
            "name": "get_callers",
            "description": "Find all symbols that call/reference a given symbol.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "id": { "type": "integer", "description": "Symbol ID to find callers for" }
                },
                "required": ["id"]
            }
        },
        {
            "name": "get_callees",
            "description": "Find all symbols that a given symbol calls/references.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "id": { "type": "integer", "description": "Symbol ID to find callees for" }
                },
                "required": ["id"]
            }
        },
        {
            "name": "get_symbol_usage",
            "description": "Get complete usage statistics for a symbol.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "id": { "type": "integer", "description": "Symbol ID" }
                },
                "required": ["id"]
            }
        },
        {
            "name": "get_tree",
            "description": "Get the complete project structure as a hierarchical tree.",
            "inputSchema": { "type": "object", "properties": {} }
        },
        {
            "name": "index",
            "description": "Index or re-index the project.",
            "inputSchema": { "type": "object", "properties": {} }
        },
        {
            "name": "status",
            "description": "Get current indexing status.",
            "inputSchema": { "type": "object", "properties": {} }
        }
    ]);
    serde_json::to_string(&tools).unwrap_or_else(|_| "[]".to_string())
}

/// Get resources list as JSON.
fn list_resources_json(ctx: *mut PluginContext) -> String {
    let mut resources = vec![
        json!({
            "uri": "adi://status",
            "name": "Index Status",
            "description": "Current indexing status and statistics",
            "mimeType": "application/json"
        }),
        json!({
            "uri": "adi://tree",
            "name": "Project Tree",
            "description": "Hierarchical view of all indexed files and symbols",
            "mimeType": "application/json"
        }),
        json!({
            "uri": "adi://config",
            "name": "Configuration",
            "description": "Current ADI configuration",
            "mimeType": "application/json"
        }),
    ];

    // Add indexed files as resources
    unsafe {
        if let Some(state) = (*(ctx as *const PluginContext)).user_data::<PluginState>() {
            if let Some(indexer) = &state.indexer {
                if let Ok(tree) = indexer.get_tree() {
                    for file_node in tree.files.iter().take(100) {
                        let path_str = file_node.path.to_string_lossy();
                        resources.push(json!({
                            "uri": format!("adi://file/{}", path_str),
                            "name": file_node.path.file_name()
                                .map(|n| n.to_string_lossy().to_string())
                                .unwrap_or_else(|| path_str.to_string()),
                            "description": format!("{} file with {} symbols",
                                file_node.language.as_str(),
                                file_node.symbols.len()
                            ),
                            "mimeType": "application/json"
                        }));
                    }
                }
            }
        }
    }

    serde_json::to_string(&resources).unwrap_or_else(|_| "[]".to_string())
}

/// Call a tool implementation.
async fn call_tool_impl(
    ctx: *mut PluginContext,
    tool_name: &str,
    args: &Value,
) -> Result<String, ServiceError> {
    let indexer = unsafe {
        let state = (*(ctx as *const PluginContext)).user_data::<PluginState>();
        match state.and_then(|s| s.indexer.clone()) {
            Some(i) => i,
            None => {
                return Err(ServiceError::invocation_error(
                    "Indexer not initialized. Set project path first.",
                ))
            }
        }
    };

    match tool_name {
        "search" => {
            let query = args.get("query").and_then(|v| v.as_str()).unwrap_or("");
            let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(10) as usize;
            let limit = limit.clamp(1, 100);

            indexer
                .search(query, limit)
                .await
                .map(|results| {
                    tool_result(&serde_json::to_string_pretty(&results).unwrap_or_default())
                })
                .map_err(|e| ServiceError::invocation_error(e.to_string()))
        }
        "search_symbols" => {
            let query = args.get("query").and_then(|v| v.as_str()).unwrap_or("");
            let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(10) as usize;

            indexer
                .search_symbols(query, limit)
                .await
                .map(|results| {
                    tool_result(&serde_json::to_string_pretty(&results).unwrap_or_default())
                })
                .map_err(|e| ServiceError::invocation_error(e.to_string()))
        }
        "search_files" => {
            let query = args.get("query").and_then(|v| v.as_str()).unwrap_or("");
            let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(10) as usize;

            indexer
                .search_files(query, limit)
                .await
                .map(|results| {
                    tool_result(&serde_json::to_string_pretty(&results).unwrap_or_default())
                })
                .map_err(|e| ServiceError::invocation_error(e.to_string()))
        }
        "get_symbol" => {
            let id = args
                .get("id")
                .and_then(|v| v.as_i64())
                .ok_or_else(|| ServiceError::invocation_error("Missing symbol id"))?;

            indexer
                .get_symbol(adi_indexer_core::SymbolId(id))
                .map(|symbol| {
                    tool_result(&serde_json::to_string_pretty(&symbol).unwrap_or_default())
                })
                .map_err(|e| ServiceError::invocation_error(e.to_string()))
        }
        "get_file" => {
            let path = args
                .get("path")
                .and_then(|v| v.as_str())
                .ok_or_else(|| ServiceError::invocation_error("Missing file path"))?;

            indexer
                .get_file(std::path::Path::new(path))
                .map(|info| tool_result(&serde_json::to_string_pretty(&info).unwrap_or_default()))
                .map_err(|e| ServiceError::invocation_error(e.to_string()))
        }
        "get_callers" => {
            let id = args
                .get("id")
                .and_then(|v| v.as_i64())
                .ok_or_else(|| ServiceError::invocation_error("Missing symbol id"))?;

            indexer
                .get_callers(adi_indexer_core::SymbolId(id))
                .map(|callers| {
                    tool_result(&serde_json::to_string_pretty(&callers).unwrap_or_default())
                })
                .map_err(|e| ServiceError::invocation_error(e.to_string()))
        }
        "get_callees" => {
            let id = args
                .get("id")
                .and_then(|v| v.as_i64())
                .ok_or_else(|| ServiceError::invocation_error("Missing symbol id"))?;

            indexer
                .get_callees(adi_indexer_core::SymbolId(id))
                .map(|callees| {
                    tool_result(&serde_json::to_string_pretty(&callees).unwrap_or_default())
                })
                .map_err(|e| ServiceError::invocation_error(e.to_string()))
        }
        "get_symbol_usage" => {
            let id = args
                .get("id")
                .and_then(|v| v.as_i64())
                .ok_or_else(|| ServiceError::invocation_error("Missing symbol id"))?;

            indexer
                .get_symbol_usage(adi_indexer_core::SymbolId(id))
                .map(|usage| tool_result(&serde_json::to_string_pretty(&usage).unwrap_or_default()))
                .map_err(|e| ServiceError::invocation_error(e.to_string()))
        }
        "get_tree" => indexer
            .get_tree()
            .map(|tree| tool_result(&serde_json::to_string_pretty(&tree).unwrap_or_default()))
            .map_err(|e| ServiceError::invocation_error(e.to_string())),
        "index" => indexer
            .index()
            .await
            .map(|progress| {
                tool_result(&format!(
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
            .map_err(|e| ServiceError::invocation_error(e.to_string())),
        "status" => indexer
            .status()
            .map(|status| tool_result(&serde_json::to_string_pretty(&status).unwrap_or_default()))
            .map_err(|e| ServiceError::invocation_error(e.to_string())),
        _ => Err(ServiceError::invocation_error(format!(
            "Unknown tool: {}",
            tool_name
        ))),
    }
}

/// Read a resource.
async fn read_resource_impl(ctx: *mut PluginContext, uri: &str) -> Result<String, ServiceError> {
    let indexer = unsafe {
        let state = (*(ctx as *const PluginContext)).user_data::<PluginState>();
        match state.and_then(|s| s.indexer.clone()) {
            Some(i) => i,
            None => {
                return Err(ServiceError::invocation_error(
                    "Indexer not initialized. Set project path first.",
                ))
            }
        }
    };

    let content = match uri {
        "adi://status" => {
            let status = indexer
                .status()
                .map_err(|e| ServiceError::invocation_error(e.to_string()))?;
            json!({
                "uri": uri,
                "mimeType": "application/json",
                "text": serde_json::to_string_pretty(&status).unwrap_or_default()
            })
        }
        "adi://tree" => {
            let tree = indexer
                .get_tree()
                .map_err(|e| ServiceError::invocation_error(e.to_string()))?;
            json!({
                "uri": uri,
                "mimeType": "application/json",
                "text": serde_json::to_string_pretty(&tree).unwrap_or_default()
            })
        }
        "adi://config" => {
            let config = indexer.config();
            json!({
                "uri": uri,
                "mimeType": "application/json",
                "text": serde_json::to_string_pretty(&config).unwrap_or_default()
            })
        }
        _ if uri.starts_with("adi://file/") => {
            let path = uri.strip_prefix("adi://file/").unwrap();
            let file_info = indexer
                .get_file(std::path::Path::new(path))
                .map_err(|e| ServiceError::invocation_error(e.to_string()))?;

            // Read actual file content
            let full_path = indexer.project_path().join(path);
            let file_content = std::fs::read_to_string(&full_path).ok();

            let content_text = if let Some(content) = file_content {
                json!({
                    "file": file_info.file,
                    "symbols": file_info.symbols,
                    "content": content
                })
                .to_string()
            } else {
                serde_json::to_string_pretty(&file_info).unwrap_or_default()
            };

            json!({
                "uri": uri,
                "mimeType": "application/json",
                "text": content_text
            })
        }
        _ if uri.starts_with("adi://symbol/") => {
            let id_str = uri.strip_prefix("adi://symbol/").unwrap();
            let id: i64 = id_str
                .parse()
                .map_err(|_| ServiceError::invocation_error("Invalid symbol ID"))?;

            let symbol = indexer
                .get_symbol(adi_indexer_core::SymbolId(id))
                .map_err(|e| ServiceError::invocation_error(e.to_string()))?;
            let usage = indexer
                .get_symbol_usage(adi_indexer_core::SymbolId(id))
                .ok();

            let content_obj = json!({
                "symbol": symbol,
                "usage": usage
            });

            json!({
                "uri": uri,
                "mimeType": "application/json",
                "text": serde_json::to_string_pretty(&content_obj).unwrap_or_default()
            })
        }
        _ => {
            return Err(ServiceError::invocation_error(format!(
                "Unknown resource URI: {}",
                uri
            )))
        }
    };

    Ok(serde_json::to_string(&content).unwrap_or_else(|_| "{}".to_string()))
}

/// Format a tool result.
fn tool_result(text: &str) -> String {
    let result = json!({
        "content": [{
            "type": "text",
            "text": text
        }]
    });
    serde_json::to_string(&result).unwrap_or_else(|_| "{}".to_string())
}
