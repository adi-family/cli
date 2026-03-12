//! Proxy Middleware Plugins
//!
//! Provides a plugin system for proxy middleware including:
//! - CORS
//! - Rate limiting
//! - Headers manipulation
//! - IP filtering

use anyhow::Result;
use async_trait::async_trait;
use axum::{
    body::Body,
    extract::Request,
    http::{header, HeaderValue, Method, Response, StatusCode},
};
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, trace, warn};

/// Trait for proxy middleware plugins
#[async_trait]
pub trait ProxyMiddleware: Send + Sync {
    fn name(&self) -> &str;

    /// Process a request, optionally returning an early response
    async fn process(&self, req: Request) -> Result<ProxyMiddlewareResult>;
}

/// Result of middleware processing
pub enum ProxyMiddlewareResult {
    /// Continue to the next middleware/backend
    Continue(Request),
    /// Return early with a response
    Response(Response<Body>),
}

/// CORS middleware plugin
pub struct CorsMiddleware {
    /// Allowed origins (* for all)
    allowed_origins: Vec<String>,
    allowed_methods: Vec<Method>,
    allowed_headers: Vec<String>,
    exposed_headers: Vec<String>,
    allow_credentials: bool,
    /// Preflight cache max age in seconds
    max_age: u32,
}

impl Default for CorsMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

impl CorsMiddleware {
    pub fn new() -> Self {
        Self {
            allowed_origins: vec!["*".to_string()],
            allowed_methods: vec![
                Method::GET,
                Method::POST,
                Method::PUT,
                Method::DELETE,
                Method::PATCH,
                Method::OPTIONS,
            ],
            allowed_headers: vec!["Content-Type".to_string(), "Authorization".to_string()],
            exposed_headers: vec![],
            allow_credentials: false,
            max_age: 86400, // 24 hours
        }
    }

    pub fn from_config(config: &serde_json::Value) -> Self {
        let mut cors = Self::new();

        if let Some(origins) = config.get("origins").and_then(|v| v.as_array()) {
            cors.allowed_origins = origins
                .iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect();
        }

        if let Some(methods) = config.get("methods").and_then(|v| v.as_array()) {
            cors.allowed_methods = methods
                .iter()
                .filter_map(|v| v.as_str())
                .filter_map(|s| s.parse().ok())
                .collect();
        }

        if let Some(headers) = config.get("headers").and_then(|v| v.as_array()) {
            cors.allowed_headers = headers
                .iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect();
        }

        if let Some(expose) = config.get("expose_headers").and_then(|v| v.as_array()) {
            cors.exposed_headers = expose
                .iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect();
        }

        if let Some(creds) = config.get("credentials").and_then(|v| v.as_bool()) {
            cors.allow_credentials = creds;
        }

        if let Some(age) = config.get("max_age").and_then(|v| v.as_u64()) {
            cors.max_age = age as u32;
        }

        cors
    }

    fn is_origin_allowed(&self, origin: &str) -> bool {
        self.allowed_origins.iter().any(|o| o == "*" || o == origin)
    }
}

#[async_trait]
impl ProxyMiddleware for CorsMiddleware {
    fn name(&self) -> &str {
        "cors"
    }

    async fn process(&self, req: Request) -> Result<ProxyMiddlewareResult> {
        let origin = req
            .headers()
            .get(header::ORIGIN)
            .and_then(|v| v.to_str().ok())
            .map(String::from);

        // Check if this is a preflight request
        if req.method() == Method::OPTIONS {
            if let Some(ref origin) = origin {
                if self.is_origin_allowed(origin) {
                    trace!(origin = %origin, "CORS preflight approved");
                    let mut response = Response::new(Body::empty());
                    *response.status_mut() = StatusCode::NO_CONTENT;

                    let headers = response.headers_mut();
                    headers.insert(
                        header::ACCESS_CONTROL_ALLOW_ORIGIN,
                        HeaderValue::from_str(origin).unwrap_or_else(|_| HeaderValue::from_static("*")),
                    );
                    headers.insert(
                        header::ACCESS_CONTROL_ALLOW_METHODS,
                        HeaderValue::from_str(
                            &self
                                .allowed_methods
                                .iter()
                                .map(|m| m.as_str())
                                .collect::<Vec<_>>()
                                .join(", "),
                        )
                        .unwrap_or_else(|_| HeaderValue::from_static("GET, POST")),
                    );
                    headers.insert(
                        header::ACCESS_CONTROL_ALLOW_HEADERS,
                        HeaderValue::from_str(&self.allowed_headers.join(", "))
                            .unwrap_or_else(|_| HeaderValue::from_static("Content-Type")),
                    );
                    headers.insert(
                        header::ACCESS_CONTROL_MAX_AGE,
                        HeaderValue::from(self.max_age),
                    );

                    if self.allow_credentials {
                        headers.insert(
                            header::ACCESS_CONTROL_ALLOW_CREDENTIALS,
                            HeaderValue::from_static("true"),
                        );
                    }

                    return Ok(ProxyMiddlewareResult::Response(response));
                }
            }
        }

        // For regular requests, we continue and add headers in the response
        // This is a simplified version - a full implementation would wrap the response
        Ok(ProxyMiddlewareResult::Continue(req))
    }
}

