//! API Key Authentication Proxy Plugin for Hive
//!
//! Validates API keys from headers, query parameters, or cookies.
//!
//! ## Configuration
//!
//! ```yaml
//! proxy:
//!   auth-api-key:
//!     # Where to look for the API key
//!     source: header              # header, query, or cookie
//!     name: X-API-Key             # Header/query param/cookie name
//!     
//!     # API keys (use env vars in production!)
//!     keys:
//!       - key: "sk_live_abc123"
//!         name: "production"
//!         scopes: ["read", "write"]
//!       - key: "sk_test_xyz789"
//!         name: "test"
//!         scopes: ["read"]
//!     
//!     # Or reference from environment
//!     keys_env: API_KEYS          # JSON array of keys
//!     
//!     # Hash keys for security (store hashed, compare hashed)
//!     hash_keys: true
//!     
//!     # Forward key info to backend
//!     forward_header: X-API-Key-Name
//! ```

use anyhow::Context;
use lib_plugin_abi_v3::{
    async_trait,
    proxy::{ProxyMiddleware, ProxyRequest, ProxyResponse, ProxyResult},
    Plugin, PluginCategory, PluginContext, PluginMetadata, PluginType,
    Result as PluginResult, SERVICE_PROXY_MIDDLEWARE,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use tracing::{debug, warn};

#[derive(Debug, Clone)]
struct ApiKeyEntry {
    #[allow(dead_code)]
    key_hash: String,
    name: String,
    scopes: Vec<String>,
}

pub struct ApiKeyAuthPlugin {
    config: ApiKeyConfig,
    keys: HashMap<String, ApiKeyEntry>, // hash -> entry
}

impl Default for ApiKeyAuthPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl ApiKeyAuthPlugin {
    pub fn new() -> Self {
        Self {
            config: ApiKeyConfig::default(),
            keys: HashMap::new(),
        }
    }

    fn hash_key(key: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(key.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    fn extract_api_key(&self, req: &ProxyRequest) -> Option<String> {
        match self.config.source.as_str() {
            "header" => req.headers.get(&self.config.name).cloned(),
            "query" => {
                let uri = &req.uri;
                let query_start = uri.find('?')?;
                let query = &uri[query_start + 1..];

                for pair in query.split('&') {
                    let mut parts = pair.splitn(2, '=');
                    let key = parts.next()?;
                    let value = parts.next().unwrap_or("");
                    if key == self.config.name {
                        return Some(value.to_string());
                    }
                }
                None
            }
            "cookie" => {
                let cookie_header = req.headers.get("cookie")?;
                for cookie in cookie_header.split(';') {
                    let cookie = cookie.trim();
                    let mut parts = cookie.splitn(2, '=');
                    let name = parts.next()?;
                    let value = parts.next().unwrap_or("");
                    if name == self.config.name {
                        return Some(value.to_string());
                    }
                }
                None
            }
            _ => None,
        }
    }

    fn validate_key(&self, key: &str) -> Option<&ApiKeyEntry> {
        let lookup_hash = if self.config.hash_keys {
            Self::hash_key(key)
        } else {
            key.to_string()
        };

        self.keys.get(&lookup_hash)
    }

    fn check_scopes(&self, entry: &ApiKeyEntry, required: &[String]) -> bool {
        if required.is_empty() {
            return true;
        }

        for scope in required {
            if !entry.scopes.contains(scope) {
                return false;
            }
        }
        true
    }
}

#[async_trait]
impl Plugin for ApiKeyAuthPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            id: "hive.proxy.auth.api-key".to_string(),
            name: "API Key Auth".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            plugin_type: PluginType::Orchestration,
            author: Some("ADI".to_string()),
            description: Some("API key authentication proxy middleware".to_string()),
            category: Some(PluginCategory::Proxy),
        }
    }

    async fn init(&mut self, ctx: &PluginContext) -> PluginResult<()> {
        if let Some(auth_config) = ctx.config.get("auth-api-key") {
            self.config = serde_json::from_value(auth_config.clone())?;
        }

        self.keys.clear();
        for key_config in &self.config.keys {
            let key_hash = if self.config.hash_keys {
                Self::hash_key(&key_config.key)
            } else {
                key_config.key.clone()
            };

            self.keys.insert(
                key_hash.clone(),
                ApiKeyEntry {
                    key_hash,
                    name: key_config.name.clone(),
                    scopes: key_config.scopes.clone().unwrap_or_default(),
                },
            );
        }

        if let Some(ref env_var) = self.config.keys_env {
            let keys_json = lib_env_parse::env_require(env_var)
                .map_err(|e| anyhow::anyhow!("{e}"))?;
            let keys: Vec<ApiKeyConfigEntry> = serde_json::from_str(&keys_json)
                .context(format!("Failed to parse API keys JSON from env var `{env_var}`"))?;
            for key_config in keys {
                let key_hash = if self.config.hash_keys {
                    Self::hash_key(&key_config.key)
                } else {
                    key_config.key.clone()
                };

                self.keys.insert(
                    key_hash.clone(),
                    ApiKeyEntry {
                        key_hash,
                        name: key_config.name.clone(),
                        scopes: key_config.scopes.clone().unwrap_or_default(),
                    },
                );
            }
        }

        debug!(
            "API Key auth plugin initialized: {} keys, source={}, name={}",
            self.keys.len(),
            self.config.source,
            self.config.name
        );
        Ok(())
    }

    fn provides(&self) -> Vec<&'static str> {
        vec![SERVICE_PROXY_MIDDLEWARE]
    }
}

