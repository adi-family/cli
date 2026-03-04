//! Embedding plugin trait
//!
//! Embedders generate vector embeddings for text content, used for
//! semantic search and similarity matching in the indexer and knowledgebase.

use crate::{Plugin, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Embedding plugin trait
///
/// Embedders provide text-to-vector conversion for semantic search.
/// They can use local models (like ONNX) or remote APIs (like OpenAI).
#[async_trait]
pub trait Embedder: Plugin {
    /// Get embedder metadata
    fn embedder_info(&self) -> EmbedderInfo;

    /// Generate embeddings for texts
    ///
    /// # Arguments
    /// * `texts` - Text strings to embed
    ///
    /// # Returns
    /// A list of embedding vectors, one per input text
    async fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>>;
}

/// Embedder metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbedderInfo {
    /// Model name (e.g., "all-MiniLM-L6-v2", "text-embedding-3-small")
    pub model_name: String,

    /// Embedding dimensions (e.g., 384, 1536)
    pub dimensions: u32,

    /// Provider name (e.g., "fastembed", "openai", "uzu")
    pub provider: String,
}

impl EmbedderInfo {
    /// Create new embedder info
    pub fn new(
        model_name: impl Into<String>,
        dimensions: u32,
        provider: impl Into<String>,
    ) -> Self {
        Self {
            model_name: model_name.into(),
            dimensions,
            provider: provider.into(),
        }
    }
}
