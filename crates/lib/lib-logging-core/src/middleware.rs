//! Axum middleware for automatic trace context propagation.
//!
//! This middleware:
//! 1. Extracts trace context from incoming request headers
//! 2. Creates a new span for the request
//! 3. Makes the context available via request extensions
//! 4. Adds trace headers to responses

// Use the appropriate axum version based on features
#[cfg(feature = "axum")]
use axum_07 as axum;
#[cfg(feature = "axum-08-compat")]
use axum_08 as axum;

#[cfg(feature = "axum")]
use tower_04 as tower;
#[cfg(feature = "axum-08-compat")]
use tower_05 as tower;

use axum::{
    extract::Request,
    response::Response,
};
use http::HeaderValue;
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};
use tower::{Layer, Service};
use uuid::Uuid;

use crate::{TraceContext, TRACE_ID_HEADER, SPAN_ID_HEADER, PARENT_SPAN_ID_HEADER};

/// Configuration for the trace layer.
#[derive(Clone)]
pub struct TraceLayerConfig {
    /// Whether to add trace headers to responses
    pub add_response_headers: bool,
}

impl Default for TraceLayerConfig {
    fn default() -> Self {
        Self {
            add_response_headers: true,
        }
    }
}

/// Layer that adds trace context to requests.
#[derive(Clone)]
pub struct TraceLayer {
    config: TraceLayerConfig,
}

impl TraceLayer {
    /// Create a new trace layer with default config.
    pub fn new() -> Self {
        Self {
            config: TraceLayerConfig::default(),
        }
    }

    /// Create with custom config.
    pub fn with_config(config: TraceLayerConfig) -> Self {
        Self { config }
    }
}

impl Default for TraceLayer {
    fn default() -> Self {
        Self::new()
    }
}

impl<S> Layer<S> for TraceLayer {
    type Service = TraceService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        TraceService {
            inner,
            config: self.config.clone(),
        }
    }
}

/// Service that extracts/injects trace context.
#[derive(Clone)]
pub struct TraceService<S> {
    inner: S,
    config: TraceLayerConfig,
}

impl<S, ResBody> Service<Request> for TraceService<S>
where
    S: Service<Request, Response = Response<ResBody>> + Clone + Send + 'static,
    S::Future: Send,
    ResBody: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut req: Request) -> Self::Future {
        let config = self.config.clone();

        // Extract trace context from headers
        let trace_id = req
            .headers()
            .get(TRACE_ID_HEADER)
            .and_then(|v| v.to_str().ok())
            .and_then(|s| Uuid::parse_str(s).ok());

        let parent_span_id = req
            .headers()
            .get(PARENT_SPAN_ID_HEADER)
            .and_then(|v| v.to_str().ok())
            .and_then(|s| Uuid::parse_str(s).ok());

        // Create trace context (new or continued)
        let ctx = TraceContext::from_headers(trace_id, parent_span_id);

        // Store in request extensions
        req.extensions_mut().insert(ctx.clone());

        let future = self.inner.call(req);

        Box::pin(async move {
            let mut response = future.await?;

            // Add trace headers to response
            if config.add_response_headers {
                let headers = response.headers_mut();
                if let Ok(v) = HeaderValue::from_str(&ctx.trace_id.to_string()) {
                    headers.insert(TRACE_ID_HEADER, v);
                }
                if let Ok(v) = HeaderValue::from_str(&ctx.span_id.to_string()) {
                    headers.insert(SPAN_ID_HEADER, v);
                }
            }

            Ok(response)
        })
    }
}

/// Extension trait to extract trace context from axum requests.
pub trait TraceContextExt {
    /// Get the trace context from request extensions.
    fn trace_context(&self) -> TraceContext;
}

impl<B> TraceContextExt for axum::extract::Request<B> {
    fn trace_context(&self) -> TraceContext {
        self.extensions()
            .get::<TraceContext>()
            .cloned()
            .unwrap_or_else(TraceContext::new)
    }
}

impl TraceContextExt for http::request::Parts {
    fn trace_context(&self) -> TraceContext {
        self.extensions
            .get::<TraceContext>()
            .cloned()
            .unwrap_or_else(TraceContext::new)
    }
}

/// Create a trace layer with default configuration.
pub fn trace_layer() -> TraceLayer {
    TraceLayer::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trace_layer_creation() {
        let _layer = TraceLayer::new();
    }
}