#[async_trait]
impl ProxyMiddleware for ApiKeyAuthPlugin {
    async fn process_request(&self, mut req: ProxyRequest) -> PluginResult<ProxyResult> {
        let api_key = match self.extract_api_key(&req) {
            Some(key) => key,
            None => {
                debug!("No API key found in request");
                return Ok(ProxyResult::Response(
                    ProxyResponse::new(401)
                        .with_header("content-type", "application/json")
                        .with_header("www-authenticate", format!("ApiKey realm=\"API\", name=\"{}\"", self.config.name))
                        .with_body(r#"{"error": "API key required"}"#),
                ));
            }
        };

        let entry = match self.validate_key(&api_key) {
            Some(entry) => entry,
            None => {
                warn!("Invalid API key attempted");
                return Ok(ProxyResult::Response(
                    ProxyResponse::new(401)
                        .with_header("content-type", "application/json")
                        .with_body(r#"{"error": "Invalid API key"}"#),
                ));
            }
        };

        if !self.check_scopes(entry, &self.config.required_scopes) {
            warn!("API key '{}' missing required scopes", entry.name);
            return Ok(ProxyResult::Response(
                ProxyResponse::new(403)
                    .with_header("content-type", "application/json")
                    .with_body(r#"{"error": "Insufficient permissions"}"#),
            ));
        }

        debug!("API key '{}' validated successfully", entry.name);

        if let Some(ref header_name) = self.config.forward_header {
            req.headers.insert(header_name.clone(), entry.name.clone());
        }

        if let Some(ref scopes_header) = self.config.forward_scopes_header {
            req.headers
                .insert(scopes_header.clone(), entry.scopes.join(","));
        }

        Ok(ProxyResult::Continue(req))
    }

    async fn process_response(&self, resp: ProxyResponse) -> PluginResult<ProxyResponse> {
        Ok(resp)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyConfig {
    /// Where to look for API key: header, query, cookie
    #[serde(default = "default_source")]
    pub source: String,
    /// Name of header/query param/cookie
    #[serde(default = "default_name")]
    pub name: String,
    #[serde(default)]
    pub keys: Vec<ApiKeyConfigEntry>,
    pub keys_env: Option<String>,
    #[serde(default)]
    pub hash_keys: bool,
    pub forward_header: Option<String>,
    pub forward_scopes_header: Option<String>,
    #[serde(default)]
    pub required_scopes: Vec<String>,
}

fn default_source() -> String {
    "header".to_string()
}

fn default_name() -> String {
    "X-API-Key".to_string()
}

impl Default for ApiKeyConfig {
    fn default() -> Self {
        Self {
            source: default_source(),
            name: default_name(),
            keys: Vec::new(),
            keys_env: None,
            hash_keys: false,
            forward_header: None,
            forward_scopes_header: None,
            required_scopes: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyConfigEntry {
    pub key: String,
    pub name: String,
    pub scopes: Option<Vec<String>>,
}

#[cfg(feature = "plugin")]
#[no_mangle]
pub fn plugin_create() -> Box<dyn Plugin> {
    Box::new(ApiKeyAuthPlugin::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metadata() {
        let plugin = ApiKeyAuthPlugin::new();
        let meta = plugin.metadata();
        assert_eq!(meta.id, "hive.proxy.auth.api-key");
        assert_eq!(meta.name, "API Key Auth");
        assert_eq!(meta.plugin_type, PluginType::Orchestration);
        assert_eq!(meta.category, Some(PluginCategory::Proxy));
    }

    #[test]
    fn test_hash_key() {
        let hash = ApiKeyAuthPlugin::hash_key("test-key");
        assert_eq!(hash.len(), 64); // SHA256 hex
    }

    #[tokio::test]
    async fn test_extract_from_header() {
        let mut plugin = ApiKeyAuthPlugin::new();
        plugin.config.source = "header".to_string();
        plugin.config.name = "X-API-Key".to_string();

        let mut headers = HashMap::new();
        headers.insert("X-API-Key".to_string(), "my-key".to_string());

        let req = ProxyRequest {
            method: "GET".to_string(),
            uri: "/api/test".to_string(),
            headers,
            client_ip: None,
            body: None,
        };

        assert_eq!(plugin.extract_api_key(&req), Some("my-key".to_string()));
    }

    #[tokio::test]
    async fn test_extract_from_query() {
        let mut plugin = ApiKeyAuthPlugin::new();
        plugin.config.source = "query".to_string();
        plugin.config.name = "api_key".to_string();

        let req = ProxyRequest {
            method: "GET".to_string(),
            uri: "/api/test?api_key=my-key&other=value".to_string(),
            headers: HashMap::new(),
            client_ip: None,
            body: None,
        };

        assert_eq!(plugin.extract_api_key(&req), Some("my-key".to_string()));
    }

    #[tokio::test]
    async fn test_extract_from_cookie() {
        let mut plugin = ApiKeyAuthPlugin::new();
        plugin.config.source = "cookie".to_string();
        plugin.config.name = "api_key".to_string();

        let mut headers = HashMap::new();
        headers.insert(
            "cookie".to_string(),
            "session=abc; api_key=my-key; other=val".to_string(),
        );

        let req = ProxyRequest {
            method: "GET".to_string(),
            uri: "/api/test".to_string(),
            headers,
            client_ip: None,
            body: None,
        };

        assert_eq!(plugin.extract_api_key(&req), Some("my-key".to_string()));
    }

    #[tokio::test]
    async fn test_validate_key() {
        let mut plugin = ApiKeyAuthPlugin::new();
        plugin.keys.insert(
            "test-key".to_string(),
            ApiKeyEntry {
                key_hash: "test-key".to_string(),
                name: "test".to_string(),
                scopes: vec!["read".to_string()],
            },
        );

        assert!(plugin.validate_key("test-key").is_some());
        assert!(plugin.validate_key("invalid").is_none());
    }

    #[tokio::test]
    async fn test_check_scopes() {
        let plugin = ApiKeyAuthPlugin::new();

        let entry = ApiKeyEntry {
            key_hash: "test".to_string(),
            name: "test".to_string(),
            scopes: vec!["read".to_string(), "write".to_string()],
        };

        assert!(plugin.check_scopes(&entry, &[]));
        assert!(plugin.check_scopes(&entry, &["read".to_string()]));
        assert!(plugin.check_scopes(
            &entry,
            &["read".to_string(), "write".to_string()]
        ));
        assert!(!plugin.check_scopes(&entry, &["admin".to_string()]));
    }
}
