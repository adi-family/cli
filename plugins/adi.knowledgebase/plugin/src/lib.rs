//! ADI Knowledgebase Plugin (v3 ABI)
//!
//! Provides tools and resources for knowledge graph with semantic embeddings.

use knowledgebase_core::Knowledgebase;
use lib_plugin_abi_v3::{
    async_trait,
    Plugin, PluginContext, PluginMetadata, PluginType, Result as PluginResult,
};
use once_cell::sync::OnceCell;
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
        vec![]
    }
}

// ============================================================================
// PLUGIN ENTRY POINT
// ============================================================================

/// Create the plugin instance (v3 entry point)
#[no_mangle]
pub fn plugin_create() -> Box<dyn Plugin> {
    Box::new(KnowledgebasePlugin::new())
}
