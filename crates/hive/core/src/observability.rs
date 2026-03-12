//! Hive Observability System
//!
//! Provides comprehensive observability through a plugin-based event streaming architecture.
//! The daemon collects all observability data and streams it to subscribed plugins via Unix socket.
//!
//! ## Event Types
//!
//! - **Log**: Service stdout/stderr output with structured fields
//! - **Metric**: Numeric measurements (gauges, counters, histograms)
//! - **Span**: Distributed trace spans for request tracking
//! - **HealthCheck**: Health check results
//! - **ServiceEvent**: Lifecycle events (start, stop, crash, restart)
//! - **ProxyRequest**: HTTP/WebSocket proxy request traces
//! - **ResourceMetrics**: Process resource utilization (CPU, memory, etc.)

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::broadcast;
use tracing::{debug, trace, warn};
use uuid::Uuid;

/// Log level (extended 7-level system)
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
            LogLevel::Trace => write!(f, "trace"),
            LogLevel::Debug => write!(f, "debug"),
            LogLevel::Info => write!(f, "info"),
            LogLevel::Notice => write!(f, "notice"),
            LogLevel::Warn => write!(f, "warn"),
            LogLevel::Error => write!(f, "error"),
            LogLevel::Fatal => write!(f, "fatal"),
        }
    }
}

