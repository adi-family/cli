//! Response Caching Proxy Plugin for Hive
//!
//! Caches HTTP responses in memory with TTL support.
//!
//! ## Features
//!
//! - **Cache reads**: Returns cached responses for matching requests
//! - **Cache writes**: Stores cacheable responses using task-local storage
//! - **TTL**: Respects Cache-Control max-age/s-maxage or uses default TTL
//! - **Vary headers**: Includes configured headers in cache key computation
//! - **LRU eviction**: Removes oldest entries when at capacity
//!
//! ## Configuration
//!
//! ```yaml
//! proxy:
//!   cache:
//!     ttl: 300                    # Default TTL in seconds
//!     max_size: 1000              # Max cached entries
//!     max_body_size: 1048576      # Max body size to cache (1MB)
//!     methods: [GET, HEAD]        # Methods to cache
//!     statuses: [200, 301, 302]   # Status codes to cache
//!     vary: [Accept, Accept-Encoding]  # Headers to include in cache key
//! ```
//!
//! ## Cache Headers
//!
//! - `x-cache: HIT` - Response served from cache
//! - `x-cache: MISS` - Response fetched from upstream
//! - `x-cache: BYPASS` - Response not cacheable (method, status, or headers)

use lib_plugin_abi_v3::{
    async_trait,
    proxy::{ProxyMiddleware, ProxyRequest, ProxyResponse, ProxyResult},
    Plugin, PluginCategory, PluginContext, PluginMetadata, PluginType,
    Result as PluginResult, SERVICE_PROXY_MIDDLEWARE,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::debug;

// Task-local storage for cache key (needed to pass key from process_request to process_response)
tokio::task_local! {
    static PENDING_CACHE_KEY: RefCell<Option<String>>;
}

#[derive(Clone)]
struct CacheEntry {
    response: ProxyResponse,
    created_at: Instant,
    ttl: Duration,
}

impl CacheEntry {
    fn is_expired(&self) -> bool {
        self.created_at.elapsed() > self.ttl
    }
}

pub struct CachePlugin {
    config: CacheConfig,
    cache: Arc<RwLock<HashMap<String, CacheEntry>>>,
}

impl Default for CachePlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl CachePlugin {
    pub fn new() -> Self {
        Self {
            config: CacheConfig::default(),
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    fn should_cache_request(&self, req: &ProxyRequest) -> bool {
        self.config.methods.iter().any(|m| m.eq_ignore_ascii_case(&req.method))
    }

    fn should_cache_response(&self, resp: &ProxyResponse) -> bool {
        if !self.config.statuses.contains(&resp.status) {
            return false;
        }

        if resp.body.len() > self.config.max_body_size {
            return false;
        }

        if let Some(cc) = resp.headers.get("cache-control") {
            let cc = cc.to_lowercase();
            if cc.contains("no-store") || cc.contains("private") {
                return false;
            }
        }

        true
    }

    fn compute_cache_key(&self, req: &ProxyRequest) -> String {
        let mut hasher = Sha256::new();

        hasher.update(req.method.as_bytes());
        hasher.update(req.uri.as_bytes());

        for header in &self.config.vary {
            if let Some(value) = req.headers.get(header) {
                hasher.update(header.as_bytes());
                hasher.update(value.as_bytes());
            }
        }

        format!("{:x}", hasher.finalize())
    }

    fn parse_ttl_from_response(&self, resp: &ProxyResponse) -> Duration {
        if let Some(cc) = resp.headers.get("cache-control") {
            for directive in cc.split(',') {
                let directive = directive.trim();
                if let Some(max_age) = directive.strip_prefix("max-age=") {
                    if let Ok(secs) = max_age.trim().parse::<u64>() {
                        return Duration::from_secs(secs);
                    }
                }
                if let Some(s_maxage) = directive.strip_prefix("s-maxage=") {
                    if let Ok(secs) = s_maxage.trim().parse::<u64>() {
                        return Duration::from_secs(secs);
                    }
                }
            }
        }

        Duration::from_secs(self.config.ttl)
    }

    async fn get_cached(&self, key: &str) -> Option<ProxyResponse> {
        let cache = self.cache.read().await;
        if let Some(entry) = cache.get(key) {
            if !entry.is_expired() {
                return Some(entry.response.clone());
            }
        }
        None
    }

    async fn store_cached(&self, key: String, response: ProxyResponse, ttl: Duration) {
        let mut cache = self.cache.write().await;

        if cache.len() >= self.config.max_size {
            cache.retain(|_, entry| !entry.is_expired());
        }

        if cache.len() >= self.config.max_size {
            if let Some(oldest_key) = cache
                .iter()
                .min_by_key(|(_, e)| e.created_at)
                .map(|(k, _)| k.clone())
            {
                cache.remove(&oldest_key);
            }
        }

        cache.insert(
            key,
            CacheEntry {
                response,
                created_at: Instant::now(),
                ttl,
            },
        );
    }

    async fn cleanup_expired(&self) {
        let mut cache = self.cache.write().await;
        cache.retain(|_, entry| !entry.is_expired());
    }
}

#[async_trait]
impl Plugin for CachePlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            id: "hive.proxy.cache".to_string(),
            name: "cache".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            plugin_type: PluginType::Orchestration,
            author: None,
            description: Some("Response caching with TTL support".to_string()),
            category: Some(PluginCategory::Proxy),
        }
    }

    async fn init(&mut self, ctx: &PluginContext) -> PluginResult<()> {
        if let Some(cache_config) = ctx.config.get("cache") {
            self.config = serde_json::from_value(cache_config.clone())?;
        }
        debug!(
            "Cache plugin initialized: ttl={}s, max_size={}",
            self.config.ttl, self.config.max_size
        );
        Ok(())
    }

    async fn shutdown(&self) -> PluginResult<()> {
        self.cleanup_expired().await;
        Ok(())
    }

    fn provides(&self) -> Vec<&'static str> {
        vec![SERVICE_PROXY_MIDDLEWARE]
    }
}

