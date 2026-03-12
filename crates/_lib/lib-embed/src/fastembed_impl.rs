// Copyright (c) 2024-2025 Ihor
// SPDX-License-Identifier: BSL-1.1
// See LICENSE file for details

//! FastEmbed implementation of the Embedder trait.
//!
//! This module is only compiled when the `fastembed` feature is enabled.
//! For smaller binaries, use the plugin-based embedder instead.

use crate::config::EmbeddingConfig;
use crate::error::{EmbedError, Result};
use crate::Embedder;
use ::fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use lib_cli_common::AdiUserDirs;
use std::sync::Mutex;

/// FastEmbed-based text embedder.
///
/// Uses the fastembed library with jina-embeddings-v2-base-code model
/// optimized for code search and understanding.
pub struct FastEmbedder {
    model: Mutex<TextEmbedding>,
    model_name: String,
    dimensions: u32,
}

impl FastEmbedder {
    /// Create a new FastEmbedder with default configuration.
    pub fn new() -> Result<Self> {
        Self::with_config(&EmbeddingConfig::default())
    }

    /// Create a new FastEmbedder with custom configuration.
    pub fn with_config(config: &EmbeddingConfig) -> Result<Self> {
        let cache_dir = AdiUserDirs::models_dir();

        let mut init_options = InitOptions::new(EmbeddingModel::JinaEmbeddingsV2BaseCode);

        if let Some(cache) = cache_dir {
            std::fs::create_dir_all(&cache)?;
            init_options = init_options.with_cache_dir(cache);
        }

        tracing::info!(
            model = %config.model,
            dimensions = config.dimensions,
            "Initializing FastEmbed model"
        );

        let model = TextEmbedding::try_new(init_options)
            .map_err(|e| EmbedError::Embedding(format!("Failed to load embedding model: {}", e)))?;

        Ok(Self {
            model: Mutex::new(model),
            model_name: config.model.clone(),
            dimensions: config.dimensions,
        })
    }

    /// Create embedder loading user config if available.
    pub fn from_user_config() -> Result<Self> {
        let config = EmbeddingConfig::load_user_config()?;
        Self::with_config(&config)
    }
}

impl Embedder for FastEmbedder {
    fn embed(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        if texts.is_empty() {
            return Ok(vec![]);
        }

        let texts: Vec<String> = texts.iter().map(|s| s.to_string()).collect();

        let model = self
            .model
            .lock()
            .map_err(|e| EmbedError::Embedding(format!("Lock error: {}", e)))?;

        let embeddings = model
            .embed(texts, None)
            .map_err(|e| EmbedError::Embedding(format!("Embedding error: {}", e)))?;

        Ok(embeddings)
    }

    fn dimensions(&self) -> u32 {
        self.dimensions
    }

    fn model_name(&self) -> &str {
        &self.model_name
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = EmbeddingConfig::default();
        assert_eq!(config.provider, "fastembed");
        assert_eq!(config.model, "jinaai/jina-embeddings-v2-base-code");
        assert_eq!(config.dimensions, 768);
    }
}