/// Rate limit state for an IP/key
struct RateLimitState {
    count: AtomicU64,
    window_start: Instant,
}

/// Rate limiting middleware plugin
pub struct RateLimitMiddleware {
    max_requests: u64,
    window: Duration,
    /// Burst allowance above max_requests
    burst: u64,
    /// Rate limit by (ip, header, path)
    by: RateLimitBy,
    /// Header name for header-based rate limiting
    header_name: Option<String>,
    states: Arc<RwLock<HashMap<String, RateLimitState>>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RateLimitBy {
    Ip,
    Header,
    Path,
}

impl Default for RateLimitMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

impl RateLimitMiddleware {
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

    pub fn from_config(config: &serde_json::Value) -> Self {
        let mut rl = Self::new();

        if let Some(requests) = config.get("requests").and_then(|v| v.as_u64()) {
            rl.max_requests = requests;
        }

        if let Some(window) = config.get("window").and_then(|v| v.as_str()) {
            rl.window = crate::service_manager::parse_duration(window).unwrap_or(Duration::from_secs(60));
        }

        if let Some(burst) = config.get("burst").and_then(|v| v.as_u64()) {
            rl.burst = burst;
        }

        if let Some(by) = config.get("by").and_then(|v| v.as_str()) {
            rl.by = match by {
                "ip" => RateLimitBy::Ip,
                "path" => RateLimitBy::Path,
                _ if by.starts_with("header:") => {
                    rl.header_name = Some(by.strip_prefix("header:").unwrap().to_string());
                    RateLimitBy::Header
                }
                _ => RateLimitBy::Ip,
            };
        }

        rl
    }

    fn get_key(&self, req: &Request) -> String {
        match self.by {
            RateLimitBy::Ip => {
                // Try to get IP from X-Forwarded-For, then from X-Real-IP, then from connection
                req.headers()
                    .get("x-forwarded-for")
                    .and_then(|v| v.to_str().ok())
                    .map(|v| v.split(',').next().unwrap_or(v).trim().to_string())
                    .or_else(|| {
                        req.headers()
                            .get("x-real-ip")
                            .and_then(|v| v.to_str().ok())
                            .map(String::from)
                    })
                    .unwrap_or_else(|| "unknown".to_string())
            }
            RateLimitBy::Header => {
                self.header_name
                    .as_ref()
                    .and_then(|name| {
                        req.headers()
                            .get(name)
                            .and_then(|v| v.to_str().ok())
                            .map(String::from)
                    })
                    .unwrap_or_else(|| "unknown".to_string())
            }
            RateLimitBy::Path => req.uri().path().to_string(),
        }
    }

