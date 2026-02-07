//! ADI Embed Plugin
//!
//! Provides text embedding services using fastembed/ONNX for local ML inference.
//! Other plugins can use the adi.embed service for generating embeddings.

use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use lib_plugin_abi_v3::{
    async_trait, Plugin, PluginContext, PluginMetadata, PluginType, Result as PluginResult,
};
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;

/// Global embedder instance
static EMBEDDER: OnceCell<Mutex<TextEmbedding>> = OnceCell::new();

/// Embedding model configuration
const MODEL_NAME: &str = "jinaai/jina-embeddings-v2-base-code";
const DIMENSIONS: u32 = 768;

// === Request/Response Types ===

#[derive(Deserialize)]
struct EmbedRequest {
    texts: Vec<String>,
}

#[derive(Serialize)]
struct EmbedResponse {
    embeddings: Vec<Vec<f32>>,
}

#[derive(Serialize)]
struct DimensionsResponse {
    dimensions: u32,
}

#[derive(Serialize)]
struct ModelInfoResponse {
    model_name: String,
    dimensions: u32,
    provider: String,
}

/// Embed Plugin
pub struct EmbedPlugin;

impl EmbedPlugin {
    pub fn new() -> Self {
        Self
    }
}

impl Default for EmbedPlugin {
    fn default() -> Self {
        Self::new()
    }
}

fn get_cache_dir() -> Option<std::path::PathBuf> {
    directories::ProjectDirs::from("com", "adi", "adi").map(|dirs| dirs.cache_dir().join("models"))
}

#[async_trait]
impl Plugin for EmbedPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            id: "adi.embed".to_string(),
            name: "ADI Embed".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            plugin_type: PluginType::Core,
            author: Some("ADI Team".to_string()),
            description: Some("Text embedding service using fastembed/ONNX".to_string()),
            category: None,
        }
    }

    async fn init(&mut self, _ctx: &PluginContext) -> PluginResult<()> {
        // Initialize the embedder
        let cache_dir = get_cache_dir();

        let mut init_options = InitOptions::new(EmbeddingModel::JinaEmbeddingsV2BaseCode);

        if let Some(cache) = cache_dir {
            let _ = std::fs::create_dir_all(&cache);
            init_options = init_options.with_cache_dir(cache);
        }

        match TextEmbedding::try_new(init_options) {
            Ok(model) => {
                let _ = EMBEDDER.set(Mutex::new(model));
                Ok(())
            }
            Err(e) => Err(lib_plugin_abi_v3::PluginError::InitFailed(format!(
                "Failed to initialize embedding model: {}",
                e
            ))),
        }
    }

    async fn shutdown(&self) -> PluginResult<()> {
        Ok(())
    }

    fn provides(&self) -> Vec<&'static str> {
        // This plugin provides an embedding service, not CLI commands
        vec![]
    }
}

// === Public API for embedding ===

impl EmbedPlugin {
    /// Generate embeddings for the given texts
    pub fn embed(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>, String> {
        if texts.is_empty() {
            return Ok(vec![]);
        }

        let embedder = EMBEDDER
            .get()
            .ok_or_else(|| "Embedder not initialized".to_string())?;

        let model = embedder
            .lock()
            .map_err(|e| format!("Lock error: {}", e))?;

        model
            .embed(texts, None)
            .map_err(|e| format!("Embedding error: {}", e))
    }

    /// Get the embedding dimensions
    pub fn dimensions(&self) -> u32 {
        DIMENSIONS
    }

    /// Get model information
    pub fn model_info(&self) -> ModelInfoResponse {
        ModelInfoResponse {
            model_name: MODEL_NAME.to_string(),
            dimensions: DIMENSIONS,
            provider: "fastembed".to_string(),
        }
    }
}

/// Create the plugin instance (v3 entry point)
#[no_mangle]
pub fn plugin_create() -> Box<dyn Plugin> {
    Box::new(EmbedPlugin::new())
}