#[async_trait]
impl ProxyMiddleware for CachePlugin {
    async fn process_request(&self, req: ProxyRequest) -> PluginResult<ProxyResult> {
        if !self.should_cache_request(&req) {
            return Ok(ProxyResult::Continue(req));
        }

        let cache_key = self.compute_cache_key(&req);

        if let Some(mut cached_response) = self.get_cached(&cache_key).await {
            debug!("Cache HIT for {}", req.uri);
            cached_response
                .headers
                .insert("x-cache".to_string(), "HIT".to_string());
            return Ok(ProxyResult::Response(cached_response));
        }

        debug!("Cache MISS for {}", req.uri);

        // Requires the proxy to wrap the request/response in the task-local scope
        let _ = PENDING_CACHE_KEY.try_with(|key| {
            *key.borrow_mut() = Some(cache_key);
        });

        Ok(ProxyResult::Continue(req))
    }

    async fn process_response(&self, mut resp: ProxyResponse) -> PluginResult<ProxyResponse> {
        let cache_key = PENDING_CACHE_KEY
            .try_with(|key| key.borrow_mut().take())
            .ok()
            .flatten();

        match cache_key {
            Some(key) => {
                if self.should_cache_response(&resp) {
                    let ttl = self.parse_ttl_from_response(&resp);
                    debug!("Caching response for key {} with TTL {:?}", key, ttl);
                    self.store_cached(key, resp.clone(), ttl).await;
                    resp.headers
                        .insert("x-cache".to_string(), "MISS".to_string());
                } else {
                    debug!("Response not cacheable (status or headers)");
                    resp.headers
                        .insert("x-cache".to_string(), "BYPASS".to_string());
                }
            }
            None => {
                resp.headers
                    .insert("x-cache".to_string(), "BYPASS".to_string());
            }
        }

        Ok(resp)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    /// Default TTL in seconds
    #[serde(default = "default_ttl")]
    pub ttl: u64,
    #[serde(default = "default_max_size")]
    pub max_size: usize,
    /// Maximum body size to cache (bytes)
    #[serde(default = "default_max_body_size")]
    pub max_body_size: usize,
    #[serde(default = "default_methods")]
    pub methods: Vec<String>,
    #[serde(default = "default_statuses")]
    pub statuses: Vec<u16>,
    /// Headers to include in cache key (Vary)
    #[serde(default = "default_vary")]
    pub vary: Vec<String>,
}

fn default_ttl() -> u64 {
    300
}

fn default_max_size() -> usize {
    1000
}

fn default_max_body_size() -> usize {
    1048576 // 1MB
}

fn default_methods() -> Vec<String> {
    vec!["GET".to_string(), "HEAD".to_string()]
}

fn default_statuses() -> Vec<u16> {
    vec![200, 301, 302, 304]
}

fn default_vary() -> Vec<String> {
    vec!["Accept".to_string(), "Accept-Encoding".to_string()]
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            ttl: default_ttl(),
            max_size: default_max_size(),
            max_body_size: default_max_body_size(),
            methods: default_methods(),
            statuses: default_statuses(),
            vary: default_vary(),
        }
    }
}

