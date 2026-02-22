//! Hive Daemon Client Library
//!
//! Provides the canonical IPC protocol types and a client for communicating
//! with the Hive daemon via Unix socket. Used by hive-core (server side),
//! hive-plugin (CLI side), and core plugins (signaling_control).

use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tokio::sync::Mutex;
use tracing::debug;
use uuid::Uuid;

// Re-export types for convenience
pub use chrono;
pub use uuid;

// ============================================================================
// WIRE PROTOCOL TYPES
// ============================================================================

/// Daemon request types (canonical protocol definition).
///
/// No authentication variants — socket file permissions (0600) handle security.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DaemonRequest {
    /// Get daemon status
    Status,

    /// Shutdown the daemon
    Shutdown { graceful: bool },

    /// List all sources
    ListSources,

    /// Add a new source
    AddSource {
        path: String,
        name: Option<String>,
    },

    /// Remove a source
    RemoveSource { name: String },

    /// Reload a source configuration
    ReloadSource { name: String },

    /// Enable a source
    EnableSource { name: String },

    /// Disable a source
    DisableSource { name: String },

    /// Start all services in a source
    StartSource { name: String },

    /// Stop all services in a source
    StopSource { name: String },

    /// Start a specific service (FQN: source:service)
    StartService { fqn: String },

    /// Stop a specific service
    StopService { fqn: String },

    /// Restart a specific service
    RestartService { fqn: String },

    /// Get service status
    GetServiceStatus { fqn: String },

    /// List all services
    ListServices { source: Option<String> },

    /// Create a new service dynamically (SQLite sources only)
    CreateService {
        source_id: String,
        name: String,
        config: serde_json::Value,
    },

    /// Update a service configuration
    UpdateService {
        fqn: String,
        patch: serde_json::Value,
    },

    /// Delete a service
    DeleteService { fqn: String },

    /// List exposed services
    ListExposed,

    /// Get logs for a service or all services
    GetLogs {
        /// Service FQN (optional, if None returns all logs)
        fqn: Option<String>,
        /// Number of lines to return (default: 100)
        lines: Option<u32>,
        /// Only return logs since this timestamp
        since: Option<DateTime<Utc>>,
        /// Minimum log level
        level: Option<String>,
    },

    /// Start streaming logs (returns stream_id, then sends LogStream messages)
    StreamLogs {
        /// Service FQN pattern (supports wildcards like "source:*")
        fqn: Option<String>,
        /// Minimum log level
        level: Option<String>,
    },

    /// Stop streaming logs
    StopLogStream { stream_id: Uuid },

    /// Ping (for connection check)
    Ping,
}

/// Daemon response types (canonical protocol definition).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DaemonResponse {
    /// Success with optional message
    Ok { message: Option<String> },

    /// Error response
    Error { code: String, message: String },

    /// Daemon status
    Status(DaemonStatus),

    /// List of sources
    Sources { sources: Vec<SourceInfo> },

    /// List of services
    Services { services: Vec<ServiceStatus> },

    /// Single service details
    Service { service: ServiceStatus },

    /// List of exposed services
    Exposed { exposed: Vec<ExposedServiceInfo> },

    /// Log lines
    Logs { logs: Vec<LogLine> },

    /// Log stream started
    StreamStarted { stream_id: Uuid },

    /// Log stream line (sent during streaming)
    LogStream { stream_id: Uuid, line: LogLine },

    /// Log stream ended
    StreamEnded { stream_id: Uuid },

    /// Pong response
    Pong,
}

// ============================================================================
// DATA TYPES
// ============================================================================

/// Daemon status information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonStatus {
    pub running: bool,
    pub pid: Option<u32>,
    pub version: String,
    pub source_count: usize,
    pub running_services: usize,
    pub total_services: usize,
    pub proxy_addresses: Vec<String>,
    pub uptime_secs: u64,
}

/// Source information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceInfo {
    pub name: String,
    pub path: PathBuf,
    pub source_type: SourceType,
    pub enabled: bool,
    pub service_count: usize,
    pub status: SourceStatus,
}

/// Source type
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SourceType {
    Yaml,
    Sqlite,
}

