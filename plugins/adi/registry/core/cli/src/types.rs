use adi_registry_core_shared::types::PlatformBuild;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CliPluginEntry {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default, alias = "plugin_types", alias = "plugin_type")]
    pub plugin_types: Vec<String>,
    #[serde(alias = "latest_version")]
    pub latest_version: String,
    #[serde(default)]
    pub downloads: u64,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CliPluginInfo {
    pub id: String,
    pub version: String,
    pub platforms: Vec<PlatformBuild>,
    #[serde(default, alias = "published_at")]
    pub published_at: u64,
    #[serde(default)]
    pub changelog: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none", alias = "preview_url")]
    pub preview_url: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty", alias = "preview_images")]
    pub preview_images: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CliRegistryIndex {
    pub version: u32,
    #[serde(default, alias = "updated_at")]
    pub updated_at: u64,
    #[serde(default)]
    pub plugins: Vec<CliPluginEntry>,
}

impl Default for CliRegistryIndex {
    fn default() -> Self {
        Self {
            version: 1,
            updated_at: 0,
            plugins: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CliSearchResults {
    pub plugins: Vec<CliPluginEntry>,
}

impl CliSearchResults {
    pub fn is_empty(&self) -> bool {
        self.plugins.is_empty()
    }

    pub fn total(&self) -> usize {
        self.plugins.len()
    }
}
