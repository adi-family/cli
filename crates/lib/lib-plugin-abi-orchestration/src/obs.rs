//! Observability Plugin Trait
//!
//! Observability plugins handle logging, metrics, and events from services.
//! Examples: stdout (console), file (log files), prometheus, datadog, etc.

use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Trait for observability plugins
#[async_trait]
pub trait ObsPlugin: Send + Sync {
    /// Get plugin metadata
    fn metadata(&self) -> crate::PluginMetadata;

    /// Initialize the plugin with configuration
    async fn init(&mut self, config: &serde_json::Value) -> Result<()>;

    /// Handle an observability event
    async fn handle(&self, event: &ObservabilityEvent);

    /// Flush any buffered events
    async fn flush(&self) -> Result<()> {
        Ok(())
    }

    /// Shutdown the plugin
    async fn shutdown(&self) -> Result<()>;
}

/// Observability event types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ObservabilityEvent {
    /// Log message from a service
    Log {
        timestamp: DateTime<Utc>,
        service_fqn: String,
        level: LogLevel,
        message: String,
        fields: HashMap<String, String>,
        stream: LogStream,
    },
    /// Service lifecycle event
    ServiceEvent {
        timestamp: DateTime<Utc>,
        service_fqn: String,
        event: ServiceEventType,
        details: Option<String>,
    },
    /// Health check result
    HealthCheck {
        timestamp: DateTime<Utc>,
        service_fqn: String,
        status: HealthStatus,
        message: Option<String>,
        response_time_ms: Option<u64>,
    },
    /// Metric value
    Metric {
        timestamp: DateTime<Utc>,
        service_fqn: String,
        name: String,
        value: MetricValue,
        labels: HashMap<String, String>,
    },
}

impl ObservabilityEvent {
    /// Get the service FQN for any event type
    pub fn service_fqn(&self) -> &str {
        match self {
            Self::Log { service_fqn, .. } => service_fqn,
            Self::ServiceEvent { service_fqn, .. } => service_fqn,
            Self::HealthCheck { service_fqn, .. } => service_fqn,
            Self::Metric { service_fqn, .. } => service_fqn,
        }
    }

    /// Get the timestamp for any event type
    pub fn timestamp(&self) -> &DateTime<Utc> {
        match self {
            Self::Log { timestamp, .. } => timestamp,
            Self::ServiceEvent { timestamp, .. } => timestamp,
            Self::HealthCheck { timestamp, .. } => timestamp,
            Self::Metric { timestamp, .. } => timestamp,
        }
    }
}

/// Log level (7-level scale)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Trace = 0,
    Debug = 1,
    Info = 2,
    Notice = 3,
    Warn = 4,
    Error = 5,
    Fatal = 6,
}

impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogLevel::Trace => write!(f, "TRACE"),
            LogLevel::Debug => write!(f, "DEBUG"),
            LogLevel::Info => write!(f, "INFO"),
            LogLevel::Notice => write!(f, "NOTICE"),
            LogLevel::Warn => write!(f, "WARN"),
            LogLevel::Error => write!(f, "ERROR"),
            LogLevel::Fatal => write!(f, "FATAL"),
        }
    }
}

impl std::str::FromStr for LogLevel {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "trace" => Ok(LogLevel::Trace),
            "debug" => Ok(LogLevel::Debug),
            "info" => Ok(LogLevel::Info),
            "notice" => Ok(LogLevel::Notice),
            "warn" | "warning" => Ok(LogLevel::Warn),
            "error" => Ok(LogLevel::Error),
            "fatal" | "critical" => Ok(LogLevel::Fatal),
            _ => Err(anyhow::anyhow!("Unknown log level: {}", s)),
        }
    }
}

/// Log stream type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogStream {
    Stdout,
    Stderr,
}

/// Service event types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServiceEventType {
    Starting,
    Started,
    Stopping,
    Stopped,
    Restarting,
    HealthChanged,
    ConfigReloaded,
    Error,
}

/// Health status for events
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HealthStatus {
    Healthy,
    Unhealthy,
    Unknown,
}

/// Metric value types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MetricValue {
    Counter(u64),
    Gauge(f64),
    Histogram(Vec<f64>),
}
