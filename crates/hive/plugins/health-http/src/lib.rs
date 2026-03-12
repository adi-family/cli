//! HTTP Health Check Plugin for Hive
//!
//! Checks service health by making HTTP requests to an endpoint.
//!
//! ## Configuration
//!
//! ```yaml
//! health:
//!   - type: http
//!     http:
//!       port: "${PORT:main}"
//!       path: /health
//!       method: GET
//!       status: 200
//!       timeout: 5s
//! ```

use async_trait::async_trait;
use lib_plugin_abi_v3::{
    health::{HealthCheck, HealthResult},
    runner::RuntimeContext,
    utils::parse_duration,
    Plugin, PluginCategory, PluginContext, PluginMetadata, PluginType,
    Result as PluginResult, SERVICE_HEALTH_CHECK,
};
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use tracing::{debug, trace, warn};

pub struct HttpHealthPlugin {
    client: reqwest::Client,
}

impl Default for HttpHealthPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl HttpHealthPlugin {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        Self { client }
    }
}

#[async_trait]
impl Plugin for HttpHealthPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            id: "hive.health.http".to_string(),
            name: "HTTP Health Check".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            plugin_type: PluginType::Orchestration,
            author: Some("ADI Team".to_string()),
            description: Some("HTTP endpoint health checker".to_string()),
            category: Some(PluginCategory::Health),
        }
    }

    async fn init(&mut self, ctx: &PluginContext) -> PluginResult<()> {
        if let Some(timeout) = ctx.config.get("timeout").and_then(|v| v.as_str()) {
            if let Some(duration) = parse_duration(timeout) {
                self.client = reqwest::Client::builder()
                    .timeout(duration)
                    .build()
                    .unwrap_or_else(|_| reqwest::Client::new());
            }
        }
        Ok(())
    }

    async fn shutdown(&self) -> PluginResult<()> {
        Ok(())
    }

    fn provides(&self) -> Vec<&'static str> {
        vec![SERVICE_HEALTH_CHECK]
    }
}

#[async_trait]
impl HealthCheck for HttpHealthPlugin {
    async fn check(
        &self,
        config: &serde_json::Value,
        ctx: &RuntimeContext,
    ) -> PluginResult<HealthResult> {
        let http_config: HttpHealthConfig = config
            .get("http")
            .ok_or_else(|| {
                lib_plugin_abi_v3::PluginError::Config("Missing 'http' configuration".to_string())
            })?
            .clone()
            .try_into()
            .map_err(|e| {
                lib_plugin_abi_v3::PluginError::Config(format!("Invalid HTTP health config: {}", e))
            })?;

        let port_str = interpolate_port(&http_config.port, ctx)?;
        let port: u16 = port_str.parse().map_err(|_| {
            lib_plugin_abi_v3::PluginError::Config(format!("Invalid port: {}", port_str))
        })?;

        let url = format!("http://127.0.0.1:{}{}", port, http_config.path);

        let timeout = http_config
            .timeout
            .as_ref()
            .and_then(|t| parse_duration(t))
            .unwrap_or(Duration::from_secs(5));

        debug!(
            url = %url,
            method = %http_config.method,
            timeout_ms = timeout.as_millis() as u64,
            expected_status = http_config.status.unwrap_or(200),
            "Starting HTTP health check"
        );

        let client = reqwest::Client::builder()
            .timeout(timeout)
            .build()
            .map_err(|e| lib_plugin_abi_v3::PluginError::Other(e.into()))?;

        let request = match http_config.method.to_uppercase().as_str() {
            "GET" => client.get(&url),
            "HEAD" => client.head(&url),
            "POST" => client.post(&url),
            _ => client.get(&url),
        };

        let start = Instant::now();
        match request.send().await {
            Ok(response) => {
                let elapsed = start.elapsed();
                let status = response.status();
                let expected = http_config.status.unwrap_or(200);

                let is_healthy = if expected == 200 {
                    status.is_success()
                } else {
                    status.as_u16() == expected
                };

                if is_healthy {
                    trace!(
                        url = %url,
                        status = status.as_u16(),
                        elapsed_ms = elapsed.as_millis() as u64,
                        "HTTP health check passed"
                    );
                    Ok(HealthResult::healthy()
                        .with_response_time(elapsed.as_millis() as u64)
                        .with_detail("url", &url)
                        .with_detail("status", &status.as_u16().to_string()))
                } else {
                    warn!(
                        url = %url,
                        status = status.as_u16(),
                        expected = expected,
                        elapsed_ms = elapsed.as_millis() as u64,
                        "HTTP health check failed: unexpected status"
                    );
                    Ok(HealthResult::unhealthy(format!(
                        "HTTP {} (expected {})",
                        status.as_u16(),
                        expected
                    ))
                    .with_response_time(elapsed.as_millis() as u64))
                }
            }
            Err(e) => {
                warn!(url = %url, error = %e, "HTTP health check request failed");
                Ok(HealthResult::unhealthy(format!(
                    "HTTP request failed: {}",
                    e
                )))
            }
        }
    }
}

