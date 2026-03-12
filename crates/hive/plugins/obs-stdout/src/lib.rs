//! Stdout Observability Plugin for Hive
//!
//! Outputs logs and events to the console with configurable formatting.
//!
//! ## Configuration
//!
//! ```yaml
//! observability:
//!   - type: stdout
//!     stdout:
//!       format: pretty  # pretty, json, compact
//!       level: info     # minimum log level
//!       colors: true    # enable ANSI colors
//! ```

use async_trait::async_trait;
use lib_plugin_abi_v3::{
    obs::{HealthStatus, LogLevel, ObservabilityEvent, ObservabilitySink},
    Plugin, PluginCategory, PluginContext, PluginMetadata, PluginType,
    Result as PluginResult, SERVICE_OBSERVABILITY_SINK,
};

pub struct StdoutObsPlugin {
    format: OutputFormat,
    min_level: LogLevel,
    colors: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Pretty,
    /// One JSON object per line
    Json,
    Compact,
}

impl Default for StdoutObsPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl StdoutObsPlugin {
    pub fn new() -> Self {
        Self {
            format: OutputFormat::Pretty,
            min_level: LogLevel::Info,
            colors: true,
        }
    }

    fn format_event(&self, event: &ObservabilityEvent) -> String {
        match self.format {
            OutputFormat::Json => {
                serde_json::to_string(event).unwrap_or_else(|_| format!("{:?}", event))
            }
            OutputFormat::Pretty => self.format_pretty(event),
            OutputFormat::Compact => self.format_compact(event),
        }
    }

    fn format_pretty(&self, event: &ObservabilityEvent) -> String {
        match event {
            ObservabilityEvent::Log {
                timestamp,
                service_fqn,
                level,
                message,
                ..
            } => {
                let level_str = if self.colors {
                    match level {
                        LogLevel::Trace => "\x1b[90mTRACE\x1b[0m",
                        LogLevel::Debug => "\x1b[36mDEBUG\x1b[0m",
                        LogLevel::Info => "\x1b[32mINFO\x1b[0m ",
                        LogLevel::Notice => "\x1b[34mNOTCE\x1b[0m",
                        LogLevel::Warn => "\x1b[33mWARN\x1b[0m ",
                        LogLevel::Error => "\x1b[31mERROR\x1b[0m",
                        LogLevel::Fatal => "\x1b[35mFATAL\x1b[0m",
                    }
                } else {
                    match level {
                        LogLevel::Trace => "TRACE",
                        LogLevel::Debug => "DEBUG",
                        LogLevel::Info => "INFO ",
                        LogLevel::Notice => "NOTCE",
                        LogLevel::Warn => "WARN ",
                        LogLevel::Error => "ERROR",
                        LogLevel::Fatal => "FATAL",
                    }
                };
                format!(
                    "[{}] {} [{}] {}",
                    timestamp.format("%Y-%m-%dT%H:%M:%S%.3f"),
                    level_str,
                    service_fqn,
                    message
                )
            }
            ObservabilityEvent::ServiceEvent {
                timestamp,
                service_fqn,
                event_type,
                ..
            } => {
                format!(
                    "[{}] SERVICE [{}] {:?}",
                    timestamp.format("%Y-%m-%dT%H:%M:%S%.3f"),
                    service_fqn,
                    event_type,
                )
            }
            ObservabilityEvent::HealthCheck {
                timestamp,
                service_fqn,
                status,
                ..
            } => {
                let status_str = if self.colors {
                    match status {
                        HealthStatus::Healthy => "\x1b[32mHEALTHY\x1b[0m",
                        HealthStatus::Unhealthy => "\x1b[31mUNHEALTHY\x1b[0m",
                        HealthStatus::Unknown => "\x1b[33mUNKNOWN\x1b[0m",
                    }
                } else {
                    match status {
                        HealthStatus::Healthy => "HEALTHY",
                        HealthStatus::Unhealthy => "UNHEALTHY",
                        HealthStatus::Unknown => "UNKNOWN",
                    }
                };
                format!(
                    "[{}] HEALTH [{}] {}",
                    timestamp.format("%Y-%m-%dT%H:%M:%S%.3f"),
                    service_fqn,
                    status_str
                )
            }
            ObservabilityEvent::Metric {
                timestamp,
                service_fqn,
                name,
                value,
                ..
            } => {
                format!(
                    "[{}] METRIC [{}] {}: {:?}",
                    timestamp.format("%Y-%m-%dT%H:%M:%S%.3f"),
                    service_fqn,
                    name,
                    value
                )
            }
        }
    }

