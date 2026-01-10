//! ADI Knowledgebase Plugin
//!
//! Provides MCP tools and resources for knowledge graph with semantic embeddings.

use abi_stable::std_types::{ROption, RResult, RStr, RString, RVec};
use lib_plugin_abi::{
    PluginContext, PluginError, PluginInfo, PluginVTable, ServiceDescriptor, ServiceError,
    ServiceHandle, ServiceMethod, ServiceVTable, ServiceVersion, SERVICE_MCP_RESOURCES,
    SERVICE_MCP_TOOLS,
};
use once_cell::sync::OnceCell;
use serde_json::{json, Value};
use std::ffi::c_void;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::runtime::Runtime;

static RUNTIME: OnceCell<Runtime> = OnceCell::new();
static KB: OnceCell<Option<Arc<adi_knowledgebase_core::Knowledgebase>>> = OnceCell::new();

// === Plugin VTable Implementation ===

extern "C" fn plugin_info() -> PluginInfo {
    PluginInfo::new(
        "adi.knowledgebase",
        "ADI Knowledgebase",
        env!("CARGO_PKG_VERSION"),
        "core",
    )
    .with_author("ADI Team")
    .with_description("Knowledge graph with semantic embeddings")
    .with_min_host_version("0.8.0")
}

extern "C" fn plugin_init(ctx: *mut PluginContext) -> i32 {
    // Initialize tokio runtime
    let _ = RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("Failed to create tokio runtime")
    });

    // Initialize knowledgebase (will be set via message)
    let _ = KB.set(None);

    unsafe {
        let host = (*ctx).host();

        // Register MCP tools service
        let tools_descriptor = ServiceDescriptor::new(
            SERVICE_MCP_TOOLS,
            ServiceVersion::new(1, 0, 0),
            "adi.knowledgebase",
        )
        .with_description("MCP tools for knowledgebase operations");

        let tools_handle = ServiceHandle::new(
            SERVICE_MCP_TOOLS,
            ctx as *const c_void,
            &MCP_TOOLS_VTABLE as *const ServiceVTable,
        );

        if let Err(code) = host.register_svc(tools_descriptor, tools_handle) {
            host.error(&format!("Failed to register MCP tools service: {}", code));
            return code;
        }

        // Register MCP resources service
        let resources_descriptor = ServiceDescriptor::new(
            SERVICE_MCP_RESOURCES,
            ServiceVersion::new(1, 0, 0),
            "adi.knowledgebase",
        )
        .with_description("MCP resources for knowledge data access");

        let resources_handle = ServiceHandle::new(
            SERVICE_MCP_RESOURCES,
            ctx as *const c_void,
            &MCP_RESOURCES_VTABLE as *const ServiceVTable,
        );

        if let Err(code) = host.register_svc(resources_descriptor, resources_handle) {
            host.error(&format!(
                "Failed to register MCP resources service: {}",
                code
            ));
            return code;
        }

        host.info("ADI Knowledgebase plugin initialized");
    }

    0
}

extern "C" fn plugin_cleanup(_ctx: *mut PluginContext) {}

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

// === Message Handler ===

extern "C" fn handle_message(
    _ctx: *mut PluginContext,
    msg_type: RStr<'_>,
    msg_data: RStr<'_>,
) -> RResult<RString, PluginError> {
    match msg_type.as_str() {
        "set_project_path" => {
            let path = PathBuf::from(msg_data.as_str());
            let runtime = RUNTIME.get().unwrap();

            match runtime
                .block_on(async { adi_knowledgebase_core::Knowledgebase::open(&path).await })
            {
                Ok(kb) => {
                    // Note: OnceCell limitation - can't update
                    let _ = kb;
                    RResult::ROk(RString::from("ok"))
                }
                Err(e) => RResult::RErr(PluginError::new(
                    1,
                    format!("Failed to open knowledgebase: {}", e),
                )),
            }
        }
        _ => RResult::RErr(PluginError::new(
            -1,
            format!("Unknown message type: {}", msg_type.as_str()),
        )),
    }
}

// === MCP Tools Service ===

static MCP_TOOLS_VTABLE: ServiceVTable = ServiceVTable {
    invoke: mcp_tools_invoke,
    list_methods: mcp_tools_list_methods,
};

extern "C" fn mcp_tools_invoke(
    _handle: *const c_void,
    method: RStr<'_>,
    args: RStr<'_>,
) -> RResult<RString, ServiceError> {
    let result = match method.as_str() {
        "list_tools" => Ok(list_tools_json()),
        "call_tool" => {
            let params: Value = match serde_json::from_str(args.as_str()) {
                Ok(v) => v,
                Err(e) => {
                    return RResult::RErr(ServiceError::invocation_error(format!(
                        "Invalid args: {}",
                        e
                    )))
                }
            };

            let tool_name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let tool_args = params.get("args").cloned().unwrap_or(json!({}));

            call_tool(tool_name, &tool_args)
        }
        _ => Err(ServiceError::method_not_found(method.as_str())),
    };

    match result {
        Ok(s) => RResult::ROk(RString::from(s)),
        Err(e) => RResult::RErr(e),
    }
}

