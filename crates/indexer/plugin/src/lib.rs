//! ADI Indexer Plugin (v3 ABI)
//!
//! Provides tools and resources for code indexing and semantic search.
//!
//! This plugin requires the adi.embed plugin to be installed for embeddings.
//! Install with: `adi plugin install adi.embed`

use lib_plugin_abi_v3::{
    async_trait,
    Plugin, PluginContext, PluginMetadata, PluginType, Result as PluginResult,
};
use once_cell::sync::OnceCell;
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
        vec![]
    }
}

// ============================================================================
// PLUGIN ENTRY POINT
// ============================================================================

#[no_mangle]
pub fn plugin_create() -> Box<dyn Plugin> {
    Box::new(IndexerPlugin::new())
}
