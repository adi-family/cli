//! HTTP routes service trait

use crate::{Plugin, Result};
use async_trait::async_trait;
use axum::http::StatusCode;
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// HTTP routes service trait
///
/// Plugins implementing this trait can provide HTTP endpoints that will be
/// mounted into the host's HTTP server.
///
/// # Example
///
/// ```rust
/// use lib_plugin_abi_v3::*;
///
/// pub struct ApiPlugin;
///
/// #[async_trait]
/// impl HttpRoutes for ApiPlugin {
///     async fn list_routes(&self) -> Vec<HttpRoute> {
///         vec![
///             HttpRoute {
///                 method: HttpMethod::Get,
///                 path: "/api/tasks".to_string(),
///                 handler_id: "list_tasks".to_string(),
///                 description: "List all tasks".to_string(),
///             },
///             HttpRoute {
///                 method: HttpMethod::Post,
///                 path: "/api/tasks".to_string(),
///                 handler_id: "create_task".to_string(),
///                 description: "Create a new task".to_string(),
///             },
///         ]
///     }
///
///     async fn handle_request(&self, req: HttpRequest) -> Result<HttpResponse> {
///         match req.handler_id.as_str() {
///             "list_tasks" => {
///                 let tasks = vec![/* ... */];
///                 HttpResponse::json(&tasks)
///             }
///             "create_task" => {
///                 let task = serde_json::from_slice(&req.body)?;
///                 // Create task logic
///                 HttpResponse::json(&task)
///             }
///             _ => Ok(HttpResponse::error(StatusCode::NOT_FOUND, "Unknown handler")),
///         }
///     }
/// }
/// ```
#[async_trait]
pub trait HttpRoutes: Plugin {
    /// List all HTTP routes provided by this plugin
    async fn list_routes(&self) -> Vec<HttpRoute>;

    /// Handle an HTTP request
    async fn handle_request(&self, req: HttpRequest) -> Result<HttpResponse>;
}

/// HTTP route metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpRoute {
    /// HTTP method
    pub method: HttpMethod,

    /// Route path (e.g., "/api/tasks")
    pub path: String,

    /// Internal handler identifier
    pub handler_id: String,

    /// Human-readable description
    pub description: String,
}

/// HTTP method
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Delete,
    Patch,
    Head,
    Options,
    Trace,
}

impl HttpMethod {
    /// Convert to axum::http::Method
    pub fn to_axum_method(&self) -> axum::http::Method {
        match self {
            HttpMethod::Get => axum::http::Method::GET,
            HttpMethod::Post => axum::http::Method::POST,
            HttpMethod::Put => axum::http::Method::PUT,
            HttpMethod::Delete => axum::http::Method::DELETE,
            HttpMethod::Patch => axum::http::Method::PATCH,
            HttpMethod::Head => axum::http::Method::HEAD,
            HttpMethod::Options => axum::http::Method::OPTIONS,
            HttpMethod::Trace => axum::http::Method::TRACE,
        }
    }

    /// Convert from axum::http::Method
    pub fn from_axum_method(method: &axum::http::Method) -> Option<Self> {
        match method {
            &axum::http::Method::GET => Some(HttpMethod::Get),
            &axum::http::Method::POST => Some(HttpMethod::Post),
            &axum::http::Method::PUT => Some(HttpMethod::Put),
            &axum::http::Method::DELETE => Some(HttpMethod::Delete),
            &axum::http::Method::PATCH => Some(HttpMethod::Patch),
            &axum::http::Method::HEAD => Some(HttpMethod::Head),
            &axum::http::Method::OPTIONS => Some(HttpMethod::Options),
            &axum::http::Method::TRACE => Some(HttpMethod::Trace),
            _ => None,
        }
    }
}

/// HTTP request
#[derive(Debug, Clone)]
pub struct HttpRequest {
    /// HTTP method
    pub method: HttpMethod,

    /// Request path
    pub path: String,

    /// Handler ID (from route definition)
    pub handler_id: String,

    /// Query parameters
    pub query: HashMap<String, String>,

    /// HTTP headers
    pub headers: HashMap<String, String>,

    /// Request body
    pub body: Bytes,

    /// Path parameters (e.g., /api/tasks/:id -> {"id": "123"})
    pub params: HashMap<String, String>,
}

impl HttpRequest {
    /// Parse body as JSON
    pub fn json<T>(&self) -> Result<T>
    where
        T: serde::de::DeserializeOwned,
    {
        Ok(serde_json::from_slice(&self.body)?)
    }

    /// Get body as string
    pub fn text(&self) -> Result<String> {
        Ok(String::from_utf8(self.body.to_vec())?)
    }

    /// Get a header value
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.get(name).map(|s| s.as_str())
    }

    /// Get a query parameter
    pub fn query_param(&self, name: &str) -> Option<&str> {
        self.query.get(name).map(|s| s.as_str())
    }

    /// Get a path parameter
    pub fn path_param(&self, name: &str) -> Option<&str> {
        self.params.get(name).map(|s| s.as_str())
    }
}

/// HTTP response
#[derive(Debug, Clone)]
pub struct HttpResponse {
    /// HTTP status code
    pub status: StatusCode,

    /// Response headers
    pub headers: HashMap<String, String>,

    /// Response body
    pub body: Bytes,
}

impl HttpResponse {
    /// Create a successful response with body
    pub fn ok(body: impl Into<Bytes>) -> Self {
        Self {
            status: StatusCode::OK,
            headers: HashMap::new(),
            body: body.into(),
        }
    }

    /// Create a JSON response
    pub fn json<T: serde::Serialize>(data: &T) -> Result<Self> {
        let body = serde_json::to_vec(data)?;
        Ok(Self {
            status: StatusCode::OK,
            headers: [(
                "content-type".to_string(),
                "application/json".to_string(),
            )]
            .into(),
            body: body.into(),
        })
    }

    /// Create an error response
    pub fn error(status: StatusCode, message: impl Into<String>) -> Self {
        Self {
            status,
            headers: HashMap::new(),
            body: message.into().into(),
        }
    }

    /// Create a custom response
    pub fn custom(status: StatusCode, headers: HashMap<String, String>, body: impl Into<Bytes>) -> Self {
        Self {
            status,
            headers,
            body: body.into(),
        }
    }

    /// Add a header
    pub fn with_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(key.into(), value.into());
        self
    }

    /// Set status code
    pub fn with_status(mut self, status: StatusCode) -> Self {
        self.status = status;
        self
    }
}