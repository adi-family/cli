// Copyright (c) 2024-2025 Ihor
// SPDX-License-Identifier: BSL-1.1
// See LICENSE file for details

//! Plugin-based embedder that calls the adi.embed service.
//!
//! This is the default embedder that doesn't bundle ONNX runtime,
//! keeping binary sizes small. Requires the adi.embed plugin to be installed.

use crate::error::{EmbedError, Result};
use crate::Embedder;
use lib_plugin_abi_v3::embed::Embedder as V3Embedder;
use lib_plugin_host::PluginManagerV3;
use std::sync::Arc;

/// Plugin-based text embedder.
///
/// Delegates embedding operations to the adi.embed plugin service.
/// This keeps the binary size small by not bundling ONNX runtime.
pub struct PluginEmbedder {
    plugin: Arc<dyn V3Embedder>,
    model_name: String,
    dimensions: u32,
}

impl PluginEmbedder {
    /// Create a new plugin embedder.
    ///
    /// # Arguments
    /// * `plugin_manager` - The plugin manager with an embedder plugin registered
    ///
    /// # Errors
    /// Returns an error if no embedder plugin is available.
    pub fn new(plugin_manager: Arc<PluginManagerV3>) -> Result<Self> {
        // Get the default embedder plugin
        let plugin = plugin_manager.get_default_embedder().ok_or_else(|| {
            EmbedError::Embedding(
                "No embedder plugin found. Install with: adi plugin install adi.embed".to_string(),
            )
        })?;

        // Get model info
        let info = plugin.embedder_info();

        Ok(Self {
            plugin,
            model_name: info.model_name,
            dimensions: info.dimensions,
        })
    }

    /// Check if any embedder plugin is available.
    pub fn is_available(plugin_manager: &PluginManagerV3) -> bool {
        plugin_manager.has_embedder()
    }
}

impl Embedder for PluginEmbedder {
    fn embed(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        if texts.is_empty() {
            return Ok(vec![]);
        }

        let plugin = self.plugin.clone();
        let texts: Vec<String> = texts.iter().map(|s| s.to_string()).collect();

        // Use tokio runtime to call async method
        let result = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async { plugin.embed(&texts).await })
        });

        result.map_err(|e| EmbedError::Embedding(format!("Embedding failed: {}", e)))
    }

    fn dimensions(&self) -> u32 {
        self.dimensions
    }

    fn model_name(&self) -> &str {
        &self.model_name
    }
}
