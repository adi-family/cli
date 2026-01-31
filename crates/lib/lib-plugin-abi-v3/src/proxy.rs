//! Proxy middleware plugin trait

use crate::{Plugin, Result};
use async_trait::async_trait;
use axum::http::StatusCode;
use bytes::Bytes;
use serde_json::Value;
use std::collections::HashMap;

/// Proxy middleware plugin trait
///
/// Proxy middleware plugins can intercept and modify HTTP requests/responses
/// (CORS, rate limiting, authentication, etc.).
#[async_trait]
pub trait ProxyMiddleware: Plugin {
    /// Initialize middleware with configuration
    async fn init_middleware(&mut self, config: &Value) -> Result<()> {
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
    /// HTTP method
    pub method: crate::http::HttpMethod,

    /// Request path
    pub path: String,

    /// HTTP headers
    pub headers: HashMap<String, String>,

    /// Request body
    pub body: Bytes,
}

/// Proxy response
#[derive(Debug, Clone)]
pub struct ProxyResponse {
    /// HTTP status code
    pub status: StatusCode,

    /// Response headers
    pub headers: HashMap<String, String>,

    /// Response body
    pub body: Bytes,
}