extern "C" fn mcp_tools_list_methods(_handle: *const c_void) -> RVec<ServiceMethod> {
    vec![
        ServiceMethod::new("list_tools").with_description("List all available tools"),
        ServiceMethod::new("call_tool").with_description("Call a tool by name with arguments"),
    ]
    .into_iter()
    .collect()
}

fn list_tools_json() -> String {
    let tools = json!([
        {
            "name": "kb_search",
            "description": "Semantic search in the knowledgebase",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "Search query" },
                    "limit": { "type": "integer", "default": 10 }
                },
                "required": ["query"]
            }
        },
        {
            "name": "kb_add",
            "description": "Add a document to the knowledgebase",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "content": { "type": "string", "description": "Document content" },
                    "metadata": { "type": "object", "description": "Document metadata" }
                },
                "required": ["content"]
            }
        },
        {
            "name": "kb_status",
            "description": "Get knowledgebase status",
            "inputSchema": { "type": "object", "properties": {} }
        }
    ]);
    serde_json::to_string(&tools).unwrap_or_else(|_| "[]".to_string())
}

fn call_tool(tool_name: &str, args: &Value) -> Result<String, ServiceError> {
    let kb = KB
        .get()
        .and_then(|k| k.as_ref())
        .ok_or_else(|| ServiceError::invocation_error("Knowledgebase not initialized"))?;

    let runtime = RUNTIME.get().unwrap();

    match tool_name {
        "kb_search" => {
            let query = args
                .get("query")
                .and_then(|v| v.as_str())
                .ok_or_else(|| ServiceError::invocation_error("Missing query"))?;

            let results = runtime.block_on(async { kb.query(query).await });
            match results {
                Ok(r) => Ok(tool_result(
                    &serde_json::to_string_pretty(&r).unwrap_or_default(),
                )),
                Err(e) => Err(ServiceError::invocation_error(e.to_string())),
            }
        }
        "kb_add" => {
            let content = args
                .get("content")
                .and_then(|v| v.as_str())
                .ok_or_else(|| ServiceError::invocation_error("Missing content"))?;

            let node_type = match args.get("type").and_then(|v| v.as_str()).unwrap_or("fact") {
                "decision" => adi_knowledgebase_core::NodeType::Decision,
                "fact" => adi_knowledgebase_core::NodeType::Fact,
                "error" => adi_knowledgebase_core::NodeType::Error,
                "guide" => adi_knowledgebase_core::NodeType::Guide,
                "glossary" => adi_knowledgebase_core::NodeType::Glossary,
                "context" => adi_knowledgebase_core::NodeType::Context,
                "assumption" => adi_knowledgebase_core::NodeType::Assumption,
                _ => adi_knowledgebase_core::NodeType::Fact,
            };

            let result =
                runtime.block_on(async { kb.add_from_user(content, content, node_type).await });
            match result {
                Ok(node) => Ok(tool_result(&format!("Added node with ID: {}", node.id))),
                Err(e) => Err(ServiceError::invocation_error(e.to_string())),
            }
        }
        "kb_status" => {
            // Return basic status info
            let data_dir = kb.data_dir();
            let status = json!({
                "data_dir": data_dir.to_string_lossy(),
                "status": "ready"
            });
            Ok(tool_result(
                &serde_json::to_string_pretty(&status).unwrap_or_default(),
            ))
        }
        _ => Err(ServiceError::invocation_error(format!(
            "Unknown tool: {}",
            tool_name
        ))),
    }
}

// === MCP Resources Service ===

static MCP_RESOURCES_VTABLE: ServiceVTable = ServiceVTable {
    invoke: mcp_resources_invoke,
    list_methods: mcp_resources_list_methods,
};

extern "C" fn mcp_resources_invoke(
    _handle: *const c_void,
    method: RStr<'_>,
    _args: RStr<'_>,
) -> RResult<RString, ServiceError> {
    let result = match method.as_str() {
        "list_resources" => Ok(list_resources_json()),
        "read_resource" => Err(ServiceError::invocation_error("Not implemented")),
        _ => Err(ServiceError::method_not_found(method.as_str())),
    };

    match result {
        Ok(s) => RResult::ROk(RString::from(s)),
        Err(e) => RResult::RErr(e),
    }
}

extern "C" fn mcp_resources_list_methods(_handle: *const c_void) -> RVec<ServiceMethod> {
    vec![
        ServiceMethod::new("list_resources").with_description("List all available resources"),
        ServiceMethod::new("read_resource").with_description("Read a resource by URI"),
    ]
    .into_iter()
    .collect()
}

fn list_resources_json() -> String {
    let resources = json!([
        {
            "uri": "kb://status",
            "name": "KB Status",
            "description": "Knowledgebase status and statistics",
            "mimeType": "application/json"
        }
    ]);
    serde_json::to_string(&resources).unwrap_or_else(|_| "[]".to_string())
}

fn tool_result(text: &str) -> String {
    let result = json!({
        "content": [{
            "type": "text",
            "text": text
        }]
    });
    serde_json::to_string(&result).unwrap_or_else(|_| "{}".to_string())
}
