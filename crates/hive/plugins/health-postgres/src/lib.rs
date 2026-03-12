//! PostgreSQL Health Check Plugin for Hive
//!
//! Checks service health by connecting to PostgreSQL.
//!
//! ## Configuration
//!
//! ```yaml
//! healthcheck:
//!   type: postgres
//!   postgres:
//!     port: "{{runtime.port.db}}"
//!     host: localhost
//!     user: adi
//!     database: adi_auth
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
use tokio_postgres::NoTls;
use tracing::{debug, trace, warn};

pub struct PostgresHealthPlugin {
    default_timeout: Duration,
}

impl Default for PostgresHealthPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl PostgresHealthPlugin {
    pub fn new() -> Self {
        Self {
            default_timeout: Duration::from_secs(5),
        }
    }
}

#[async_trait]
impl Plugin for PostgresHealthPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            id: "hive.health.postgres".to_string(),
            name: "PostgreSQL Health Check".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            plugin_type: PluginType::Orchestration,
            author: Some("ADI Team".to_string()),
            description: Some("PostgreSQL connection health check".to_string()),
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
impl HealthCheck for PostgresHealthPlugin {
    async fn check(&self, config: &serde_json::Value, ctx: &RuntimeContext) -> PluginResult<HealthResult> {
        let pg_config: PostgresHealthConfig = config
            .get("postgres")
            .ok_or_else(|| lib_plugin_abi_v3::PluginError::Config("Missing 'postgres' configuration".to_string()))?
            .clone()
            .try_into()
            .map_err(|e| lib_plugin_abi_v3::PluginError::Config(format!("Invalid PostgreSQL health config: {}", e)))?;

        let port_str = ctx.interpolate(&pg_config.port)?;
        let port: u16 = port_str
            .parse()
            .map_err(|_| lib_plugin_abi_v3::PluginError::Config(format!("Invalid port: {}", port_str)))?;

        let host = pg_config.host.as_deref().unwrap_or("127.0.0.1");
        let user = pg_config.user.as_deref().unwrap_or("postgres");
        let database = pg_config.database.as_deref().unwrap_or("postgres");

        let connection_string = if let Some(ref password) = pg_config.password {
            format!(
                "host={} port={} user={} password={} dbname={} connect_timeout=5",
                host, port, user, password, database
            )
        } else {
            format!(
                "host={} port={} user={} dbname={} connect_timeout=5",
                host, port, user, database
            )
        };

        let check_timeout = pg_config
            .timeout
            .as_ref()
            .and_then(|t| parse_duration(t))
            .unwrap_or(self.default_timeout);

        debug!(
            host = %host,
            port = port,
            database = %database,
            user = %user,
            timeout_ms = check_timeout.as_millis() as u64,
            "Starting PostgreSQL health check"
        );

        let start = Instant::now();

        let result = tokio::time::timeout(
            check_timeout,
            tokio_postgres::connect(&connection_string, NoTls),
        )
        .await;

        let elapsed = start.elapsed();

        match result {
            Ok(Ok((client, connection))) => {
                tokio::spawn(async move {
                    let _ = connection.await;
                });

                match client.simple_query("SELECT 1").await {
                    Ok(_) => {
                        trace!(
                            host = %host, port = port, database = %database,
                            elapsed_ms = elapsed.as_millis() as u64,
                            "PostgreSQL health check passed"
                        );
                        Ok(HealthResult::healthy()
                            .with_response_time(elapsed.as_millis() as u64)
                            .with_message(format!(
                                "PostgreSQL connected: {}:{}/{}",
                                host, port, database
                            )))
                    }
                    Err(e) => {
                        warn!(host = %host, port = port, error = %e, "PostgreSQL health check query failed");
                        Ok(HealthResult::unhealthy(format!("Query failed: {}", e))
                            .with_response_time(elapsed.as_millis() as u64))
                    }
                }
            }
            Ok(Err(e)) => {
                warn!(host = %host, port = port, error = %e, "PostgreSQL health check connection failed");
                Ok(HealthResult::unhealthy(format!("Connection failed: {}", e))
                    .with_response_time(elapsed.as_millis() as u64))
            }
            Err(_) => {
                warn!(host = %host, port = port, timeout_s = check_timeout.as_secs(), "PostgreSQL health check timed out");
                Ok(HealthResult::unhealthy(format!(
                    "Connection timed out after {}s",
                    check_timeout.as_secs()
                )))
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostgresHealthConfig {
    /// Port (can be interpolated with {{runtime.port.X}})
    pub port: String,
    /// Default: 127.0.0.1
    pub host: Option<String>,
    /// Default: postgres
    pub user: Option<String>,
    pub password: Option<String>,
    /// Default: postgres
    pub database: Option<String>,
    pub timeout: Option<String>,
}

impl TryFrom<serde_json::Value> for PostgresHealthConfig {
    type Error = serde_json::Error;

    fn try_from(value: serde_json::Value) -> Result<Self, Self::Error> {
        serde_json::from_value(value)
    }
}

#[cfg(feature = "plugin")]
#[no_mangle]
pub fn plugin_create() -> Box<dyn Plugin> {
    Box::new(PostgresHealthPlugin::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metadata() {
        let plugin = PostgresHealthPlugin::new();
        let meta = plugin.metadata();
        assert_eq!(meta.id, "hive.health.postgres");
        assert_eq!(meta.name, "PostgreSQL Health Check");
    }

    #[test]
    fn test_config_parse() {
        let config = serde_json::json!({
            "port": "5432",
            "host": "localhost",
            "user": "adi",
            "database": "adi_auth"
        });

        let pg_config: PostgresHealthConfig = config.try_into().unwrap();
        assert_eq!(pg_config.port, "5432");
        assert_eq!(pg_config.host, Some("localhost".to_string()));
    }
}
