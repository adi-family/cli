// Copyright (c) 2024-2025 Ihor
// SPDX-License-Identifier: BSL-1.1
// See LICENSE file for details

//! Shared embedding library for ADI tools.
//!
//! Provides a common interface for text embeddings.
//!
//! ## Backends
//!
//! - **Plugin-based** (default): Uses the `adi.embed` plugin service.
//!   Requires the plugin to be installed: `adi plugin install adi.embed`
//!
//! - **Local fastembed** (optional): Enable the `fastembed` feature for
//!   built-in ONNX-based embeddings. Adds ~20MB to binary size.
//!
//! ## Usage
//!
//! ```rust,ignore
//! use lib_embed::{Embedder, PluginEmbedder};
//!
//! let embedder = PluginEmbedder::new(plugin_host)?;
//! let embeddings = embedder.embed(&["hello world"])?;
//! ```

mod config;
mod error;
mod plugin_embedder;

#[cfg(feature = "fastembed")]
mod fastembed_impl;

pub use config::EmbeddingConfig;
pub use error::{EmbedError, Result};
pub use plugin_embedder::PluginEmbedder;

#[cfg(feature = "fastembed")]
pub use fastembed_impl::FastEmbedder;

/// Trait for text embedding providers.
pub trait Embedder: Send + Sync {
    /// Generate embeddings for a batch of texts.
    fn embed(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>>;

    /// Get the embedding dimensions.
    fn dimensions(&self) -> u32;

    /// Get the model name.
    fn model_name(&self) -> &str;
}
