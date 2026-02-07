//! ADI Indexer Plugin (v3 ABI)
//!
//! Provides MCP tools and resources for code indexing and semantic search.
//!
//! This plugin requires the adi.embed plugin to be installed for embeddings.
//! Install with: `adi plugin install adi.embed`

mod mcp;

use lib_plugin_abi_v3::{
    async_trait,
    mcp::{McpResource, McpResourceContent, McpResources, McpTool, McpToolResult, McpTools},
    Plugin, PluginContext, PluginMetadata, PluginType, Result as PluginResult,
    SERVICE_MCP_RESOURCES, SERVICE_MCP_TOOLS,
};
use once_cell::sync::OnceCell;
use serde_json::Value;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::runtime::Runtime;
use tokio::sync::RwLock;

/// Global tokio runtime for async operations.
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
    indexer: Option<Arc<indexer_core::Adi>>,
}

impl Default for PluginState {
    fn default() -> Self {
        Self {
            project_path: PathBuf::from("."),
            indexer: None,
        }
    }
}

// ============================================================================
// PLUGIN IMPLEMENTATION
// ============================================================================

/// ADI Indexer Plugin
pub struct IndexerPlugin {
    state: Arc<RwLock<PluginState>>,
}

impl IndexerPlugin {
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(PluginState::default())),
        }
    }
}

impl Default for IndexerPlugin {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Plugin for IndexerPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            id: "adi.indexer".to_string(),
            name: "ADI Indexer".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            plugin_type: PluginType::Core,
            author: Some("ADI Team".to_string()),
            description: Some(
                "Code indexer with semantic search and symbol analysis".to_string(),
            ),
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
impl McpTools for IndexerPlugin {
    async fn list_tools(&self) -> Vec<McpTool> {
        mcp::list_tools()
    }

    async fn call_tool(&self, name: &str, arguments: Value) -> PluginResult<McpToolResult> {
        mcp::call_tool(&self.state, name, arguments).await
    }
}

// ============================================================================
// MCP RESOURCES IMPLEMENTATION
// ============================================================================

#[async_trait]
impl McpResources for IndexerPlugin {
    async fn list_resources(&self) -> Vec<McpResource> {
        mcp::list_resources(&self.state).await
    }

    async fn read_resource(&self, uri: &str) -> PluginResult<McpResourceContent> {
        mcp::read_resource(&self.state, uri).await
    }
}

// ============================================================================
// PLUGIN ENTRY POINT
// ============================================================================

#[no_mangle]
pub fn plugin_create() -> Box<dyn Plugin> {
    Box::new(IndexerPlugin::new())
}

/// Create MCP tools service (for service discovery)
#[no_mangle]
pub fn plugin_create_mcp_tools() -> Box<dyn McpTools> {
    Box::new(IndexerPlugin::new())
}

/// Create MCP resources service (for service discovery)
#[no_mangle]
pub fn plugin_create_mcp_resources() -> Box<dyn McpResources> {
    Box::new(IndexerPlugin::new())
}