/// Supports `${PORT:name}` (v3) and legacy `{{runtime.port.name}}` syntax, or a literal port number.
fn interpolate_port(port_config: &str, ctx: &RuntimeContext) -> PluginResult<String> {
    if let Some(rest) = port_config.strip_prefix("${PORT:") {
        if let Some(name) = rest.strip_suffix('}') {
            return ctx.ports.get(name).map(|p| p.to_string()).ok_or_else(|| {
                lib_plugin_abi_v3::PluginError::Config(format!("Unknown port: {}", name))
            });
        }
    }

    if let Some(rest) = port_config.strip_prefix("{{runtime.port.") {
        if let Some(name) = rest.strip_suffix("}}") {
            return ctx.ports.get(name).map(|p| p.to_string()).ok_or_else(|| {
                lib_plugin_abi_v3::PluginError::Config(format!("Unknown port: {}", name))
            });
        }
    }

    Ok(port_config.to_string())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpHealthConfig {
    /// Port (can be interpolated with ${PORT:X} or {{runtime.port.X}})
    pub port: String,
    #[serde(default = "default_path")]
    pub path: String,
    #[serde(default = "default_method")]
    pub method: String,
    /// Expected status code (default: 200)
    pub status: Option<u16>,
    pub timeout: Option<String>,
}

fn default_path() -> String {
    "/health".to_string()
}

fn default_method() -> String {
    "GET".to_string()
}

impl TryFrom<serde_json::Value> for HttpHealthConfig {
    type Error = serde_json::Error;

    fn try_from(value: serde_json::Value) -> Result<Self, Self::Error> {
        serde_json::from_value(value)
    }
}

#[cfg(feature = "plugin")]
#[no_mangle]
pub fn plugin_create() -> Box<dyn Plugin> {
    Box::new(HttpHealthPlugin::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_parse() {
        let config = serde_json::json!({
            "port": "8080",
            "path": "/api/health",
            "method": "GET",
            "timeout": "5s"
        });

        let http_config: HttpHealthConfig = config.try_into().unwrap();
        assert_eq!(http_config.port, "8080");
        assert_eq!(http_config.path, "/api/health");
    }

    #[test]
    fn test_interpolate_port_v3_syntax() {
        use std::path::PathBuf;

        let ctx = RuntimeContext::new("test-service", PathBuf::from("/tmp"))
            .with_port("main", 8080)
            .with_port("admin", 9090);

        assert_eq!(
            interpolate_port("${PORT:main}", &ctx).unwrap(),
            "8080"
        );
        assert_eq!(
            interpolate_port("${PORT:admin}", &ctx).unwrap(),
            "9090"
        );
    }

    #[test]
    fn test_interpolate_port_old_syntax() {
        use std::path::PathBuf;

        let ctx = RuntimeContext::new("test-service", PathBuf::from("/tmp"))
            .with_port("main", 8080);

        assert_eq!(
            interpolate_port("{{runtime.port.main}}", &ctx).unwrap(),
            "8080"
        );
    }

    #[test]
    fn test_interpolate_port_literal() {
        use std::path::PathBuf;

        let ctx = RuntimeContext::new("test-service", PathBuf::from("/tmp"));

        assert_eq!(interpolate_port("3000", &ctx).unwrap(), "3000");
    }
}