/// Source status
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SourceStatus {
    Loaded,
    Running,
    Stopped,
    Error(String),
}

/// Service status information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceStatus {
    /// Fully qualified name (source:service)
    pub fqn: String,
    /// Source name
    pub source: String,
    /// Service name
    pub name: String,
    /// Current state (string representation for forward compatibility)
    pub state: String,
    /// Whether service is healthy
    pub healthy: Option<bool>,
    /// Process ID (if running)
    pub pid: Option<u32>,
    /// Container ID (for docker-based services)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub container_id: Option<String>,
    /// When the service was started
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub started_at: Option<DateTime<Utc>>,
    /// Assigned ports
    pub ports: HashMap<String, u16>,
    /// Restart count
    pub restart_count: u32,
}

/// Exposed service information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExposedServiceInfo {
    /// Expose name
    pub name: String,
    /// Source name
    pub source: String,
    /// Service name
    pub service: String,
    /// Whether the service is healthy
    pub healthy: bool,
    /// Exposed variable names (keys only, not values for security)
    pub var_names: Vec<String>,
    /// Exposed port names
    pub port_names: Vec<String>,
}

/// Log line
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogLine {
    pub timestamp: DateTime<Utc>,
    pub level: String,
    /// Service FQN (source:service)
    pub service_fqn: String,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fields: Option<HashMap<String, serde_json::Value>>,
}

// ============================================================================
// CLIENT IMPLEMENTATION
// ============================================================================

/// Daemon client for communicating with the Hive daemon.
///
/// Uses a persistent connection model (Arc<Mutex<ClientInner>>).
#[derive(Clone)]
pub struct DaemonClient {
    socket_path: PathBuf,
    inner: Arc<Mutex<ClientInner>>,
}

struct ClientInner {
    stream: Option<UnixStream>,
}

impl DaemonClient {
    /// Create a new daemon client with the given socket path
    pub fn new(socket_path: impl Into<PathBuf>) -> Self {
        Self {
            socket_path: socket_path.into(),
            inner: Arc::new(Mutex::new(ClientInner { stream: None })),
        }
    }

    /// Create a client with default socket path (~/.adi/hive/hive.sock)
    pub fn new_default() -> Result<Self> {
        let socket_path = dirs::home_dir()
            .ok_or_else(|| anyhow!("Could not determine home directory"))?
            .join(".adi/hive/hive.sock");

        Ok(Self::new(socket_path))
    }

    /// Get the socket path
    pub fn socket_path(&self) -> &Path {
        &self.socket_path
    }

    /// Connect to the daemon (lazy connection)
    async fn ensure_connected(&self) -> Result<()> {
        let mut inner = self.inner.lock().await;

        if inner.stream.is_none() {
            debug!("Connecting to daemon at {:?}", self.socket_path);

            let stream = UnixStream::connect(&self.socket_path)
                .await
                .with_context(|| {
                    format!(
                        "Failed to connect to daemon at {}. Is the daemon running?",
                        self.socket_path.display()
                    )
                })?;

            inner.stream = Some(stream);
            debug!("Connected to daemon");
        }

        Ok(())
    }

    /// Send a request and wait for response (alias for `request`)
    pub async fn send(&self, req: DaemonRequest) -> Result<DaemonResponse> {
        self.request(req).await
    }

    /// Send a request and wait for response
    pub async fn request(&self, req: DaemonRequest) -> Result<DaemonResponse> {
        self.ensure_connected().await?;

        let mut inner = self.inner.lock().await;
        let stream = inner
            .stream
            .as_mut()
            .ok_or_else(|| anyhow!("Not connected to daemon"))?;

        let json = serde_json::to_string(&req).with_context(|| "Failed to serialize request")?;

        debug!("Sending request: {}", json);

        stream.write_all(json.as_bytes()).await?;
        stream.write_all(b"\n").await?;
        stream.flush().await?;

        let mut reader = BufReader::new(stream);
        let mut response_line = String::new();
        reader
            .read_line(&mut response_line)
            .await
            .with_context(|| "Failed to read response from daemon")?;

        if response_line.is_empty() {
            return Err(anyhow!("Daemon closed connection"));
        }

        let response: DaemonResponse = serde_json::from_str(&response_line)
            .with_context(|| format!("Failed to parse daemon response: {}", response_line))?;

        debug!("Received response: {:?}", response);

        Ok(response)
    }

