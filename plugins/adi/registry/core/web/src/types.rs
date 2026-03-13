use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebPluginEntry {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
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
pub struct WebPluginInfo {
    pub id: String,
    pub version: String,
    #[serde(alias = "js_url")]
    pub js_url: String,
    #[serde(default, skip_serializing_if = "Option::is_none", alias = "css_url")]
    pub css_url: Option<String>,
    #[serde(default, alias = "size_bytes")]
    pub size_bytes: u64,
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
pub struct WebRegistryIndex {
    pub version: u32,
    #[serde(default, alias = "updated_at")]
    pub updated_at: u64,
    #[serde(default)]
    pub plugins: Vec<WebPluginEntry>,
}

impl Default for WebRegistryIndex {
    fn default() -> Self {
        Self {
            version: 1,
            updated_at: 0,
            plugins: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WebSearchResults {
    pub plugins: Vec<WebPluginEntry>,
}
