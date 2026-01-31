//! Observability sink plugin trait

use crate::{Plugin, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json::Value;
use std::collections::HashMap;

/// Observability sink plugin trait
///
/// Observability plugins capture and route logs, metrics, and events to
/// various destinations (stdout, files, Loki, Prometheus, etc.).
#[async_trait]
pub trait ObservabilitySink: Plugin {
    /// Handle an observability event
    async fn handle(&self, event: &ObservabilityEvent);

    /// Flush buffered events
    async fn flush(&self) -> Result<()>;
}

/// Observability event
#[derive(Debug, Clone)]
pub enum ObservabilityEvent {
    /// Log message
    Log {
        timestamp: DateTime<Utc>,
        level: LogLevel,
        source: String,
        message: String,
        fields: HashMap<String, String>,
    },

    /// Service lifecycle event
    ServiceEvent {
        timestamp: DateTime<Utc>,
        service_name: String,
        event_type: ServiceEventType,
        details: HashMap<String, String>,
    },

    /// Health check result
    HealthCheck {
        timestamp: DateTime<Utc>,
        service_name: String,
        result: crate::health::HealthResult,
    },

    /// Metric
    Metric {
        timestamp: DateTime<Utc>,
        name: String,
        value: f64,
        labels: HashMap<String, String>,
    },
}

/// Log level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    Trace = 0,
    Debug = 1,
    Info = 2,
    Notice = 3,
    Warn = 4,
    Error = 5,
    Fatal = 6,
}

/// Service event type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceEventType {
    Starting,
    Started,
    Healthy,
    Unhealthy,
    Stopping,
    Stopped,
    Failed,
}