    /// Send a request with a custom timeout
    pub async fn request_with_timeout(
        &self,
        req: DaemonRequest,
        timeout: Duration,
    ) -> Result<DaemonResponse> {
        tokio::time::timeout(timeout, self.request(req))
            .await
            .map_err(|_| anyhow!("Request timed out"))?
    }

    /// Send a request and extract an expected response variant.
    /// Handles DaemonResponse::Error uniformly.
    async fn extract<T>(
        &self,
        req: DaemonRequest,
        f: impl FnOnce(DaemonResponse) -> Option<T>,
    ) -> Result<T> {
        match self.request(req).await? {
            DaemonResponse::Error { code, message } => {
                Err(anyhow!("Daemon error [{}]: {}", code, message))
            }
            resp => f(resp).ok_or_else(|| anyhow!("Unexpected response")),
        }
    }

    /// Send a request with custom timeout and extract the expected response variant.
    async fn extract_with_timeout<T>(
        &self,
        req: DaemonRequest,
        timeout: Duration,
        f: impl FnOnce(DaemonResponse) -> Option<T>,
    ) -> Result<T> {
        match self.request_with_timeout(req, timeout).await? {
            DaemonResponse::Error { code, message } => {
                Err(anyhow!("Daemon error [{}]: {}", code, message))
            }
            resp => f(resp).ok_or_else(|| anyhow!("Unexpected response")),
        }
    }

    /// Send a request that expects DaemonResponse::Ok
    async fn expect_ok(&self, req: DaemonRequest) -> Result<()> {
        self.extract(req, |r| matches!(r, DaemonResponse::Ok { .. }).then_some(()))
            .await
    }

    /// Send a request with custom timeout that expects DaemonResponse::Ok
    async fn expect_ok_with_timeout(&self, req: DaemonRequest, timeout: Duration) -> Result<()> {
        self.extract_with_timeout(req, timeout, |r| {
            matches!(r, DaemonResponse::Ok { .. }).then_some(())
        })
        .await
    }

    // ====================================================================
    // Convenience methods
    // ====================================================================

    /// Check if daemon is running (ping)
    pub async fn ping(&self) -> Result<bool> {
        match self.request(DaemonRequest::Ping).await {
            Ok(DaemonResponse::Pong) => Ok(true),
            Ok(_) => Ok(false),
            Err(_) => Ok(false),
        }
    }

    /// Get daemon status
    pub async fn status(&self) -> Result<DaemonStatus> {
        self.extract(DaemonRequest::Status, |r| match r {
            DaemonResponse::Status(s) => Some(s),
            _ => None,
        })
        .await
    }

    /// List all sources
    pub async fn list_sources(&self) -> Result<Vec<SourceInfo>> {
        self.extract(DaemonRequest::ListSources, |r| match r {
            DaemonResponse::Sources { sources } => Some(sources),
            _ => None,
        })
        .await
    }

    /// List all services (optionally filtered by source)
    pub async fn list_services(&self, source: Option<&str>) -> Result<Vec<ServiceStatus>> {
        self.extract(
            DaemonRequest::ListServices {
                source: source.map(String::from),
            },
            |r| match r {
                DaemonResponse::Services { services } => Some(services),
                _ => None,
            },
        )
        .await
    }

    /// Get service status by FQN (source:service)
    pub async fn get_service_status(&self, fqn: &str) -> Result<Option<ServiceStatus>> {
        self.extract(
            DaemonRequest::GetServiceStatus {
                fqn: fqn.to_string(),
            },
            |r| match r {
                DaemonResponse::Service { service } => Some(Some(service)),
                DaemonResponse::Services { services } => Some(services.into_iter().next()),
                DaemonResponse::Error { code, .. } if code == "NOT_FOUND" => Some(None),
                _ => None,
            },
        )
        .await
    }

