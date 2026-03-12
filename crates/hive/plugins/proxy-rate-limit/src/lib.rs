//! Rate Limiting Proxy Middleware Plugin for Hive
//!
//! Limits request rates to protect backend services.
//!
//! ## Configuration
//!
//! ```yaml
//! proxy:
//!   plugins:
//!     - type: rate-limit
//!       rate-limit:
//!         requests: 1000
//!         window: 60s
//!         burst: 50
//!         by: ip  # ip, path, or header:X-Api-Key
//! ```

use lib_plugin_abi_v3::{
    async_trait,
    proxy::{ProxyMiddleware, ProxyRequest, ProxyResponse, ProxyResult},
    utils::parse_duration,
    Plugin, PluginCategory, PluginContext, PluginMetadata, PluginType,
    Result as PluginResult, SERVICE_PROXY_MIDDLEWARE,
};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::debug;

const MAX_TRACKED_KEYS: usize = 10_000;
/// Half of MAX_TRACKED_KEYS; entries are evicted down to this count when cleanup is triggered.
const CLEANUP_THRESHOLD: usize = MAX_TRACKED_KEYS / 2;

pub struct RateLimitPlugin {
    max_requests: u64,
    window: Duration,
    burst: u64,
    by: RateLimitBy,
    header_name: Option<String>,
    states: Arc<RwLock<HashMap<String, RateLimitState>>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RateLimitBy {
    Ip,
    Header,
    Path,
}

struct RateLimitState {
    count: AtomicU64,
    window_start: Instant,
}

impl Default for RateLimitPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl RateLimitPlugin {
    pub fn new() -> Self {
        Self {
            max_requests: 1000,
            window: Duration::from_secs(60),
            burst: 50,
            by: RateLimitBy::Ip,
            header_name: None,
            states: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    fn get_key(&self, req: &ProxyRequest) -> String {
        match self.by {
            RateLimitBy::Ip => req
                .header("x-forwarded-for")
                .map(|v| v.split(',').next().unwrap_or(v).trim().to_string())
                .or_else(|| req.header("x-real-ip").map(String::from))
                .or_else(|| req.client_ip.clone())
                .unwrap_or_else(|| "unknown".to_string()),
            RateLimitBy::Header => self
                .header_name
                .as_ref()
                .and_then(|name| req.header(name).map(String::from))
                .unwrap_or_else(|| "unknown".to_string()),
            RateLimitBy::Path => req.uri.clone(),
        }
    }

    async fn is_rate_limited(&self, key: &str) -> bool {
        let mut states = self.states.write().await;
        let now = Instant::now();

        if states.len() > MAX_TRACKED_KEYS {
            self.cleanup_expired_entries(&mut states, now);
        }

        if let Some(state) = states.get(key) {
            if now.duration_since(state.window_start) > self.window {
                states.insert(
                    key.to_string(),
                    RateLimitState {
                        count: AtomicU64::new(1),
                        window_start: now,
                    },
                );
                false
            } else {
                let count = state.count.fetch_add(1, Ordering::Relaxed);
                count >= self.max_requests + self.burst
            }
        } else {
            states.insert(
                key.to_string(),
                RateLimitState {
                    count: AtomicU64::new(1),
                    window_start: now,
                },
            );
            false
        }
    }

    fn cleanup_expired_entries(
        &self,
        states: &mut HashMap<String, RateLimitState>,
        now: Instant,
    ) {
        let window = self.window;
        let initial_len = states.len();
        
        states.retain(|_, state| now.duration_since(state.window_start) <= window);
        
        let removed = initial_len - states.len();
        if removed > 0 {
            debug!("Rate limiter cleanup: removed {} expired entries", removed);
        }
        
        if states.len() > CLEANUP_THRESHOLD {
            let mut entries: Vec<_> = states.iter()
                .map(|(k, v)| (k.clone(), v.window_start))
                .collect();
            entries.sort_by_key(|(_, start)| *start);
            
            let to_remove = states.len() - CLEANUP_THRESHOLD;
            for (key, _) in entries.into_iter().take(to_remove) {
                states.remove(&key);
            }
            debug!("Rate limiter cleanup: evicted {} oldest entries", to_remove);
        }
    }
}

#[async_trait]
impl Plugin for RateLimitPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            id: "hive.proxy.rate-limit".to_string(),
            name: "Rate Limit".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            plugin_type: PluginType::Orchestration,
            author: Some("ADI Team".to_string()),
            description: Some("Request rate limiting middleware".to_string()),
            category: Some(PluginCategory::Proxy),
        }
    }

    async fn init(&mut self, ctx: &PluginContext) -> PluginResult<()> {
        let config = &ctx.config;

        if let Some(requests) = config.get("requests").and_then(|v| v.as_u64()) {
            self.max_requests = requests;
        }

        if let Some(window) = config.get("window").and_then(|v| v.as_str()) {
            self.window = parse_duration(window).unwrap_or(Duration::from_secs(60));
        }

        if let Some(burst) = config.get("burst").and_then(|v| v.as_u64()) {
            self.burst = burst;
        }

        if let Some(by) = config.get("by").and_then(|v| v.as_str()) {
            self.by = match by {
                "ip" => RateLimitBy::Ip,
                "path" => RateLimitBy::Path,
                _ if by.starts_with("header:") => {
                    self.header_name = Some(by.strip_prefix("header:").unwrap().to_string());
                    RateLimitBy::Header
                }
                _ => RateLimitBy::Ip,
            };
        }

        Ok(())
    }

    async fn shutdown(&self) -> PluginResult<()> {
        Ok(())
    }

    fn provides(&self) -> Vec<&'static str> {
        vec![SERVICE_PROXY_MIDDLEWARE]
    }
}

#[async_trait]
impl ProxyMiddleware for RateLimitPlugin {
    async fn process_request(&self, req: ProxyRequest) -> PluginResult<ProxyResult> {
        let key = self.get_key(&req);

        if self.is_rate_limited(&key).await {
            debug!("Rate limited: {}", key);

            let response = ProxyResponse::too_many_requests()
                .with_header("retry-after", self.window.as_secs().to_string());

            return Ok(ProxyResult::Response(response));
        }

        Ok(ProxyResult::Continue(req))
    }

    async fn process_response(&self, resp: ProxyResponse) -> PluginResult<ProxyResponse> {
        Ok(resp)
    }
}

#[cfg(feature = "plugin")]
#[no_mangle]
pub fn plugin_create() -> Box<dyn Plugin> {
    Box::new(RateLimitPlugin::new())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_rate_limiting() {
        let mut plugin = RateLimitPlugin::new();
        let ctx = PluginContext::new(
            "hive.proxy.rate-limit",
            PathBuf::from("/tmp"),
            PathBuf::from("/tmp"),
            serde_json::json!({
                "requests": 2,
                "window": "60s",
                "burst": 0
            }),
        );
        plugin.init(&ctx).await.unwrap();

        // First two requests should pass
        assert!(!plugin.is_rate_limited("test-key").await);
        assert!(!plugin.is_rate_limited("test-key").await);

        // Third should be limited
        assert!(plugin.is_rate_limited("test-key").await);
    }
}