impl std::str::FromStr for LogLevel {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "trace" => Ok(LogLevel::Trace),
            "debug" => Ok(LogLevel::Debug),
            "info" => Ok(LogLevel::Info),
            "notice" => Ok(LogLevel::Notice),
            "warn" | "warning" => Ok(LogLevel::Warn),
            "error" | "err" => Ok(LogLevel::Error),
            "fatal" | "critical" | "crit" => Ok(LogLevel::Fatal),
            _ => Err(format!("Unknown log level: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogStream {
    Stdout,
    Stderr,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MetricValue {
    /// Current value (e.g., CPU %)
    Gauge(f64),
    /// Monotonic counter (e.g., requests)
    Counter(u64),
    /// Distribution (e.g., latency)
    Histogram {
        count: u64,
        sum: f64,
        buckets: Vec<(f64, u64)>, // (le, count)
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SpanStatus {
    Ok,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HealthStatus {
    Healthy,
    Unhealthy,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServiceEventType {
    Starting,
    Started,
    Stopping,
    Stopped,
    Crashed,
    Restarting,
    HealthChanged,
    BuildStarted,
    BuildCompleted,
    BuildFailed,
}

impl std::fmt::Display for ServiceEventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ServiceEventType::Starting => write!(f, "starting"),
            ServiceEventType::Started => write!(f, "started"),
            ServiceEventType::Stopping => write!(f, "stopping"),
            ServiceEventType::Stopped => write!(f, "stopped"),
            ServiceEventType::Crashed => write!(f, "crashed"),
            ServiceEventType::Restarting => write!(f, "restarting"),
            ServiceEventType::HealthChanged => write!(f, "health_changed"),
            ServiceEventType::BuildStarted => write!(f, "build_started"),
            ServiceEventType::BuildCompleted => write!(f, "build_completed"),
            ServiceEventType::BuildFailed => write!(f, "build_failed"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event_type", rename_all = "snake_case")]
pub enum ObservabilityEvent {
    /// Service logs (stdout/stderr)
    Log {
        timestamp: DateTime<Utc>,
        service_fqn: String,
        level: LogLevel,
        message: String,
        #[serde(default)]
        fields: HashMap<String, serde_json::Value>,
        stream: LogStream,
    },

    Metric {
        timestamp: DateTime<Utc>,
        service_fqn: String,
        name: String,
        value: MetricValue,
        #[serde(default)]
        labels: HashMap<String, String>,
    },

    /// Distributed trace spans
    Span {
        trace_id: Uuid,
        span_id: Uuid,
        parent_span_id: Option<Uuid>,
        service_fqn: String,
        operation: String,
        start: DateTime<Utc>,
        duration_us: u64,
        status: SpanStatus,
        #[serde(default)]
        attributes: HashMap<String, serde_json::Value>,
    },

    HealthCheck {
        timestamp: DateTime<Utc>,
        service_fqn: String,
        check_type: String,
        status: HealthStatus,
        latency_ms: u32,
        error: Option<String>,
    },

    ServiceEvent {
        timestamp: DateTime<Utc>,
        service_fqn: String,
        event: ServiceEventType,
        #[serde(default)]
        details: HashMap<String, serde_json::Value>,
    },

    ProxyRequest {
        timestamp: DateTime<Utc>,
        trace_id: Uuid,
        span_id: Uuid,
        service_fqn: String,
        method: String,
        path: String,
        status_code: u16,
        duration_us: u64,
        request_bytes: u64,
        response_bytes: u64,
        client_ip: Option<String>,
        user_agent: Option<String>,
        is_websocket: bool,
    },

    ResourceMetrics {
        timestamp: DateTime<Utc>,
        service_fqn: String,
        pid: u32,
        cpu_percent: f32,
        memory_rss_bytes: u64,
        memory_vms_bytes: u64,
        open_fds: u32,
        threads: u32,
        network_rx_bytes: u64,
        network_tx_bytes: u64,
    },

    Custom {
        timestamp: DateTime<Utc>,
        service_fqn: String,
        event_name: String,
        data: serde_json::Value,
    },
}

impl ObservabilityEvent {
    pub fn timestamp(&self) -> DateTime<Utc> {
        match self {
            ObservabilityEvent::Log { timestamp, .. } => *timestamp,
            ObservabilityEvent::Metric { timestamp, .. } => *timestamp,
            ObservabilityEvent::Span { start, .. } => *start,
            ObservabilityEvent::HealthCheck { timestamp, .. } => *timestamp,
            ObservabilityEvent::ServiceEvent { timestamp, .. } => *timestamp,
            ObservabilityEvent::ProxyRequest { timestamp, .. } => *timestamp,
            ObservabilityEvent::ResourceMetrics { timestamp, .. } => *timestamp,
            ObservabilityEvent::Custom { timestamp, .. } => *timestamp,
        }
    }

    pub fn service_fqn(&self) -> &str {
        match self {
            ObservabilityEvent::Log { service_fqn, .. } => service_fqn,
            ObservabilityEvent::Metric { service_fqn, .. } => service_fqn,
            ObservabilityEvent::Span { service_fqn, .. } => service_fqn,
            ObservabilityEvent::HealthCheck { service_fqn, .. } => service_fqn,
            ObservabilityEvent::ServiceEvent { service_fqn, .. } => service_fqn,
            ObservabilityEvent::ProxyRequest { service_fqn, .. } => service_fqn,
            ObservabilityEvent::ResourceMetrics { service_fqn, .. } => service_fqn,
            ObservabilityEvent::Custom { service_fqn, .. } => service_fqn,
        }
    }

    pub fn is_log(&self) -> bool {
        matches!(self, ObservabilityEvent::Log { .. })
    }

    pub fn log(
        service_fqn: impl Into<String>,
        level: LogLevel,
        message: impl Into<String>,
        stream: LogStream,
    ) -> Self {
        ObservabilityEvent::Log {
            timestamp: Utc::now(),
            service_fqn: service_fqn.into(),
            level,
            message: message.into(),
            fields: HashMap::new(),
            stream,
        }
    }

    pub fn service_event(service_fqn: impl Into<String>, event: ServiceEventType) -> Self {
        ObservabilityEvent::ServiceEvent {
            timestamp: Utc::now(),
            service_fqn: service_fqn.into(),
            event,
            details: HashMap::new(),
        }
    }

    pub fn health_check(
        service_fqn: impl Into<String>,
        check_type: impl Into<String>,
        status: HealthStatus,
        latency_ms: u32,
        error: Option<String>,
    ) -> Self {
        ObservabilityEvent::HealthCheck {
            timestamp: Utc::now(),
            service_fqn: service_fqn.into(),
            check_type: check_type.into(),
            status,
            latency_ms,
            error,
        }
    }
}

/// Get the event type name as a string (for tracing)
fn event_type_name(event: &ObservabilityEvent) -> &'static str {
    match event {
        ObservabilityEvent::Log { .. } => "log",
        ObservabilityEvent::Metric { .. } => "metric",
        ObservabilityEvent::Span { .. } => "span",
        ObservabilityEvent::HealthCheck { .. } => "health_check",
        ObservabilityEvent::ServiceEvent { .. } => "service_event",
        ObservabilityEvent::ProxyRequest { .. } => "proxy_request",
        ObservabilityEvent::ResourceMetrics { .. } => "resource_metrics",
        ObservabilityEvent::Custom { .. } => "custom",
    }
}

/// Log line for responses (simplified log representation)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogLine {
    pub timestamp: DateTime<Utc>,
    pub service_fqn: String,
    pub level: LogLevel,
    pub message: String,
    pub stream: LogStream,
}

impl From<&ObservabilityEvent> for Option<LogLine> {
    fn from(event: &ObservabilityEvent) -> Self {
        match event {
            ObservabilityEvent::Log {
                timestamp,
                service_fqn,
                level,
                message,
                stream,
                ..
            } => Some(LogLine {
                timestamp: *timestamp,
                service_fqn: service_fqn.clone(),
                level: *level,
                message: message.clone(),
                stream: *stream,
            }),
            _ => None,
        }
    }
}

/// Subscription filter for event streams
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EventSubscription {
    /// Event types to receive (empty = all)
    #[serde(default)]
    pub event_types: Vec<String>,
    /// Services to receive events from (supports wildcards like "default:*")
    #[serde(default)]
    pub services: Vec<String>,
    /// Minimum log level (only for log events)
    pub min_log_level: Option<LogLevel>,
}

impl EventSubscription {
    pub fn all() -> Self {
        Self::default()
    }

    pub fn logs() -> Self {
        Self {
            event_types: vec!["log".to_string()],
            ..Default::default()
        }
    }

    pub fn for_service(service_fqn: impl Into<String>) -> Self {
        Self {
            services: vec![service_fqn.into()],
            ..Default::default()
        }
    }

    pub fn matches(&self, event: &ObservabilityEvent) -> bool {
        // Check event type filter
        if !self.event_types.is_empty() {
            let event_type = match event {
                ObservabilityEvent::Log { .. } => "log",
                ObservabilityEvent::Metric { .. } => "metric",
                ObservabilityEvent::Span { .. } => "span",
                ObservabilityEvent::HealthCheck { .. } => "health_check",
                ObservabilityEvent::ServiceEvent { .. } => "service_event",
                ObservabilityEvent::ProxyRequest { .. } => "proxy_request",
                ObservabilityEvent::ResourceMetrics { .. } => "resource_metrics",
                ObservabilityEvent::Custom { .. } => "custom",
            };
            if !self.event_types.iter().any(|t| t == event_type) {
                return false;
            }
        }

        // Check service filter
        if !self.services.is_empty() {
            let service_fqn = event.service_fqn();
            let matches_service = self.services.iter().any(|pattern| {
                if pattern.ends_with(":*") {
                    // Wildcard: match source prefix (e.g., "source:*" matches "source:auth")
                    let source = pattern.trim_end_matches(":*");
                    service_fqn.starts_with(&format!("{}:", source))
                } else if pattern == "*" {
                    // Match all
                    true
                } else if pattern.contains(':') {
                    // Full FQN - exact match
                    pattern == service_fqn
                } else {
                    // Service name only - match as suffix (e.g., "auth" matches "source:auth")
                    service_fqn.ends_with(&format!(":{}", pattern)) || service_fqn == pattern
                }
            });
            if !matches_service {
                return false;
            }
        }

        // Check log level filter
        if let Some(min_level) = self.min_log_level {
            if let ObservabilityEvent::Log { level, .. } = event {
                if *level < min_level {
                    return false;
                }
            }
        }

        true
    }
}

/// Event collector that manages the event broadcast channel
pub struct EventCollector {
    /// Broadcast channel sender
    tx: broadcast::Sender<ObservabilityEvent>,
    buffer_size: usize,
}

impl EventCollector {
    pub fn new() -> Self {
        Self::with_buffer_size(10000)
    }

    pub fn with_buffer_size(buffer_size: usize) -> Self {
        let (tx, _) = broadcast::channel(buffer_size);
        debug!(buffer_size, "EventCollector created");
        Self { tx, buffer_size }
    }

    pub fn emit(&self, event: ObservabilityEvent) {
        trace!(
            event_type = %event_type_name(&event),
            service_fqn = %event.service_fqn(),
            subscribers = self.tx.receiver_count(),
            "Emitting observability event"
        );
        // Ignore send errors (no subscribers)
        let _ = self.tx.send(event);
    }

    pub fn subscribe(&self, filter: EventSubscription) -> FilteredReceiver {
        debug!(
            event_types = ?filter.event_types,
            services = ?filter.services,
            min_log_level = ?filter.min_log_level,
            "New event subscription created"
        );
        FilteredReceiver {
            rx: self.tx.subscribe(),
            filter,
        }
    }

    pub fn subscriber_count(&self) -> usize {
        self.tx.receiver_count()
    }

    pub fn buffer_size(&self) -> usize {
        self.buffer_size
    }
}

impl Default for EventCollector {
    fn default() -> Self {
        Self::new()
    }
}

/// A filtered receiver that only yields events matching the subscription
pub struct FilteredReceiver {
    rx: broadcast::Receiver<ObservabilityEvent>,
    filter: EventSubscription,
}

impl FilteredReceiver {
    pub async fn recv(&mut self) -> Result<ObservabilityEvent, broadcast::error::RecvError> {
        loop {
            match self.rx.recv().await {
                Ok(event) => {
                    if self.filter.matches(&event) {
                        trace!(
                            event_type = %event_type_name(&event),
                            service_fqn = %event.service_fqn(),
                            "FilteredReceiver matched event"
                        );
                        return Ok(event);
                    }
                }
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    warn!(
                        skipped = n,
                        "FilteredReceiver lagged, {} events dropped",
                        n
                    );
                    return Err(broadcast::error::RecvError::Lagged(n));
                }
                Err(e) => return Err(e),
            }
        }
    }

    pub fn try_recv(&mut self) -> Result<ObservabilityEvent, broadcast::error::TryRecvError> {
        loop {
            let event = self.rx.try_recv()?;
            if self.filter.matches(&event) {
                return Ok(event);
            }
        }
    }
}

/// Log buffer for storing recent logs per service
pub struct LogBuffer {
    /// Maximum lines per service
    max_lines: usize,
    /// Logs per service FQN
    logs: std::sync::RwLock<HashMap<String, Vec<LogLine>>>,
}

impl LogBuffer {
    pub fn new(max_lines: usize) -> Self {
        Self {
            max_lines,
            logs: std::sync::RwLock::new(HashMap::new()),
        }
    }

    pub fn add(&self, log: LogLine) {
        let mut logs = self.logs.write().unwrap();
        let service_fqn = log.service_fqn.clone();
        let service_logs = logs.entry(service_fqn.clone()).or_default();
        service_logs.push(log);

        // Trim if over limit
        if service_logs.len() > self.max_lines {
            let drain_count = service_logs.len() - self.max_lines;
            trace!(service = %service_fqn, trimmed = drain_count, "LogBuffer trimming old entries");
            service_logs.drain(0..drain_count);
        }
    }

    pub fn get(&self, service_fqn: &str, limit: Option<usize>) -> Vec<LogLine> {
        let logs = self.logs.read().unwrap();
        if let Some(service_logs) = logs.get(service_fqn) {
            let start = if let Some(limit) = limit {
                service_logs.len().saturating_sub(limit)
            } else {
                0
            };
            service_logs[start..].to_vec()
        } else {
            Vec::new()
        }
    }

    /// Get logs for all services matching a pattern
    pub fn get_all(&self, pattern: Option<&str>, limit: Option<usize>) -> Vec<LogLine> {
        trace!(pattern = ?pattern, limit = ?limit, "LogBuffer::get_all queried");
        let logs = self.logs.read().unwrap();
        let mut all_logs: Vec<LogLine> = logs
            .iter()
            .filter(|(fqn, _)| {
                pattern
                    .map(|p| {
                        if p.ends_with(":*") {
                            let source = p.trim_end_matches(":*");
                            fqn.starts_with(&format!("{}:", source))
                        } else if p == "*" {
                            true
                        } else {
                            *fqn == p
                        }
                    })
                    .unwrap_or(true)
            })
            .flat_map(|(_, logs)| logs.iter().cloned())
            .collect();

        // Sort by timestamp
        all_logs.sort_by_key(|l| l.timestamp);

        // Apply limit
        if let Some(limit) = limit {
            let start = all_logs.len().saturating_sub(limit);
            all_logs.drain(0..start);
        }

        all_logs
    }

    pub fn clear(&self, service_fqn: &str) {
        debug!(service = %service_fqn, "Clearing log buffer for service");
        let mut logs = self.logs.write().unwrap();
        logs.remove(service_fqn);
    }

    pub fn clear_all(&self) {
        debug!("Clearing all log buffers");
        let mut logs = self.logs.write().unwrap();
        logs.clear();
    }
}

impl Default for LogBuffer {
    fn default() -> Self {
        Self::new(10000)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_level_ordering() {
        assert!(LogLevel::Trace < LogLevel::Debug);
        assert!(LogLevel::Debug < LogLevel::Info);
        assert!(LogLevel::Info < LogLevel::Notice);
        assert!(LogLevel::Notice < LogLevel::Warn);
        assert!(LogLevel::Warn < LogLevel::Error);
        assert!(LogLevel::Error < LogLevel::Fatal);
    }

    #[test]
    fn test_log_level_parse() {
        assert_eq!("trace".parse::<LogLevel>().unwrap(), LogLevel::Trace);
        assert_eq!("debug".parse::<LogLevel>().unwrap(), LogLevel::Debug);
        assert_eq!("info".parse::<LogLevel>().unwrap(), LogLevel::Info);
        assert_eq!("notice".parse::<LogLevel>().unwrap(), LogLevel::Notice);
        assert_eq!("warn".parse::<LogLevel>().unwrap(), LogLevel::Warn);
        assert_eq!("warning".parse::<LogLevel>().unwrap(), LogLevel::Warn);
        assert_eq!("error".parse::<LogLevel>().unwrap(), LogLevel::Error);
        assert_eq!("fatal".parse::<LogLevel>().unwrap(), LogLevel::Fatal);
    }

    #[test]
    fn test_subscription_matches_all() {
        let sub = EventSubscription::all();
        let event = ObservabilityEvent::log("test:service", LogLevel::Info, "test", LogStream::Stdout);
        assert!(sub.matches(&event));
    }

    #[test]
    fn test_subscription_matches_service() {
        let sub = EventSubscription::for_service("test:service");
        
        let event1 = ObservabilityEvent::log("test:service", LogLevel::Info, "test", LogStream::Stdout);
        assert!(sub.matches(&event1));
        
        let event2 = ObservabilityEvent::log("other:service", LogLevel::Info, "test", LogStream::Stdout);
        assert!(!sub.matches(&event2));
    }

    #[test]
    fn test_subscription_matches_wildcard() {
        let sub = EventSubscription {
            services: vec!["test:*".to_string()],
            ..Default::default()
        };
        
        let event1 = ObservabilityEvent::log("test:service1", LogLevel::Info, "test", LogStream::Stdout);
        assert!(sub.matches(&event1));
        
        let event2 = ObservabilityEvent::log("test:service2", LogLevel::Info, "test", LogStream::Stdout);
        assert!(sub.matches(&event2));
        
        let event3 = ObservabilityEvent::log("other:service", LogLevel::Info, "test", LogStream::Stdout);
        assert!(!sub.matches(&event3));
    }

    #[test]
    fn test_subscription_matches_log_level() {
        let sub = EventSubscription {
            min_log_level: Some(LogLevel::Warn),
            ..Default::default()
        };
        
        let event1 = ObservabilityEvent::log("test:service", LogLevel::Info, "test", LogStream::Stdout);
        assert!(!sub.matches(&event1));
        
        let event2 = ObservabilityEvent::log("test:service", LogLevel::Warn, "test", LogStream::Stdout);
        assert!(sub.matches(&event2));
        
        let event3 = ObservabilityEvent::log("test:service", LogLevel::Error, "test", LogStream::Stdout);
        assert!(sub.matches(&event3));
    }

    #[test]
    fn test_log_buffer() {
        let buffer = LogBuffer::new(5);
        
        for i in 0..10 {
            buffer.add(LogLine {
                timestamp: Utc::now(),
                service_fqn: "test:service".to_string(),
                level: LogLevel::Info,
                message: format!("message {}", i),
                stream: LogStream::Stdout,
            });
        }
        
        let logs = buffer.get("test:service", None);
        assert_eq!(logs.len(), 5);
        assert_eq!(logs[0].message, "message 5");
        assert_eq!(logs[4].message, "message 9");
    }

    #[test]
    fn test_log_buffer_limit() {
        let buffer = LogBuffer::new(100);
        
        for i in 0..10 {
            buffer.add(LogLine {
                timestamp: Utc::now(),
                service_fqn: "test:service".to_string(),
                level: LogLevel::Info,
                message: format!("message {}", i),
                stream: LogStream::Stdout,
            });
        }
        
        let logs = buffer.get("test:service", Some(3));
        assert_eq!(logs.len(), 3);
        assert_eq!(logs[0].message, "message 7");
        assert_eq!(logs[2].message, "message 9");
    }
}
