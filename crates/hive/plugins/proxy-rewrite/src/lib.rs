//! URL Rewriting Proxy Plugin for Hive
//!
//! Rewrites URL paths and query parameters before forwarding to backend.
//!
//! ## Configuration
//!
//! ```yaml
//! proxy:
//!   rewrite:
//!     rules:
//!       - match: ^/api/v1/(.*)$
//!         replace: /v1/$1
//!       - match: ^/old-path$
//!         replace: /new-path
//!         redirect: 301           # Optional: return redirect instead
//!     strip_prefix: /api          # Remove prefix from all requests
//!     add_prefix: /backend        # Add prefix to all requests
//!     query:
//!       add:
//!         format: json
//!       remove:
//!         - debug
//! ```

use async_trait::async_trait;
use lib_plugin_abi_v3::{
    proxy::{ProxyMiddleware, ProxyRequest, ProxyResponse, ProxyResult},
    Plugin, PluginCategory, PluginContext, PluginMetadata, PluginType,
    Result as PluginResult, SERVICE_PROXY_MIDDLEWARE,
};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::debug;

struct CompiledRule {
    pattern: Regex,
    replace: String,
    redirect: Option<u16>,
}

pub struct RewritePlugin {
    config: RewriteConfig,
    compiled_rules: Vec<CompiledRule>,
}

impl Default for RewritePlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl RewritePlugin {
    pub fn new() -> Self {
        Self {
            config: RewriteConfig::default(),
            compiled_rules: Vec::new(),
        }
    }

    fn rewrite_uri(&self, uri: &str) -> PluginResult<RewriteResult> {
        let (path, query) = match uri.find('?') {
            Some(pos) => (&uri[..pos], Some(&uri[pos + 1..])),
            None => (uri, None),
        };

        let mut new_path = path.to_string();

        if let Some(ref prefix) = self.config.strip_prefix {
            if new_path.starts_with(prefix) {
                new_path = new_path[prefix.len()..].to_string();
                if new_path.is_empty() {
                    new_path = "/".to_string();
                }
            }
        }

        for rule in &self.compiled_rules {
            if rule.pattern.is_match(&new_path) {
                let rewritten = rule.pattern.replace(&new_path, rule.replace.as_str());

                if let Some(status) = rule.redirect {
                    let redirect_uri = self.build_uri(&rewritten, query);
                    return Ok(RewriteResult::Redirect {
                        uri: redirect_uri,
                        status,
                    });
                }

                new_path = rewritten.to_string();
                break;
            }
        }

        if let Some(ref prefix) = self.config.add_prefix {
            new_path = format!("{}{}", prefix, new_path);
        }

        let new_query = self.rewrite_query(query);

        let final_uri = self.build_uri(&new_path, new_query.as_deref());

        Ok(RewriteResult::Rewritten(final_uri))
    }

    fn rewrite_query(&self, query: Option<&str>) -> Option<String> {
        let query_config = self.config.query.as_ref()?;

        let mut params: HashMap<String, String> = query
            .map(|q| {
                q.split('&')
                    .filter_map(|pair| {
                        let mut parts = pair.splitn(2, '=');
                        let key = parts.next()?;
                        let value = parts.next().unwrap_or("");
                        Some((key.to_string(), value.to_string()))
                    })
                    .collect()
            })
            .unwrap_or_default();

        if let Some(ref remove) = query_config.remove {
            for key in remove {
                params.remove(key);
            }
        }

        if let Some(ref add) = query_config.add {
            for (key, value) in add {
                params.insert(key.clone(), value.clone());
            }
        }

        if params.is_empty() {
            return None;
        }

        let query_string: String = params
            .iter()
            .map(|(k, v)| {
                if v.is_empty() {
                    k.clone()
                } else {
                    format!("{}={}", k, v)
                }
            })
            .collect::<Vec<_>>()
            .join("&");

        Some(query_string)
    }

    fn build_uri(&self, path: &str, query: Option<&str>) -> String {
        match query {
            Some(q) if !q.is_empty() => format!("{}?{}", path, q),
            _ => path.to_string(),
        }
    }
}

enum RewriteResult {
    Rewritten(String),
    Redirect { uri: String, status: u16 },
}

#[async_trait]
impl Plugin for RewritePlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            id: "hive.proxy.rewrite".to_string(),
            name: "rewrite".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            plugin_type: PluginType::Orchestration,
            author: Some("ADI Team".to_string()),
            description: Some("URL path and query rewriting".to_string()),
            category: Some(PluginCategory::Proxy),
        }
    }

    async fn init(&mut self, ctx: &PluginContext) -> PluginResult<()> {
        if let Some(rewrite_config) = ctx.config.get("rewrite") {
            self.config = serde_json::from_value(rewrite_config.clone())?;
        }

        self.compiled_rules.clear();
        for rule in &self.config.rules {
            let pattern = Regex::new(&rule.match_pattern).map_err(|e| {
                lib_plugin_abi_v3::PluginError::InitFailed(format!(
                    "Invalid rewrite pattern '{}': {}",
                    rule.match_pattern, e
                ))
            })?;

            self.compiled_rules.push(CompiledRule {
                pattern,
                replace: rule.replace.clone(),
                redirect: rule.redirect,
            });
        }

        debug!(
            "Rewrite plugin initialized: {} rules, strip_prefix={:?}, add_prefix={:?}",
            self.compiled_rules.len(),
            self.config.strip_prefix,
            self.config.add_prefix
        );
        Ok(())
    }

    fn provides(&self) -> Vec<&'static str> {
        vec![SERVICE_PROXY_MIDDLEWARE]
    }
}

