//! ADI Knowledgebase Plugin (v3 ABI)
//!
//! Provides MCP tools and resources for knowledge graph with semantic embeddings.

use knowledgebase_core::{Knowledgebase, NodeType};
use lib_plugin_abi_v3::{
    async_trait,
    mcp::{McpResource, McpResourceContent, McpResources, McpTool, McpToolResult, McpTools},
    Plugin, PluginContext, PluginError, PluginMetadata, PluginType, Result as PluginResult,
    SERVICE_MCP_RESOURCES, SERVICE_MCP_TOOLS,
};
use once_cell::sync::OnceCell;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::runtime::Runtime;
use tokio::sync::RwLock;

/// Global tokio runtime for async operations
static RUNTIME: OnceCell<Runtime> = OnceCell::new();

fn get_runtime() -> &'static Runtime {
    RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("Failed to create tokio runtime")
    })
}

/// Plugin state
struct PluginState {
    project_path: PathBuf,
    kb: Option<Arc<Knowledgebase>>,
}

impl Default for PluginState {
    fn default() -> Self {
        Self {
            project_path: PathBuf::from("."),
            kb: None,
        }
    }
}

// ============================================================================
// PLUGIN IMPLEMENTATION
// ============================================================================

/// ADI Knowledgebase Plugin
pub struct KnowledgebasePlugin {
    state: Arc<RwLock<PluginState>>,
}

impl KnowledgebasePlugin {
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(PluginState::default())),
        }
    }
}

impl Default for KnowledgebasePlugin {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Plugin for KnowledgebasePlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            id: "adi.knowledgebase".to_string(),
            name: "ADI Knowledgebase".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            plugin_type: PluginType::Core,
            author: Some("ADI Team".to_string()),
            description: Some("Knowledge graph with semantic embeddings".to_string()),
            category: None,
        }
    }

    async fn init(&mut self, _ctx: &PluginContext) -> PluginResult<()> {
        // Initialize tokio runtime
        let _ = get_runtime();
        Ok(())
    }

    async fn shutdown(&self) -> PluginResult<()> {
        Ok(())
    }

    fn provides(&self) -> Vec<&'static str> {
        vec![SERVICE_MCP_TOOLS, SERVICE_MCP_RESOURCES]
    }
}

// ============================================================================
// MCP TOOLS IMPLEMENTATION
// ============================================================================

#[async_trait]
impl McpTools for KnowledgebasePlugin {
    async fn list_tools(&self) -> Vec<McpTool> {
        vec![
            McpTool {
                name: "kb_search".to_string(),
                description: "Semantic search in the knowledgebase".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "query": { "type": "string", "description": "Search query" },
                        "limit": { "type": "integer", "default": 10 }
                    },
                    "required": ["query"]
                }),
            },
            McpTool {
                name: "kb_add".to_string(),
                description: "Add a document to the knowledgebase".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "content": { "type": "string", "description": "Document content" },
                        "type": {
                            "type": "string",
                            "description": "Node type",
                            "enum": ["fact", "decision", "error", "guide", "glossary", "context", "assumption"],
                            "default": "fact"
                        },
                        "metadata": { "type": "object", "description": "Document metadata" }
                    },
                    "required": ["content"]
                }),
            },
            McpTool {
                name: "kb_status".to_string(),
                description: "Get knowledgebase status".to_string(),
                input_schema: json!({ "type": "object", "properties": {} }),
            },
            McpTool {
                name: "set_project_path".to_string(),
                description: "Set the project path for the knowledgebase".to_string(),
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

    async fn call_tool(&self, name: &str, arguments: Value) -> PluginResult<McpToolResult> {
        match name {
            "set_project_path" => {
                let path = arguments
                    .get("path")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| PluginError::CommandFailed("Missing path".to_string()))?;

                let mut state = self.state.write().await;
                state.project_path = PathBuf::from(path);

                // Get the plugin manager from thread-local storage for plugin support
                let plugin_manager = lib_plugin_host::current_plugin_manager().ok_or_else(|| {
                    PluginError::CommandFailed("Plugin manager not available".to_string())
                })?;

                // Initialize the knowledgebase for the new path
                match Knowledgebase::open_with_plugins(&state.project_path, plugin_manager).await {
                    Ok(kb) => {
                        state.kb = Some(Arc::new(kb));
                        Ok(McpToolResult::text("ok"))
                    }
                    Err(e) => Err(PluginError::CommandFailed(format!(
                        "Failed to open knowledgebase: {}",
                        e
                    ))),
                }
            }
            _ => {
                // All other tools require an initialized knowledgebase
                let state = self.state.read().await;
                let kb = state.kb.as_ref().ok_or_else(|| {
                    PluginError::CommandFailed(
                        "Knowledgebase not initialized. Call set_project_path first.".to_string(),
                    )
                })?;

                self.call_tool_impl(kb, name, &arguments).await
            }
        }
    }
}