    /// Add a source
    pub async fn add_source(&self, path: &str, name: Option<&str>) -> Result<String> {
        self.extract(
            DaemonRequest::AddSource {
                path: path.to_string(),
                name: name.map(String::from),
            },
            |r| match r {
                DaemonResponse::Ok { message } => {
                    Some(message.unwrap_or_else(|| "Source added".to_string()))
                }
                _ => None,
            },
        )
        .await
    }

    /// Remove a source
    pub async fn remove_source(&self, name: &str) -> Result<()> {
        self.expect_ok(DaemonRequest::RemoveSource {
            name: name.to_string(),
        })
        .await
    }

    /// Start a source
    pub async fn start_source(&self, name: &str) -> Result<()> {
        self.expect_ok_with_timeout(
            DaemonRequest::StartSource {
                name: name.to_string(),
            },
            Duration::from_secs(5 * 60),
        )
        .await
    }

    /// Stop a source
    pub async fn stop_source(&self, name: &str) -> Result<()> {
        self.expect_ok_with_timeout(
            DaemonRequest::StopSource {
                name: name.to_string(),
            },
            Duration::from_secs(2 * 60),
        )
        .await
    }

    /// Shutdown the daemon
    pub async fn shutdown(&self, graceful: bool) -> Result<()> {
        self.expect_ok_with_timeout(
            DaemonRequest::Shutdown { graceful },
            Duration::from_secs(2 * 60),
        )
        .await
    }

    /// Start a service
    pub async fn start_service(&self, fqn: &str) -> Result<()> {
        self.expect_ok_with_timeout(
            DaemonRequest::StartService {
                fqn: fqn.to_string(),
            },
            Duration::from_secs(5 * 60),
        )
        .await
    }

    /// Stop a service
    pub async fn stop_service(&self, fqn: &str) -> Result<()> {
        self.expect_ok_with_timeout(
            DaemonRequest::StopService {
                fqn: fqn.to_string(),
            },
            Duration::from_secs(2 * 60),
        )
        .await
    }

    /// Restart a service
    pub async fn restart_service(&self, fqn: &str) -> Result<()> {
        self.expect_ok_with_timeout(
            DaemonRequest::RestartService {
                fqn: fqn.to_string(),
            },
            Duration::from_secs(5 * 60),
        )
        .await
    }

    /// Create a new service dynamically
    pub async fn create_service(
        &self,
        source_id: &str,
        name: &str,
        config: serde_json::Value,
    ) -> Result<()> {
        self.expect_ok(DaemonRequest::CreateService {
            source_id: source_id.to_string(),
            name: name.to_string(),
            config,
        })
        .await
    }

    /// Update a service configuration
    pub async fn update_service(&self, fqn: &str, patch: serde_json::Value) -> Result<()> {
        self.expect_ok(DaemonRequest::UpdateService {
            fqn: fqn.to_string(),
            patch,
        })
        .await
    }

    /// Delete a service
    pub async fn delete_service(&self, fqn: &str) -> Result<()> {
        self.expect_ok(DaemonRequest::DeleteService {
            fqn: fqn.to_string(),
        })
        .await
    }

    /// Get logs
    pub async fn get_logs(
        &self,
        fqn: Option<&str>,
        lines: Option<u32>,
        since: Option<DateTime<Utc>>,
        level: Option<&str>,
    ) -> Result<Vec<LogLine>> {
        self.extract(
            DaemonRequest::GetLogs {
                fqn: fqn.map(String::from),
                lines,
                since,
                level: level.map(String::from),
            },
            |r| match r {
                DaemonResponse::Logs { logs } => Some(logs),
                _ => None,
            },
        )
        .await
    }

