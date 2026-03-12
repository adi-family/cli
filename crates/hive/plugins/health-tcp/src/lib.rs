//! TCP Health Check Plugin for Hive
//!
//! Checks service health by attempting to connect to a TCP port.
//!
//! ## Configuration
//!
//! ```yaml
//! health:
//!   - type: tcp
//!     tcp:
//!       port: "{{runtime.port.main}}"
//!       timeout: 5s
//! ```

use anyhow::anyhow;
use lib_plugin_abi_v3::{
    async_trait,
    health::{HealthCheck, HealthResult},
    runner::RuntimeContext,
    utils::parse_duration,
    Plugin, PluginCategory, PluginContext, PluginMetadata, PluginType,
    Result as PluginResult, SERVICE_HEALTH_CHECK,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::{Duration, Instant};
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tracing::{debug, trace, warn};

pub struct TcpHealthPlugin {
    default_timeout: Duration,
}

impl Default for TcpHealthPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl TcpHealthPlugin {
    pub fn new() -> Self {
        Self {
            default_timeout: Duration::from_secs(5),
        }
    }
}

#[async_trait]
impl Plugin for TcpHealthPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            id: "hive.health.tcp".to_string(),
            name: "TCP Health Check".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            plugin_type: PluginType::Orchestration,
            author: Some("ADI Team".to_string()),
            description: Some("TCP port health check".to_string()),
            category: Some(PluginCategory::Health),
        }
    }

    async fn init(&mut self, ctx: &PluginContext) -> PluginResult<()> {
        if let Some(timeout) = ctx.config.get("timeout").and_then(|v| v.as_str()) {
            if let Some(duration) = parse_duration(timeout) {
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
impl HealthCheck for TcpHealthPlugin {
    async fn check(&self, config: &Value, ctx: &RuntimeContext) -> PluginResult<HealthResult> {
        let tcp_config: TcpHealthConfig = config
            .get("tcp")
            .ok_or_else(|| anyhow!("Missing 'tcp' configuration"))?
            .clone()
            .try_into()
            .map_err(|e| anyhow!("Invalid TCP health config: {}", e))?;

        let port_str = ctx.interpolate(&tcp_config.port)?;
        let port: u16 = port_str
            .parse()
            .map_err(|_| anyhow!("Invalid port: {}", port_str))?;

        let timeout = tcp_config
            .timeout
            .as_ref()
            .and_then(|t| parse_duration(t))
            .unwrap_or(self.default_timeout);

        let addr = format!("127.0.0.1:{}", port);

        debug!(
            addr = %addr,
            timeout_ms = timeout.as_millis() as u64,
            "Starting TCP health check"
        );

        let start = Instant::now();

        match tokio::time::timeout(timeout, TcpStream::connect(&addr)).await {
            Ok(Ok(mut stream)) => {
                let elapsed = start.elapsed();
                let _ = stream.shutdown().await;
                trace!(addr = %addr, elapsed_ms = elapsed.as_millis() as u64, "TCP health check passed");
                Ok(HealthResult::healthy()
                    .with_response_time(elapsed.as_millis() as u64)
                    .with_detail("message", format!("TCP port {} open", port)))
            }
            Ok(Err(e)) => {
                warn!(addr = %addr, error = %e, "TCP health check connection failed");
                Ok(HealthResult::unhealthy(format!(
                    "TCP connection failed: {}",
                    e
                )))
            }
            Err(_) => {
                warn!(addr = %addr, timeout_ms = timeout.as_millis() as u64, "TCP health check timed out");
                Ok(HealthResult::unhealthy("TCP connection timed out"))
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TcpHealthConfig {
    /// Port (can be interpolated with {{runtime.port.X}})
    pub port: String,
    pub timeout: Option<String>,
}

impl TryFrom<Value> for TcpHealthConfig {
    type Error = serde_json::Error;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        serde_json::from_value(value)
    }
}

#[cfg(feature = "plugin")]
#[no_mangle]
pub fn plugin_create() -> Box<dyn Plugin> {
    Box::new(TcpHealthPlugin::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_parse() {
        let config = serde_json::json!({
            "port": "5432",
            "timeout": "5s"
        });

        let tcp_config: TcpHealthConfig = config.try_into().unwrap();
        assert_eq!(tcp_config.port, "5432");
    }
}
