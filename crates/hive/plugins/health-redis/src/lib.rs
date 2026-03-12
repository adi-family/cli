//! Redis Health Check Plugin for Hive
//!
//! Checks service health by sending PING to Redis.
//!
//! ## Configuration
//!
//! ```yaml
//! healthcheck:
//!   type: redis
//!   redis:
//!     port: "{{runtime.port.cache}}"
//!     host: localhost
//!     timeout: 5s
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

pub struct RedisHealthPlugin {
    default_timeout: Duration,
}

impl Default for RedisHealthPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl RedisHealthPlugin {
    pub fn new() -> Self {
        Self {
            default_timeout: Duration::from_secs(5),
        }
    }
}

#[async_trait]
impl Plugin for RedisHealthPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            id: "hive.health.redis".to_string(),
            name: "Redis Health Check".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            plugin_type: PluginType::Orchestration,
            author: Some("ADI Team".to_string()),
            description: Some("Redis PING health check".to_string()),
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
impl HealthCheck for RedisHealthPlugin {
    async fn check(&self, config: &serde_json::Value, ctx: &RuntimeContext) -> PluginResult<HealthResult> {
        let redis_config: RedisHealthConfig = config
            .get("redis")
            .ok_or_else(|| lib_plugin_abi_v3::PluginError::Config("Missing 'redis' configuration".to_string()))?
            .clone()
            .try_into()
            .map_err(|e| lib_plugin_abi_v3::PluginError::Config(format!("Invalid Redis health config: {}", e)))?;

        let port_str = ctx.interpolate(&redis_config.port)?;
        let port: u16 = port_str
            .parse()
            .map_err(|_| lib_plugin_abi_v3::PluginError::Config(format!("Invalid port: {}", port_str)))?;

        let host = redis_config.host.as_deref().unwrap_or("127.0.0.1");

        let url = if let Some(ref password) = redis_config.password {
            if let Some(ref username) = redis_config.username {
                format!("redis://{}:{}@{}:{}", username, password, host, port)
            } else {
                format!("redis://:{}@{}:{}", password, host, port)
            }
        } else {
            format!("redis://{}:{}", host, port)
        };

        let check_timeout = redis_config
            .timeout
            .as_ref()
            .and_then(|t| parse_duration(t))
            .unwrap_or(self.default_timeout);

        debug!(
            host = %host,
            port = port,
            timeout_ms = check_timeout.as_millis() as u64,
            "Starting Redis health check"
        );

        let start = Instant::now();

        let client = redis::Client::open(url.as_str())
            .map_err(|e| lib_plugin_abi_v3::PluginError::HealthCheckFailed(format!("Failed to create Redis client: {}", e)))?;

        let result = tokio::time::timeout(check_timeout, client.get_multiplexed_async_connection()).await;

        let elapsed = start.elapsed();

        match result {
            Ok(Ok(mut conn)) => {
                let ping_result: Result<String, redis::RedisError> =
                    redis::cmd("PING").query_async(&mut conn).await;

                match ping_result {
                    Ok(response) if response == "PONG" => {
                        trace!(host = %host, port = port, elapsed_ms = elapsed.as_millis() as u64, "Redis health check passed");
                        Ok(HealthResult::healthy()
                            .with_response_time(elapsed.as_millis() as u64)
                            .with_message(format!("Redis PONG from {}:{}", host, port)))
                    }
                    Ok(response) => {
                        warn!(host = %host, port = port, response = %response, "Redis unexpected PING response");
                        Ok(HealthResult::unhealthy(format!(
                            "Unexpected PING response: {}",
                            response
                        ))
                        .with_response_time(elapsed.as_millis() as u64))
                    }
                    Err(e) => {
                        warn!(host = %host, port = port, error = %e, "Redis PING failed");
                        Ok(HealthResult::unhealthy(format!("PING failed: {}", e))
                            .with_response_time(elapsed.as_millis() as u64))
                    }
                }
            }
            Ok(Err(e)) => {
                warn!(host = %host, port = port, error = %e, "Redis health check connection failed");
                Ok(HealthResult::unhealthy(format!("Connection failed: {}", e))
                    .with_response_time(elapsed.as_millis() as u64))
            }
            Err(_) => {
                warn!(host = %host, port = port, timeout_s = check_timeout.as_secs(), "Redis health check timed out");
                Ok(HealthResult::unhealthy(format!(
                    "Connection timed out after {}s",
                    check_timeout.as_secs()
                )))
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisHealthConfig {
    /// Port (can be interpolated with {{runtime.port.X}})
    pub port: String,
    /// Default: 127.0.0.1
    pub host: Option<String>,
    /// Username for Redis 6+ ACL
    pub username: Option<String>,
    pub password: Option<String>,
    /// Database number (default: 0)
    pub db: Option<u8>,
    pub timeout: Option<String>,
}

impl TryFrom<serde_json::Value> for RedisHealthConfig {
    type Error = serde_json::Error;

    fn try_from(value: serde_json::Value) -> Result<Self, Self::Error> {
        serde_json::from_value(value)
    }
}

#[cfg(feature = "plugin")]
#[no_mangle]
pub fn plugin_create() -> Box<dyn Plugin> {
    Box::new(RedisHealthPlugin::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metadata() {
        let plugin = RedisHealthPlugin::new();
        let meta = plugin.metadata();
        assert_eq!(meta.id, "hive.health.redis");
        assert_eq!(meta.name, "Redis Health Check");
    }

    #[test]
    fn test_config_parse() {
        let config = serde_json::json!({
            "port": "6379",
            "host": "localhost"
        });

        let redis_config: RedisHealthConfig = config.try_into().unwrap();
        assert_eq!(redis_config.port, "6379");
        assert_eq!(redis_config.host, Some("localhost".to_string()));
    }
}