    fn format_compact(&self, event: &ObservabilityEvent) -> String {
        match event {
            ObservabilityEvent::Log {
                service_fqn,
                level,
                message,
                ..
            } => {
                format!("{:?}|{}|{}", level, service_fqn, message)
            }
            _ => format!("{:?}", event),
        }
    }
}

#[async_trait]
impl Plugin for StdoutObsPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            id: "hive.obs.stdout".to_string(),
            name: "stdout".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            plugin_type: PluginType::Orchestration,
            author: Some("ADI Team".to_string()),
            description: Some("Console output for logs and events".to_string()),
            category: Some(PluginCategory::Obs),
        }
    }

    async fn init(&mut self, ctx: &PluginContext) -> PluginResult<()> {
        let config = &ctx.config;

        if let Some(format) = config.get("format").and_then(|v| v.as_str()) {
            self.format = match format {
                "json" => OutputFormat::Json,
                "compact" => OutputFormat::Compact,
                _ => OutputFormat::Pretty,
            };
        }

        if let Some(level) = config.get("level").and_then(|v| v.as_str()) {
            self.min_level = level.parse().unwrap_or(LogLevel::Info);
        }

        if let Some(colors) = config.get("colors").and_then(|v| v.as_bool()) {
            self.colors = colors;
        }

        Ok(())
    }

    async fn shutdown(&self) -> PluginResult<()> {
        Ok(())
    }

    fn provides(&self) -> Vec<&'static str> {
        vec![SERVICE_OBSERVABILITY_SINK]
    }
}

#[async_trait]
impl ObservabilitySink for StdoutObsPlugin {
    async fn init_obs(&mut self, _config: &serde_json::Value) -> PluginResult<()> {
        Ok(())
    }

    async fn handle(&self, event: &ObservabilityEvent) {
        if let ObservabilityEvent::Log { level, .. } = event {
            if *level < self.min_level {
                return;
            }
        }

        let output = self.format_event(event);
        println!("{}", output);
    }

    async fn flush(&self) -> PluginResult<()> {
        Ok(())
    }
}

#[cfg(feature = "plugin")]
#[no_mangle]
pub fn plugin_create() -> Box<dyn Plugin> {
    Box::new(StdoutObsPlugin::new())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use lib_plugin_abi_v3::obs::LogStream;
    use std::collections::HashMap;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_init() {
        let mut plugin = StdoutObsPlugin::new();
        let ctx = PluginContext::new(
            "hive.obs.stdout",
            PathBuf::from("/tmp/data"),
            PathBuf::from("/tmp/config"),
            serde_json::json!({"format": "json"}),
        );
        plugin.init(&ctx).await.unwrap();
        assert_eq!(plugin.format, OutputFormat::Json);
    }

    #[test]
    fn test_format_pretty() {
        let plugin = StdoutObsPlugin::new();
        let event = ObservabilityEvent::Log {
            timestamp: Utc::now(),
            service_fqn: "test".to_string(),
            level: LogLevel::Info,
            message: "test message".to_string(),
            fields: HashMap::new(),
            stream: LogStream::Stdout,
        };

        let output = plugin.format_event(&event);
        assert!(output.contains("test message"));
    }
}
