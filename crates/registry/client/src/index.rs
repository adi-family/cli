//! Registry index types.

use serde::{Deserialize, Deserializer, Serialize};

/// Deserializes either a single string `"web"` or an array `["web","http"]`
/// into `Vec<String>`. Handles old index files that stored a single type string.
fn de_string_or_seq<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    struct V;
    impl<'de> serde::de::Visitor<'de> for V {
        type Value = Vec<String>;
        fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            f.write_str("a string or array of strings")
        }
        fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Self::Value, E> {
            Ok(vec![v.to_string()])
        }
        fn visit_seq<A: serde::de::SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
            let mut out = Vec::new();
            while let Some(s) = seq.next_element::<String>()? { out.push(s); }
            Ok(out)
        }
    }
    deserializer.deserialize_any(V)
}

/// Registry index containing all packages and plugins.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistryIndex {
    /// Index format version
    pub version: u32,

    /// Last updated timestamp (Unix epoch)
    #[serde(default, alias = "updated_at")]
    pub updated_at: u64,

    /// Multi-plugin packages
    #[serde(default)]
    pub packages: Vec<PackageEntry>,

    /// Single plugins
    #[serde(default)]
    pub plugins: Vec<PluginEntry>,
}

impl Default for RegistryIndex {
    fn default() -> Self {
        Self {
            version: 1,
            updated_at: 0,
            packages: Vec::new(),
            plugins: Vec::new(),
        }
    }
}

/// Entry for a multi-plugin package.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageEntry {
    /// Unique identifier
    pub id: String,

    /// Human-readable name
    pub name: String,

    /// Description
    #[serde(default)]
    pub description: String,

    /// Number of plugins in the package
    #[serde(default, alias = "plugin_count")]
    pub plugin_count: u32,

    /// IDs of plugins contained in the package
    #[serde(default, alias = "plugin_ids")]
    pub plugin_ids: Vec<String>,

    /// Latest version
    #[serde(alias = "latest_version")]
    pub latest_version: String,

    /// Download count
    #[serde(default)]
    pub downloads: u64,

    /// Author
    #[serde(default)]
    pub author: String,

    /// Tags/categories
    #[serde(default)]
    pub tags: Vec<String>,
}

/// Entry for a single plugin.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginEntry {
    /// Unique identifier
    pub id: String,

    /// Human-readable name
    pub name: String,

    /// Description
    #[serde(default)]
    pub description: String,

    /// Plugin kinds this plugin supports (e.g. ["web"], ["http","web"], ["core"]).
    #[serde(
        alias = "plugin_types",
        alias = "plugin_type",
        alias = "pluginType",
        deserialize_with = "de_string_or_seq"
    )]
    pub plugin_types: Vec<String>,

    /// Parent package ID (None if standalone)
    #[serde(default, alias = "package_id")]
    pub package_id: Option<String>,

    /// Latest version
    #[serde(alias = "latest_version")]
    pub latest_version: String,

    /// Download count
    #[serde(default)]
    pub downloads: u64,

    /// Author
    #[serde(default)]
    pub author: String,

    /// Tags/categories
    #[serde(default)]
    pub tags: Vec<String>,
}

/// Detailed package information (from specific version endpoint).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageInfo {
    /// Package ID
    pub id: String,

    /// Version
    pub version: String,

    /// Platform builds available
    pub platforms: Vec<PlatformBuild>,

    /// Publication timestamp
    #[serde(default, alias = "published_at")]
    pub published_at: u64,

    /// Changelog/release notes
    #[serde(default)]
    pub changelog: Option<String>,
}

/// Detailed plugin information (from specific version endpoint).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginInfo {
    /// Plugin ID
    pub id: String,

    /// Version
    pub version: String,

    /// Platform builds available
    pub platforms: Vec<PlatformBuild>,

    /// Publication timestamp
    #[serde(default, alias = "published_at")]
    pub published_at: u64,

    /// Web UI metadata (present if plugin has a web interface)
    #[serde(default, skip_serializing_if = "Option::is_none", alias = "web_ui")]
    pub web_ui: Option<WebUiMeta>,
}

/// Metadata about a plugin's web UI entry point.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebUiMeta {
    /// URL path to the JS entry point (e.g., "/v1/plugins/{id}/{version}/web.js")
    #[serde(alias = "entry_url")]
    pub entry_url: String,

    /// Size of the JS file in bytes
    #[serde(alias = "size_bytes")]
    pub size_bytes: u64,
}

/// Certificate issued by the registry binding a publisher ID to their public key.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PublisherCertificate {
    pub publisher_id: String,
    pub publisher_public_key: String,
    pub registry_signature: String,
    pub created_at: u64,
}

/// Platform-specific build information.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlatformBuild {
    /// Platform identifier (e.g., "darwin-aarch64")
    pub platform: String,

    /// Download URL
    #[serde(alias = "download_url")]
    pub download_url: String,

    /// File size in bytes
    #[serde(default, alias = "size_bytes")]
    pub size_bytes: u64,

    /// SHA256 checksum
    pub checksum: String,

    /// Base64 Ed25519 signature from publisher
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub publisher_signature: Option<String>,

    /// Base64 public key of publisher
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub publisher_public_key: Option<String>,

    /// Base64 Ed25519 co-signature from registry
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub registry_signature: Option<String>,

    /// Publisher identity
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub publisher_id: Option<String>,

    /// Publisher certificate (registry-signed binding of publisher_id to public key)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub publisher_certificate: Option<PublisherCertificate>,
}

/// Search results containing both packages and plugins.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SearchResults {
    /// Matching packages
    pub packages: Vec<PackageEntry>,

    /// Matching plugins
    pub plugins: Vec<PluginEntry>,
}

impl SearchResults {
    /// Check if results are empty.
    pub fn is_empty(&self) -> bool {
        self.packages.is_empty() && self.plugins.is_empty()
    }

    /// Total number of results.
    pub fn total(&self) -> usize {
        self.packages.len() + self.plugins.len()
    }
}

/// What kind of items to search for.
#[derive(Debug, Clone, Copy, Default)]
pub enum SearchKind {
    /// Search both packages and plugins
    #[default]
    All,
    /// Search only packages
    PackagesOnly,
    /// Search only plugins
    PluginsOnly,
}

impl SearchKind {
    /// Convert to query parameter value.
    pub fn as_str(&self) -> &'static str {
        match self {
            SearchKind::All => "all",
            SearchKind::PackagesOnly => "package",
            SearchKind::PluginsOnly => "plugin",
        }
    }
}