    /// Start streaming logs, returning a handle for receiving log lines.
    ///
    /// Opens a dedicated connection for streaming (separate from the
    /// request/response connection) so that log lines can be read
    /// concurrently with other requests.
    pub async fn stream_logs(
        &self,
        fqn: Option<&str>,
        level: Option<&str>,
    ) -> Result<LogStreamHandle> {
        let stream = UnixStream::connect(&self.socket_path)
            .await
            .with_context(|| {
                format!(
                    "Failed to connect to daemon at {}. Is the daemon running?",
                    self.socket_path.display()
                )
            })?;

        let (reader, mut writer) = stream.into_split();
        let mut reader = BufReader::new(reader);

        let request = DaemonRequest::StreamLogs {
            fqn: fqn.map(String::from),
            level: level.map(String::from),
        };
        let request_json = serde_json::to_string(&request)?;
        writer.write_all(request_json.as_bytes()).await?;
        writer.write_all(b"\n").await?;

        let mut response_line = String::new();
        reader.read_line(&mut response_line).await?;

        let response: DaemonResponse = serde_json::from_str(response_line.trim())
            .with_context(|| "Invalid response from daemon")?;

        let stream_id = match response {
            DaemonResponse::StreamStarted { stream_id } => stream_id,
            DaemonResponse::Error { code, message } => {
                return Err(anyhow!("Daemon error [{}]: {}", code, message));
            }
            _ => return Err(anyhow!("Unexpected response")),
        };

        Ok(LogStreamHandle {
            stream_id,
            reader,
            writer,
        })
    }

    /// Disconnect from daemon
    pub async fn disconnect(&self) {
        let mut inner = self.inner.lock().await;
        if let Some(stream) = inner.stream.take() {
            drop(stream);
            debug!("Disconnected from daemon");
        }
    }
}

/// Handle for streaming logs from the daemon.
///
/// Uses a dedicated Unix socket connection so log lines can be
/// received independently of other daemon requests.
pub struct LogStreamHandle {
    stream_id: Uuid,
    reader: BufReader<tokio::net::unix::OwnedReadHalf>,
    writer: tokio::net::unix::OwnedWriteHalf,
}

impl LogStreamHandle {
    /// Get the stream ID
    pub fn stream_id(&self) -> Uuid {
        self.stream_id
    }

    /// Receive the next log line, or `None` when the stream ends.
    pub async fn recv(&mut self) -> Result<Option<LogLine>> {
        let mut line = String::new();
        let bytes_read = self.reader.read_line(&mut line).await?;

        if bytes_read == 0 {
            return Ok(None);
        }

        let response: DaemonResponse = serde_json::from_str(line.trim())
            .with_context(|| "Invalid response from daemon")?;

        match response {
            DaemonResponse::LogStream { line, .. } => Ok(Some(line)),
            DaemonResponse::StreamEnded { .. } => Ok(None),
            DaemonResponse::Error { code, message } => {
                Err(anyhow!("Daemon error [{}]: {}", code, message))
            }
            _ => Err(anyhow!("Unexpected response")),
        }
    }

    /// Stop the log stream
    pub async fn stop(mut self) -> Result<()> {
        let request = DaemonRequest::StopLogStream {
            stream_id: self.stream_id,
        };
        let request_json = serde_json::to_string(&request)?;
        self.writer.write_all(request_json.as_bytes()).await?;
        self.writer.write_all(b"\n").await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_client_creation() {
        let client = DaemonClient::new(PathBuf::from("/tmp/test.sock"));
        assert!(client.socket_path.to_str().unwrap().contains("test.sock"));
    }

    #[test]
    fn test_request_serialization() {
        let req = DaemonRequest::Status;
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("status"));
    }

    #[test]
    fn test_response_deserialization() {
        let json = r#"{"type":"ok","message":"test"}"#;
        let resp: DaemonResponse = serde_json::from_str(json).unwrap();
        match resp {
            DaemonResponse::Ok { message } => {
                assert_eq!(message, Some("test".to_string()));
            }
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn test_service_status_with_optional_fields() {
        // Verify optional fields deserialize correctly when absent
        let json = r#"{"fqn":"src:svc","source":"src","name":"svc","state":"running","healthy":true,"pid":123,"ports":{"http":8080},"restart_count":0}"#;
        let status: ServiceStatus = serde_json::from_str(json).unwrap();
        assert_eq!(status.fqn, "src:svc");
        assert!(status.container_id.is_none());
        assert!(status.started_at.is_none());
    }

    #[test]
    fn test_log_line_serialization() {
        let line = LogLine {
            timestamp: Utc::now(),
            level: "info".to_string(),
            service_fqn: "default:api".to_string(),
            message: "Server started".to_string(),
            fields: None,
        };
        let json = serde_json::to_string(&line).unwrap();
        assert!(json.contains("service_fqn"));
        assert!(json.contains("Server started"));
    }
}
