//! Proxy middleware plugin trait

use crate::{Plugin, Result};
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;

/// Proxy middleware plugin trait
///
/// Proxy middleware plugins can intercept and modify HTTP requests/responses
/// (CORS, rate limiting, authentication, etc.).
#[async_trait]
pub trait ProxyMiddleware: Plugin {
    /// Initialize middleware with configuration
    async fn init_middleware(&mut self, _config: &Value) -> Result<()> {
        Ok(())
    }

    /// Process an incoming request
    async fn process_request(&self, req: ProxyRequest) -> Result<ProxyResult>;

    /// Process an outgoing response
    async fn process_response(&self, resp: ProxyResponse) -> Result<ProxyResponse>;
}

/// Proxy middleware result
#[derive(Debug)]
pub enum ProxyResult {
    /// Continue to next middleware
    Continue(ProxyRequest),

    /// Short-circuit with response
    Response(ProxyResponse),
}

/// Proxy request
#[derive(Debug, Clone)]
pub struct ProxyRequest {
    /// HTTP method (e.g., "GET", "POST")
    pub method: String,

    /// Request URI (e.g., "/api/users?limit=10")
    pub uri: String,

    /// HTTP headers
    pub headers: HashMap<String, String>,

    /// Client IP address
    pub client_ip: Option<String>,

    /// Request body
    pub body: Option<Vec<u8>>,
}

impl ProxyRequest {
    /// Get a header value (case-insensitive)
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.get(name).map(|s| s.as_str())
    }

    /// Check if request has a specific header
    pub fn has_header(&self, name: &str) -> bool {
        self.headers.contains_key(name)
    }
}

/// Proxy response
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
    /// Create a new response with status code
    pub fn new(status: u16) -> Self {
        Self {
            status,
            headers: HashMap::new(),
            body: Vec::new(),
        }
    }

    /// Create a 204 No Content response (useful for CORS preflight)
    pub fn no_content() -> Self {
        Self::new(204)
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
    pub fn json<T: serde::Serialize>(status: u16, data: &T) -> Result<Self> {
        let body = serde_json::to_vec(data)?;
        Ok(Self {
            status,
            headers: [("content-type".to_string(), "application/json".to_string())]
                .into_iter()
                .collect(),
            body,
        })
    }
}
