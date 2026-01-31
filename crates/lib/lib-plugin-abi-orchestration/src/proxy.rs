//! Proxy Middleware Plugin Trait
//!
//! Proxy plugins process HTTP requests before/after they reach the backend.
//! Examples: cors, rate-limit, headers, ip-filter, auth, etc.

use anyhow::Result;
use async_trait::async_trait;
use serde::Serialize;
use std::collections::HashMap;

/// Trait for proxy middleware plugins
#[async_trait]
pub trait ProxyPlugin: Send + Sync {
    /// Get plugin metadata
    fn metadata(&self) -> crate::PluginMetadata;

    /// Initialize the plugin with configuration
    async fn init(&mut self, config: &serde_json::Value) -> Result<()>;

    /// Process an incoming request
    /// Returns Continue to pass to next middleware, or Response to return early
    async fn process_request(&self, req: ProxyRequest) -> Result<ProxyResult>;

    /// Process an outgoing response (optional)
    async fn process_response(&self, _resp: ProxyResponse) -> Result<ProxyResponse> {
        Ok(_resp)
    }

    /// Shutdown the plugin
    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }
}

/// Simplified HTTP request for plugin processing
#[derive(Debug, Clone)]
pub struct ProxyRequest {
    /// HTTP method
    pub method: String,
    /// Request URI
    pub uri: String,
    /// Request headers
    pub headers: HashMap<String, String>,
    /// Client IP address
    pub client_ip: Option<String>,
    /// Request body (if small enough to buffer)
    pub body: Option<Vec<u8>>,
}

impl ProxyRequest {
    /// Get a header value
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.get(name).map(|s| s.as_str())
    }

    /// Check if request has a specific header
    pub fn has_header(&self, name: &str) -> bool {
        self.headers.contains_key(name)
    }
}

/// Simplified HTTP response for plugin processing
#[derive(Debug, Clone)]
pub struct ProxyResponse {
    /// HTTP status code
    pub status: u16,
    /// Response headers
    pub headers: HashMap<String, String>,
    /// Response body
    pub body: Vec<u8>,
}

impl ProxyResponse {
    /// Create a new response
    pub fn new(status: u16) -> Self {
        Self {
            status,
            headers: HashMap::new(),
            body: Vec::new(),
        }
    }

    /// Add a header
    pub fn with_header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(name.into(), value.into());
        self
    }

    /// Set the body
    pub fn with_body(mut self, body: impl Into<Vec<u8>>) -> Self {
        self.body = body.into();
        self
    }

    /// Create a JSON response
    pub fn json(status: u16, body: &impl Serialize) -> Result<Self> {
        let body = serde_json::to_vec(body)?;
        Ok(Self {
            status,
            headers: [("content-type".to_string(), "application/json".to_string())]
                .into_iter()
                .collect(),
            body,
        })
    }

    /// Create a 403 Forbidden response
    pub fn forbidden() -> Self {
        Self::new(403).with_body("Forbidden")
    }

    /// Create a 429 Too Many Requests response
    pub fn too_many_requests() -> Self {
        Self::new(429)
            .with_header("content-type", "application/json")
            .with_body(r#"{"error": "rate limited"}"#)
    }

    /// Create a 204 No Content response (for CORS preflight)
    pub fn no_content() -> Self {
        Self::new(204)
    }
}

/// Result of proxy middleware processing
#[derive(Debug)]
pub enum ProxyResult {
    /// Continue to next middleware/backend with modified request
    Continue(ProxyRequest),
    /// Return early with a response
    Response(ProxyResponse),
}
