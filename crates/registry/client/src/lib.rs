//! Plugin registry HTTP client.
//!
//! Provides async client for fetching plugins from a registry server.
//!
//! # Example
//!
//! ```rust,ignore
//! use lib_plugin_registry::{RegistryClient, SearchKind};
//!
//! let client = RegistryClient::new("https://plugins.example.com")
//!     .with_cache(PathBuf::from("~/.cache/plugins"));
//!
//! // Fetch index
//! let index = client.fetch_index().await?;
//!
//! // Search
//! let results = client.search("theme", SearchKind::All).await?;
//!
//! // Download
//! let bytes = client.download_package("vendor.pack", "1.0.0", "darwin-aarch64", |done, total| {
//!     println!("Progress: {}/{}", done, total);
//! }).await?;
//! ```

mod cache;
mod client;
mod error;
mod index;

pub use cache::*;
pub use client::*;
pub use error::*;
pub use index::*;