    async fn is_rate_limited(&self, key: &str) -> bool {
        let mut states = self.states.write().await;
        let now = Instant::now();

        if let Some(state) = states.get(key) {
            if now.duration_since(state.window_start) > self.window {
                // Window expired, reset
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
}

#[async_trait]
impl ProxyMiddleware for RateLimitMiddleware {
    fn name(&self) -> &str {
        "rate-limit"
    }

    async fn process(&self, req: Request) -> Result<ProxyMiddlewareResult> {
        let key = self.get_key(&req);

        if self.is_rate_limited(&key).await {
            debug!("Rate limited: {}", key);
            
            let mut response = Response::new(Body::from(r#"{"error": "rate limited"}"#));
            *response.status_mut() = StatusCode::TOO_MANY_REQUESTS;
            response.headers_mut().insert(
                header::CONTENT_TYPE,
                HeaderValue::from_static("application/json"),
            );
            response.headers_mut().insert(
                header::RETRY_AFTER,
                HeaderValue::from(self.window.as_secs()),
            );

            return Ok(ProxyMiddlewareResult::Response(response));
        }

        Ok(ProxyMiddlewareResult::Continue(req))
    }
}

/// Headers manipulation middleware
pub struct HeadersMiddleware {
    /// Headers to add (only if not already present)
    add: HashMap<String, String>,
    remove: Vec<String>,
    /// Headers to set (overwrite existing)
    set: HashMap<String, String>,
}

impl Default for HeadersMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

impl HeadersMiddleware {
    pub fn new() -> Self {
        Self {
            add: HashMap::new(),
            remove: Vec::new(),
            set: HashMap::new(),
        }
    }

    pub fn from_config(config: &serde_json::Value) -> Self {
        let mut headers = Self::new();

        if let Some(add) = config.get("add").and_then(|v| v.as_object()) {
            for (k, v) in add {
                if let Some(v) = v.as_str() {
                    headers.add.insert(k.clone(), v.to_string());
                }
            }
        }

        if let Some(remove) = config.get("remove").and_then(|v| v.as_array()) {
            headers.remove = remove
                .iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect();
        }

        if let Some(set) = config.get("set").and_then(|v| v.as_object()) {
            for (k, v) in set {
                if let Some(v) = v.as_str() {
                    headers.set.insert(k.clone(), v.to_string());
                }
            }
        }

        headers
    }
}

#[async_trait]
impl ProxyMiddleware for HeadersMiddleware {
    fn name(&self) -> &str {
        "headers"
    }

    async fn process(&self, req: Request) -> Result<ProxyMiddlewareResult> {
        let (mut parts, body) = req.into_parts();

        // Remove headers
        for name in &self.remove {
            if let Ok(header_name) = name.parse::<header::HeaderName>() {
                trace!(header = %name, "Removing header");
                parts.headers.remove(header_name);
            }
        }

        // Add headers (only if not present)
        for (name, value) in &self.add {
            if let (Ok(header_name), Ok(header_value)) = (
                name.parse::<header::HeaderName>(),
                HeaderValue::from_str(value),
            ) {
                if !parts.headers.contains_key(&header_name) {
                    parts.headers.insert(header_name, header_value);
                }
            }
        }

        // Set headers (overwrite)
        for (name, value) in &self.set {
            if let (Ok(header_name), Ok(header_value)) = (
                name.parse::<header::HeaderName>(),
                HeaderValue::from_str(value),
            ) {
                parts.headers.insert(header_name, header_value);
            }
        }

        Ok(ProxyMiddlewareResult::Continue(Request::from_parts(parts, body)))
    }
}

/// IP filter middleware
pub struct IpFilterMiddleware {
    allow: Vec<IpRange>,
    deny: Vec<IpRange>,
    /// When true, use X-Forwarded-For for the client IP instead of X-Real-IP
    trust_xff: bool,
}

/// IP range (single IP or CIDR)
#[derive(Debug, Clone)]
pub struct IpRange {
    addr: IpAddr,
    prefix_len: Option<u8>,
}

impl IpRange {
    pub fn parse(s: &str) -> Option<Self> {
        if let Some((addr, prefix)) = s.split_once('/') {
            let addr: IpAddr = addr.parse().ok()?;
            let prefix_len: u8 = prefix.parse().ok()?;
            Some(IpRange {
                addr,
                prefix_len: Some(prefix_len),
            })
        } else {
            let addr: IpAddr = s.parse().ok()?;
            Some(IpRange {
                addr,
                prefix_len: None,
            })
        }
    }

    pub fn contains(&self, ip: IpAddr) -> bool {
        match (self.addr, ip) {
            (IpAddr::V4(range), IpAddr::V4(target)) => {
                if let Some(prefix_len) = self.prefix_len {
                    let mask = !0u32 << (32 - prefix_len);
                    (u32::from(range) & mask) == (u32::from(target) & mask)
                } else {
                    range == target
                }
            }
            (IpAddr::V6(range), IpAddr::V6(target)) => {
                if let Some(prefix_len) = self.prefix_len {
                    let range_bits = u128::from(range);
                    let target_bits = u128::from(target);
                    let mask = !0u128 << (128 - prefix_len);
                    (range_bits & mask) == (target_bits & mask)
                } else {
                    range == target
                }
            }
            _ => false,
        }
    }
}

impl Default for IpFilterMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

impl IpFilterMiddleware {
    pub fn new() -> Self {
        Self {
            allow: Vec::new(),
            deny: Vec::new(),
            trust_xff: false,
        }
    }

    pub fn from_config(config: &serde_json::Value) -> Self {
        let mut filter = Self::new();

        if let Some(allow) = config.get("allow").and_then(|v| v.as_array()) {
            filter.allow = allow
                .iter()
                .filter_map(|v| v.as_str())
                .filter_map(IpRange::parse)
                .collect();
        }

        if let Some(deny) = config.get("deny").and_then(|v| v.as_array()) {
            filter.deny = deny
                .iter()
                .filter_map(|v| v.as_str())
                .filter_map(IpRange::parse)
                .collect();
        }

        if let Some(trust) = config.get("trust_xff").and_then(|v| v.as_bool()) {
            filter.trust_xff = trust;
        }

        filter
    }

    fn get_client_ip(&self, req: &Request) -> Option<IpAddr> {
        if self.trust_xff {
            if let Some(xff) = req.headers().get("x-forwarded-for") {
                if let Ok(xff_str) = xff.to_str() {
                    if let Some(first_ip) = xff_str.split(',').next() {
                        if let Ok(ip) = first_ip.trim().parse() {
                            return Some(ip);
                        }
                    }
                }
            }
        }

        req.headers()
            .get("x-real-ip")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse().ok())
    }

    fn is_allowed(&self, ip: IpAddr) -> bool {
        // Check deny list first
        if self.deny.iter().any(|range| range.contains(ip)) {
            return false;
        }

        // If allow list is empty, allow all (that aren't denied)
        if self.allow.is_empty() {
            return true;
        }

        // Check allow list
        self.allow.iter().any(|range| range.contains(ip))
    }
}

#[async_trait]
impl ProxyMiddleware for IpFilterMiddleware {
    fn name(&self) -> &str {
        "ip-filter"
    }

    async fn process(&self, req: Request) -> Result<ProxyMiddlewareResult> {
        if let Some(ip) = self.get_client_ip(&req) {
            if !self.is_allowed(ip) {
                debug!("IP blocked: {}", ip);
                
                let mut response = Response::new(Body::from("Forbidden"));
                *response.status_mut() = StatusCode::FORBIDDEN;

                return Ok(ProxyMiddlewareResult::Response(response));
            }
        }

        Ok(ProxyMiddlewareResult::Continue(req))
    }
}

/// Middleware chain runner
pub struct MiddlewareChain {
    middlewares: Vec<Box<dyn ProxyMiddleware>>,
}

impl Default for MiddlewareChain {
    fn default() -> Self {
        Self::new()
    }
}

impl MiddlewareChain {
    pub fn new() -> Self {
        Self {
            middlewares: Vec::new(),
        }
    }

    pub fn add(&mut self, middleware: Box<dyn ProxyMiddleware>) {
        self.middlewares.push(middleware);
    }

    pub fn from_config(plugins: &[crate::hive_config::ProxyPluginConfig]) -> Self {
        let mut chain = Self::new();

        for plugin in plugins {
            let config = plugin
                .config
                .get(&plugin.plugin_type)
                .cloned()
                .unwrap_or_default();

            debug!(plugin_type = %plugin.plugin_type, "Adding proxy middleware to chain");
            match plugin.plugin_type.as_str() {
                "cors" => chain.add(Box::new(CorsMiddleware::from_config(&config))),
                "rate-limit" => chain.add(Box::new(RateLimitMiddleware::from_config(&config))),
                "headers" => chain.add(Box::new(HeadersMiddleware::from_config(&config))),
                "ip-filter" => chain.add(Box::new(IpFilterMiddleware::from_config(&config))),
                other => {
                    warn!("Unknown proxy plugin type: {}", other);
                }
            }
        }

        debug!(middleware_count = chain.middlewares.len(), "Middleware chain built");
        chain
    }

    pub async fn process(&self, mut req: Request) -> Result<ProxyMiddlewareResult> {
        let path = req.uri().path().to_string();
        trace!(path = %path, middlewares = self.middlewares.len(), "Processing middleware chain");

        for middleware in &self.middlewares {
            match middleware.process(req).await? {
                ProxyMiddlewareResult::Continue(r) => {
                    trace!(middleware = middleware.name(), path = %path, "Middleware passed");
                    req = r;
                }
                ProxyMiddlewareResult::Response(response) => {
                    debug!(
                        middleware = middleware.name(),
                        path = %path,
                        status = %response.status(),
                        "Middleware short-circuited with response"
                    );
                    return Ok(ProxyMiddlewareResult::Response(response));
                }
            }
        }

        Ok(ProxyMiddlewareResult::Continue(req))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ip_range_parse() {
        let range = IpRange::parse("10.0.0.0/8").unwrap();
        assert!(range.contains("10.1.2.3".parse().unwrap()));
        assert!(!range.contains("11.0.0.1".parse().unwrap()));

        let single = IpRange::parse("192.168.1.1").unwrap();
        assert!(single.contains("192.168.1.1".parse().unwrap()));
        assert!(!single.contains("192.168.1.2".parse().unwrap()));
    }

    #[test]
    fn test_cors_config() {
        let config = serde_json::json!({
            "origins": ["https://example.com"],
            "methods": ["GET", "POST"],
            "credentials": true
        });

        let cors = CorsMiddleware::from_config(&config);
        assert!(cors.is_origin_allowed("https://example.com"));
        assert!(!cors.is_origin_allowed("https://other.com"));
        assert!(cors.allow_credentials);
    }
}