#[async_trait]
impl ProxyMiddleware for RewritePlugin {
    async fn process_request(&self, mut req: ProxyRequest) -> PluginResult<ProxyResult> {
        let original_uri = req.uri.clone();

        match self.rewrite_uri(&req.uri)? {
            RewriteResult::Rewritten(new_uri) => {
                if new_uri != original_uri {
                    debug!("Rewriting URI: {} -> {}", original_uri, new_uri);
                    req.uri = new_uri;
                }
                Ok(ProxyResult::Continue(req))
            }
            RewriteResult::Redirect { uri, status } => {
                debug!("Redirecting {} -> {} ({})", original_uri, uri, status);
                let response = ProxyResponse::new(status)
                    .with_header("location", uri)
                    .with_header("content-length", "0");
                Ok(ProxyResult::Response(response))
            }
        }
    }

    async fn process_response(&self, resp: ProxyResponse) -> PluginResult<ProxyResponse> {
        Ok(resp)
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RewriteConfig {
    #[serde(default)]
    pub rules: Vec<RewriteRule>,
    pub strip_prefix: Option<String>,
    pub add_prefix: Option<String>,
    pub query: Option<QueryConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewriteRule {
    #[serde(rename = "match")]
    pub match_pattern: String,
    pub replace: String,
    /// HTTP status code for redirect (e.g. 301). If absent, rewrites in-place.
    pub redirect: Option<u16>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct QueryConfig {
    pub add: Option<HashMap<String, String>>,
    pub remove: Option<Vec<String>>,
}

#[cfg(feature = "plugin")]
#[no_mangle]
pub fn plugin_create() -> Box<dyn Plugin> {
    Box::new(RewritePlugin::new())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn make_test_context(config: serde_json::Value) -> PluginContext {
        PluginContext::new(
            "hive.proxy.rewrite",
            PathBuf::from("/tmp/data"),
            PathBuf::from("/tmp/config"),
            config,
        )
    }

    #[tokio::test]
    async fn test_metadata() {
        let plugin = RewritePlugin::new();
        let meta = plugin.metadata();
        assert_eq!(meta.id, "hive.proxy.rewrite");
        assert_eq!(meta.name, "rewrite");
    }

    #[tokio::test]
    async fn test_strip_prefix() {
        let mut plugin = RewritePlugin::new();
        plugin.config.strip_prefix = Some("/api".to_string());

        match plugin.rewrite_uri("/api/users").unwrap() {
            RewriteResult::Rewritten(uri) => assert_eq!(uri, "/users"),
            _ => panic!("Expected Rewritten"),
        }
    }

    #[tokio::test]
    async fn test_add_prefix() {
        let mut plugin = RewritePlugin::new();
        plugin.config.add_prefix = Some("/backend".to_string());

        match plugin.rewrite_uri("/users").unwrap() {
            RewriteResult::Rewritten(uri) => assert_eq!(uri, "/backend/users"),
            _ => panic!("Expected Rewritten"),
        }
    }

    #[tokio::test]
    async fn test_rewrite_rule() {
        let mut plugin = RewritePlugin::new();
        plugin.config.rules.push(RewriteRule {
            match_pattern: r"^/api/v1/(.*)$".to_string(),
            replace: "/v2/$1".to_string(),
            redirect: None,
        });

        plugin.init(&make_test_context(serde_json::json!({}))).await.unwrap();

        match plugin.rewrite_uri("/api/v1/users").unwrap() {
            RewriteResult::Rewritten(uri) => assert_eq!(uri, "/v2/users"),
            _ => panic!("Expected Rewritten"),
        }
    }

    #[tokio::test]
    async fn test_redirect_rule() {
        let mut plugin = RewritePlugin::new();
        plugin.config.rules.push(RewriteRule {
            match_pattern: r"^/old$".to_string(),
            replace: "/new".to_string(),
            redirect: Some(301),
        });

        plugin.init(&make_test_context(serde_json::json!({}))).await.unwrap();

        match plugin.rewrite_uri("/old").unwrap() {
            RewriteResult::Redirect { uri, status } => {
                assert_eq!(uri, "/new");
                assert_eq!(status, 301);
            }
            _ => panic!("Expected Redirect"),
        }
    }

    #[tokio::test]
    async fn test_query_modification() {
        let mut plugin = RewritePlugin::new();
        plugin.config.query = Some(QueryConfig {
            add: Some([("format".to_string(), "json".to_string())].into()),
            remove: Some(vec!["debug".to_string()]),
        });

        match plugin.rewrite_uri("/api?debug=1&foo=bar").unwrap() {
            RewriteResult::Rewritten(uri) => {
                assert!(uri.contains("format=json"));
                assert!(uri.contains("foo=bar"));
                assert!(!uri.contains("debug"));
            }
            _ => panic!("Expected Rewritten"),
        }
    }
}
