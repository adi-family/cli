//! Observability sink plugin trait

use crate::{Plugin, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Observability sink plugin trait
///
/// Observability plugins capture and route logs, metrics, and events to
/// various destinations (stdout, files, Loki, Prometheus, etc.).
#[async_trait]
pub trait ObservabilitySink: Plugin {
    /// Initialize the plugin with configuration
    async fn init_obs(&mut self, _config: &Value) -> Result<()> {
        Ok(())
    }

    /// Handle an observability event
    async fn handle(&self, event: &ObservabilityEvent);

    /// Flush buffered events
    async fn flush(&self) -> Result<()>;
}

/// Observability event
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ObservabilityEvent {
    /// Log message
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
        event_type: ServiceEventType,
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

    /// Metric
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

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
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

/// Service event type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServiceEventType {
    Starting,
    Started,
    Healthy,
    Unhealthy,
    Stopping,
    Stopped,
    Restarting,
    ConfigReloaded,
    Failed,
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