#[cfg(feature = "plugin")]
#[no_mangle]
pub fn plugin_create() -> Box<dyn Plugin> {
    Box::new(CachePlugin::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metadata() {
        let plugin = CachePlugin::new();
        let meta = plugin.metadata();
        assert_eq!(meta.id, "hive.proxy.cache");
        assert_eq!(meta.name, "cache");
    }

    #[test]
    fn test_should_cache_request() {
        let plugin = CachePlugin::new();

        let get_req = ProxyRequest {
            method: "GET".to_string(),
            uri: "/api/test".to_string(),
            headers: HashMap::new(),
            client_ip: None,
            body: None,
        };
        assert!(plugin.should_cache_request(&get_req));

        let post_req = ProxyRequest {
            method: "POST".to_string(),
            uri: "/api/test".to_string(),
            headers: HashMap::new(),
            client_ip: None,
            body: None,
        };
        assert!(!plugin.should_cache_request(&post_req));
    }

    #[test]
    fn test_should_cache_response() {
        let plugin = CachePlugin::new();

        let ok_resp = ProxyResponse {
            status: 200,
            headers: HashMap::new(),
            body: vec![0; 100],
        };
        assert!(plugin.should_cache_response(&ok_resp));

        let error_resp = ProxyResponse {
            status: 500,
            headers: HashMap::new(),
            body: vec![0; 100],
        };
        assert!(!plugin.should_cache_response(&error_resp));

        // No-store should not be cached
        let mut no_store_resp = ProxyResponse {
            status: 200,
            headers: HashMap::new(),
            body: vec![0; 100],
        };
        no_store_resp
            .headers
            .insert("cache-control".to_string(), "no-store".to_string());
        assert!(!plugin.should_cache_response(&no_store_resp));
    }

    #[test]
    fn test_parse_ttl() {
        let plugin = CachePlugin::new();

        let mut resp = ProxyResponse::new(200);
        resp.headers
            .insert("cache-control".to_string(), "max-age=600".to_string());
        assert_eq!(plugin.parse_ttl_from_response(&resp), Duration::from_secs(600));

        let resp_default = ProxyResponse::new(200);
        assert_eq!(
            plugin.parse_ttl_from_response(&resp_default),
            Duration::from_secs(300)
        );
    }

    #[test]
    fn test_cache_key() {
        let plugin = CachePlugin::new();

        let req1 = ProxyRequest {
            method: "GET".to_string(),
            uri: "/api/test".to_string(),
            headers: HashMap::new(),
            client_ip: None,
            body: None,
        };

        let req2 = ProxyRequest {
            method: "GET".to_string(),
            uri: "/api/test".to_string(),
            headers: HashMap::new(),
            client_ip: None,
            body: None,
        };

        let req3 = ProxyRequest {
            method: "GET".to_string(),
            uri: "/api/other".to_string(),
            headers: HashMap::new(),
            client_ip: None,
            body: None,
        };

        assert_eq!(plugin.compute_cache_key(&req1), plugin.compute_cache_key(&req2));
        assert_ne!(plugin.compute_cache_key(&req1), plugin.compute_cache_key(&req3));
    }
}
