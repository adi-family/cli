mod cache;
mod client;
mod error;

#[cfg(test)]
mod tests;

pub use cache::*;
pub use client::*;
pub use error::*;

// Re-export types from core crates
pub use adi_registry_core_cli::{
    CliPluginEntry, CliPluginInfo, CliRegistryIndex, CliSearchResults,
};
pub use adi_registry_core_shared::types::{PlatformBuild, PublisherCertificate};
