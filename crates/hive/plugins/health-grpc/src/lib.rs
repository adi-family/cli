//! gRPC Port Reachability Check Plugin for Hive
//!
//! Checks if a gRPC port is reachable via TCP connection.
//!
//! **Note:** This plugin performs TCP connectivity checks only. It does NOT
//! implement the gRPC Health Checking Protocol (grpc.health.v1.Health/Check).
//! A successful check means the port is open and accepting connections, not
//! that the gRPC service is healthy or serving requests correctly.
//!
//! For true gRPC health checking, use a command-based health check with
//! `grpcurl` or similar tools that can send actual gRPC health check RPCs.
//!
//! ## Configuration
//!
//! ```yaml
//! healthcheck:
//!   type: grpc
//!   grpc:
//!     port: "{{runtime.port.grpc}}"
//!     timeout: 5s
//! ```
//!
//! ## Limitations
//!
//! - Only checks TCP connectivity, not gRPC protocol health
//! - The `service` field is accepted but currently ignored
//! - Always connects to 127.0.0.1 (localhost)

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
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tracing::{debug, trace, warn};

pub struct GrpcHealthPlugin {
    default_timeout: Duration,
}

impl Default for GrpcHealthPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl GrpcHealthPlugin {
    pub fn new() -> Self {
        Self {
            default_timeout: Duration::from_secs(5),
        }
    }
}

#[async_trait]
impl Plugin for GrpcHealthPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            id: "hive.health.grpc".to_string(),
            name: "gRPC Health Check".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            plugin_type: PluginType::Orchestration,
            author: Some("ADI Team".to_string()),
            description: Some("gRPC port reachability check (TCP only, not full gRPC health protocol)".to_string()),
            category: Some(PluginCategory::Health),
        }
    }

    async fn init(&mut self, ctx: &PluginContext) -> PluginResult<()> {
        if let Some(timeout_str) = ctx.config.get("timeout").and_then(|v| v.as_str()) {
            if let Some(duration) = parse_duration(timeout_str) {
                self.default_timeout = duration;
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
impl HealthCheck for GrpcHealthPlugin {
    async fn check(&self, config: &serde_json::Value, ctx: &RuntimeContext) -> PluginResult<HealthResult> {
        let grpc_config: GrpcHealthConfig = config
            .get("grpc")
            .ok_or_else(|| lib_plugin_abi_v3::PluginError::Config("Missing 'grpc' configuration".to_string()))?
            .clone()
            .try_into()
            .map_err(|e| lib_plugin_abi_v3::PluginError::Config(format!("Invalid gRPC health config: {}", e)))?;

        let port_str = ctx.interpolate(&grpc_config.port)?;
        let port: u16 = port_str
            .parse()
            .map_err(|_| lib_plugin_abi_v3::PluginError::Config(format!("Invalid port: {}", port_str)))?;

        let addr = format!("127.0.0.1:{}", port);

        let check_timeout = grpc_config
            .timeout
            .as_ref()
            .and_then(|t| parse_duration(t))
            .unwrap_or(self.default_timeout);

        if grpc_config.service.is_some() {
            warn!(
                service = ?grpc_config.service,
                "gRPC service name specified but this plugin only checks TCP connectivity. \
                 Use hive.health.cmd with grpcurl for proper gRPC health checking."
            );
        }

        debug!(
            addr = %addr,
            timeout_ms = check_timeout.as_millis() as u64,
            "Starting gRPC port reachability check (TCP only)"
        );

        let start = Instant::now();

        let result = tokio::time::timeout(
            check_timeout,
            TcpStream::connect(&addr),
        )
        .await;

        let elapsed = start.elapsed();

        match result {
            Ok(Ok(mut stream)) => {
                let _ = stream.shutdown().await;
                trace!(addr = %addr, elapsed_ms = elapsed.as_millis() as u64, "gRPC port reachable (TCP check only)");
                Ok(HealthResult::healthy()
                    .with_response_time(elapsed.as_millis() as u64)
                    .with_message(format!("gRPC port {} reachable (TCP check only - not full gRPC health)", port)))
            }
            Ok(Err(e)) => {
                warn!(addr = %addr, error = %e, "gRPC port unreachable");
                Ok(HealthResult::unhealthy(format!("TCP connection failed: {}", e))
                    .with_response_time(elapsed.as_millis() as u64))
            }
            Err(_) => {
                warn!(addr = %addr, timeout_s = check_timeout.as_secs(), "gRPC port connection timed out");
                Ok(HealthResult::unhealthy(format!(
                    "TCP connection timed out after {}s",
                    check_timeout.as_secs()
                )))
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrpcHealthConfig {
    /// Port (can be interpolated with {{runtime.port.X}})
    pub port: String,
    /// Service name; accepted but ignored — this plugin only checks TCP connectivity
    pub service: Option<String>,
    pub timeout: Option<String>,
}

impl TryFrom<serde_json::Value> for GrpcHealthConfig {
    type Error = serde_json::Error;

    fn try_from(value: serde_json::Value) -> Result<Self, Self::Error> {
        serde_json::from_value(value)
    }
}

#[cfg(feature = "plugin")]
#[no_mangle]
pub fn plugin_create() -> Box<dyn Plugin> {
    Box::new(GrpcHealthPlugin::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metadata() {
        let plugin = GrpcHealthPlugin::new();
        let meta = plugin.metadata();
        assert_eq!(meta.id, "hive.health.grpc");
        assert_eq!(meta.name, "gRPC Health Check");
    }

    #[test]
    fn test_config_parse() {
        let config = serde_json::json!({
            "port": "9090",
            "service": "my.service",
            "timeout": "5s"
        });

        let grpc_config: GrpcHealthConfig = config.try_into().unwrap();
        assert_eq!(grpc_config.port, "9090");
        assert_eq!(grpc_config.service, Some("my.service".to_string()));
    }
}