impl KnowledgebasePlugin {
    async fn call_tool_impl(
        &self,
        kb: &Arc<Knowledgebase>,
        tool_name: &str,
        args: &Value,
    ) -> PluginResult<McpToolResult> {
        match tool_name {
            "kb_search" => {
                let query = args
                    .get("query")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| PluginError::CommandFailed("Missing query".to_string()))?;

                let results = kb.query(query).await.map_err(|e| {
                    PluginError::CommandFailed(format!("Search failed: {}", e))
                })?;

                McpToolResult::json(&results).map_err(|e| {
                    PluginError::CommandFailed(format!("Failed to serialize results: {}", e))
                })
            }
            "kb_add" => {
                let content = args
                    .get("content")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| PluginError::CommandFailed("Missing content".to_string()))?;

                let node_type = match args.get("type").and_then(|v| v.as_str()).unwrap_or("fact") {
                    "decision" => NodeType::Decision,
                    "fact" => NodeType::Fact,
                    "error" => NodeType::Error,
                    "guide" => NodeType::Guide,
                    "glossary" => NodeType::Glossary,
                    "context" => NodeType::Context,
                    "assumption" => NodeType::Assumption,
                    _ => NodeType::Fact,
                };

                let node = kb.add_from_user(content, content, node_type).await.map_err(|e| {
                    PluginError::CommandFailed(format!("Failed to add node: {}", e))
                })?;

                Ok(McpToolResult::text(format!("Added node with ID: {}", node.id)))
            }
            "kb_status" => {
                let data_dir = kb.data_dir();
                let status = json!({
                    "data_dir": data_dir.to_string_lossy(),
                    "status": "ready"
                });
                McpToolResult::json(&status).map_err(|e| {
                    PluginError::CommandFailed(format!("Failed to serialize status: {}", e))
                })
            }
            _ => Err(PluginError::CommandFailed(format!(
                "Unknown tool: {}",
                tool_name
            ))),
        }
    }
}

// ============================================================================
// MCP RESOURCES IMPLEMENTATION
// ============================================================================

#[async_trait]
impl McpResources for KnowledgebasePlugin {
    async fn list_resources(&self) -> Vec<McpResource> {
        vec![McpResource {
            uri: "kb://status".to_string(),
            name: "KB Status".to_string(),
            description: "Knowledgebase status and statistics".to_string(),
            mime_type: "application/json".to_string(),
        }]
    }

    async fn read_resource(&self, uri: &str) -> PluginResult<McpResourceContent> {
        let state = self.state.read().await;
        let kb = state.kb.as_ref().ok_or_else(|| {
            PluginError::CommandFailed(
                "Knowledgebase not initialized. Call set_project_path first.".to_string(),
            )
        })?;

        match uri {
            "kb://status" => {
                let data_dir = kb.data_dir();
                let status = json!({
                    "data_dir": data_dir.to_string_lossy(),
                    "status": "ready"
                });
                let content = serde_json::to_string_pretty(&status)?;
                Ok(McpResourceContent::text(uri, content, "application/json"))
            }
            _ => Err(PluginError::CommandFailed(format!(
                "Unknown resource URI: {}",
                uri
            ))),
        }
    }
}

// ============================================================================
// PLUGIN ENTRY POINTS
// ============================================================================

/// Create the plugin instance (v3 entry point)
#[no_mangle]
pub fn plugin_create() -> Box<dyn Plugin> {
    Box::new(KnowledgebasePlugin::new())
}

/// Create MCP tools service (for service discovery)
#[no_mangle]
pub fn plugin_create_mcp_tools() -> Box<dyn McpTools> {
    Box::new(KnowledgebasePlugin::new())
}

/// Create MCP resources service (for service discovery)
#[no_mangle]
pub fn plugin_create_mcp_resources() -> Box<dyn McpResources> {
    Box::new(KnowledgebasePlugin::new())
}
