//! Version header middleware for HTTP responses.
//!
//! When enabled via `SHOW_VERSION_IN_HEADERS=true`, adds headers:
//! - `X-Service-Name`: The service name
//! - `X-Service-Version`: The service version

use http::{Request, Response};
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};
use tower::{Layer, Service};

/// Configuration for version header middleware.
#[derive(Clone)]
pub struct VersionHeaderConfig {
    /// The service name to include in headers
    pub service_name: &'static str,
    /// The service version to include in headers
    pub version: &'static str,
    /// Whether to show version headers (controlled by SHOW_VERSION_IN_HEADERS env)
    pub enabled: bool,
}

impl VersionHeaderConfig {
    /// Create a new config from environment.
    ///
    /// Reads `SHOW_VERSION_IN_HEADERS` env var to determine if headers should be added.
    pub fn from_env(service_name: &'static str, version: &'static str) -> Self {
        let enabled = std::env::var("SHOW_VERSION_IN_HEADERS")
            .map(|v| v.eq_ignore_ascii_case("true") || v == "1")
            .unwrap_or(false);

        Self {
            service_name,
            version,
            enabled,
        }
    }
}

/// Layer that adds version headers to responses.
#[derive(Clone)]
pub struct VersionHeaderLayer {
    config: VersionHeaderConfig,
}

impl VersionHeaderLayer {
    pub fn new(config: VersionHeaderConfig) -> Self {
        Self { config }
    }
}

impl<S> Layer<S> for VersionHeaderLayer {
    type Service = VersionHeaderService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        VersionHeaderService {
            inner,
            config: self.config.clone(),
        }
    }
}

/// Service that adds version headers to responses.
#[derive(Clone)]
pub struct VersionHeaderService<S> {
    inner: S,
    config: VersionHeaderConfig,
}

impl<S, ReqBody, ResBody> Service<Request<ReqBody>> for VersionHeaderService<S>
where
    S: Service<Request<ReqBody>, Response = Response<ResBody>> + Clone + Send + 'static,
    S::Future: Send,
    ReqBody: Send + 'static,
    ResBody: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<ReqBody>) -> Self::Future {
        let config = self.config.clone();
        let future = self.inner.call(req);

        Box::pin(async move {
            let mut response = future.await?;

            if config.enabled {
                let headers = response.headers_mut();
                headers.insert("X-Service-Name", config.service_name.parse().unwrap());
                headers.insert("X-Service-Version", config.version.parse().unwrap());
            }

            Ok(response)
        })
    }
}

/// Create a version header layer from environment configuration.
///
/// # Example
///
/// ```rust,ignore
/// use lib_http_common::version_header_layer;
///
/// let app = Router::new()
///     .route("/health", get(health))
///     .layer(version_header_layer(env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION")));
/// ```
pub fn version_header_layer(
    service_name: &'static str,
    version: &'static str,
) -> VersionHeaderLayer {
    VersionHeaderLayer::new(VersionHeaderConfig::from_env(service_name, version))
}
