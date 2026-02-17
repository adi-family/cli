//! Hive Daemon Client Library
//!
//! Provides a client for communicating with the Hive daemon via Unix socket.
//! Used by core plugins to interact with the orchestrator.

use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tokio::sync::Mutex;
use tracing::debug;
use uuid::Uuid;

// Re-export types for convenience
pub use chrono;
pub use uuid;

/// Daemon request types
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
        name: Option<String>
    },

    /// Remove a source
    RemoveSource {
        name: String
    },

    /// Reload a source configuration
    ReloadSource {
        name: String
    },

    /// Enable a source
    EnableSource {
        name: String
    },

    /// Disable a source
    DisableSource {
        name: String
    },

    /// Start all services in a source
    StartSource {
        name: String
    },

    /// Stop all services in a source
    StopSource {
        name: String
    },

    /// Start a specific service (FQN: source:service)
    StartService {
        fqn: String
    },

    /// Stop a specific service
    StopService {
        fqn: String
    },

    /// Restart a specific service
    RestartService {
        fqn: String
    },

    /// Get service status
    GetServiceStatus {
        fqn: String
    },

    /// List all services
    ListServices {
        source: Option<String>
    },

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
    DeleteService {
        fqn: String,
    },

    /// List exposed services
    ListExposed,

    /// Get logs for a service or all services
    GetLogs {
        fqn: Option<String>,
        lines: Option<u32>,
        since: Option<DateTime<Utc>>,
        level: Option<String>,
    },

    /// Start streaming logs
    StreamLogs {
        fqn: Option<String>,
        level: Option<String>,
    },

    /// Stop streaming logs
    StopLogStream {
        stream_id: Uuid
    },

    /// Ping (for connection check)
    Ping,
}

/// Daemon response types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DaemonResponse {
    /// Success with optional message
    Ok {
        message: Option<String>
    },

    /// Error response
    Error {
        code: String,
        message: String
    },

    /// Daemon status
    Status(DaemonStatus),

    /// List of sources
    Sources {
        sources: Vec<SourceInfo>
    },

    /// List of services
    Services {
        services: Vec<ServiceStatus>
    },

    /// Service details
    Service {
        service: ServiceStatus,
    },

    /// List of exposed services
    Exposed {
        exposed: Vec<ExposedServiceInfo>
    },

    /// Log lines
    Logs {
        logs: Vec<LogLine>
    },

    /// Log stream started
    StreamStarted {
        stream_id: Uuid
    },

    /// Log stream line (sent during streaming)
    LogStream {
        stream_id: Uuid,
        line: LogLine
    },

    /// Log stream stopped
    StreamStopped {
        stream_id: Uuid
    },

    /// Pong response
    Pong,
}

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
    pub fqn: String,
    pub source: String,
    pub name: String,
    pub state: ServiceState,
    pub pid: Option<u32>,
    pub container_id: Option<String>,
    pub started_at: Option<DateTime<Utc>>,
    pub healthy: Option<bool>,
    pub restart_count: u32,
    pub ports: HashMap<String, u16>,
}

/// Service state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ServiceState {
    Stopped,
    Starting,
    Running,
    Stopping,
    Crashed,
    Restarting,
}

/// Exposed service information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExposedServiceInfo {
    pub name: String,
    pub source: String,
    pub service: String,
    pub has_secret: bool,
    pub vars: Vec<String>,
    pub consumers: Vec<String>,
}

/// Log line
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogLine {
    pub timestamp: DateTime<Utc>,
    pub level: String,
    pub fqn: String,
    pub message: String,
    pub fields: Option<HashMap<String, serde_json::Value>>,
}

/// Daemon client for communicating with the Hive daemon
#[derive(Clone)]
pub struct DaemonClient {
    socket_path: PathBuf,
    inner: Arc<Mutex<ClientInner>>,
}

struct ClientInner {
    stream: Option<UnixStream>,
}

impl DaemonClient {
    /// Create a new daemon client
    pub fn new(socket_path: PathBuf) -> Self {
        Self {
            socket_path,
            inner: Arc::new(Mutex::new(ClientInner {
                stream: None,
            })),
        }
    }

    /// Create a client with default socket path
    pub fn new_default() -> Result<Self> {
        let socket_path = dirs::home_dir()
            .ok_or_else(|| anyhow!("Could not determine home directory"))?
            .join(".adi/hive/hive.sock");

        Ok(Self::new(socket_path))
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

    /// Send a request and wait for response
    pub async fn request(&self, req: DaemonRequest) -> Result<DaemonResponse> {
        self.ensure_connected().await?;

        let mut inner = self.inner.lock().await;
        let stream = inner.stream.as_mut()
            .ok_or_else(|| anyhow!("Not connected to daemon"))?;

        // Serialize request
        let json = serde_json::to_string(&req)
            .with_context(|| "Failed to serialize request")?;

        debug!("Sending request: {}", json);

        // Send request (newline-delimited JSON)
        stream.write_all(json.as_bytes()).await?;
        stream.write_all(b"\n").await?;
        stream.flush().await?;

        // Read response
        let mut reader = BufReader::new(stream);
        let mut response_line = String::new();
        reader.read_line(&mut response_line).await
            .with_context(|| "Failed to read response from daemon")?;

        if response_line.is_empty() {
            return Err(anyhow!("Daemon closed connection"));
        }

        // Deserialize response
        let response: DaemonResponse = serde_json::from_str(&response_line)
            .with_context(|| format!("Failed to parse daemon response: {}", response_line))?;

        debug!("Received response: {:?}", response);

        Ok(response)
    }

    /// Check if daemon is running
    pub async fn ping(&self) -> Result<bool> {
        match self.request(DaemonRequest::Ping).await {
            Ok(DaemonResponse::Pong) => Ok(true),
            Ok(_) => Ok(false),
            Err(_) => Ok(false),
        }
    }

    /// Get daemon status
    pub async fn status(&self) -> Result<DaemonStatus> {
        match self.request(DaemonRequest::Status).await? {
            DaemonResponse::Status(status) => Ok(status),
            DaemonResponse::Error { code, message } => {
                Err(anyhow!("Daemon error {}: {}", code, message))
            }
            _ => Err(anyhow!("Unexpected response")),
        }
    }

    /// List all sources
    pub async fn list_sources(&self) -> Result<Vec<SourceInfo>> {
        match self.request(DaemonRequest::ListSources).await? {
            DaemonResponse::Sources { sources } => Ok(sources),
            DaemonResponse::Error { code, message } => {
                Err(anyhow!("Daemon error {}: {}", code, message))
            }
            _ => Err(anyhow!("Unexpected response")),
        }
    }

    /// List all services (optionally filtered by source)
    pub async fn list_services(&self, source: Option<String>) -> Result<Vec<ServiceStatus>> {
        match self.request(DaemonRequest::ListServices { source }).await? {
            DaemonResponse::Services { services } => Ok(services),
            DaemonResponse::Error { code, message } => {
                Err(anyhow!("Daemon error {}: {}", code, message))
            }
            _ => Err(anyhow!("Unexpected response")),
        }
    }

    /// Get service status
    pub async fn get_service_status(&self, fqn: String) -> Result<ServiceStatus> {
        match self.request(DaemonRequest::GetServiceStatus { fqn }).await? {
            DaemonResponse::Service { service } => Ok(service),
            DaemonResponse::Error { code, message } => {
                Err(anyhow!("Daemon error {}: {}", code, message))
            }
            _ => Err(anyhow!("Unexpected response")),
        }
    }

    /// Create a new service dynamically (SQLite sources only)
    pub async fn create_service(
        &self,
        source_id: String,
        name: String,
        config: serde_json::Value,
    ) -> Result<()> {
        match self.request(DaemonRequest::CreateService {
            source_id,
            name,
            config
        }).await? {
            DaemonResponse::Ok { .. } => Ok(()),
            DaemonResponse::Error { code, message } => {
                Err(anyhow!("Daemon error {}: {}", code, message))
            }
            _ => Err(anyhow!("Unexpected response")),
        }
    }

    /// Update a service configuration
    pub async fn update_service(
        &self,
        fqn: String,
        patch: serde_json::Value,
    ) -> Result<()> {
        match self.request(DaemonRequest::UpdateService { fqn, patch }).await? {
            DaemonResponse::Ok { .. } => Ok(()),
            DaemonResponse::Error { code, message } => {
                Err(anyhow!("Daemon error {}: {}", code, message))
            }
            _ => Err(anyhow!("Unexpected response")),
        }
    }

    /// Delete a service
    pub async fn delete_service(&self, fqn: String) -> Result<()> {
        match self.request(DaemonRequest::DeleteService { fqn }).await? {
            DaemonResponse::Ok { .. } => Ok(()),
            DaemonResponse::Error { code, message } => {
                Err(anyhow!("Daemon error {}: {}", code, message))
            }
            _ => Err(anyhow!("Unexpected response")),
        }
    }

    /// Start a service
    pub async fn start_service(&self, fqn: String) -> Result<()> {
        match self.request(DaemonRequest::StartService { fqn }).await? {
            DaemonResponse::Ok { .. } => Ok(()),
            DaemonResponse::Error { code, message } => {
                Err(anyhow!("Daemon error {}: {}", code, message))
            }
            _ => Err(anyhow!("Unexpected response")),
        }
    }

    /// Stop a service
    pub async fn stop_service(&self, fqn: String) -> Result<()> {
        match self.request(DaemonRequest::StopService { fqn }).await? {
            DaemonResponse::Ok { .. } => Ok(()),
            DaemonResponse::Error { code, message } => {
                Err(anyhow!("Daemon error {}: {}", code, message))
            }
            _ => Err(anyhow!("Unexpected response")),
        }
    }

    /// Restart a service
    pub async fn restart_service(&self, fqn: String) -> Result<()> {
        match self.request(DaemonRequest::RestartService { fqn }).await? {
            DaemonResponse::Ok { .. } => Ok(()),
            DaemonResponse::Error { code, message } => {
                Err(anyhow!("Daemon error {}: {}", code, message))
            }
            _ => Err(anyhow!("Unexpected response")),
        }
    }

    /// Get logs for a service
    pub async fn get_logs(
        &self,
        fqn: Option<String>,
        lines: Option<u32>,
        since: Option<DateTime<Utc>>,
        level: Option<String>,
    ) -> Result<Vec<LogLine>> {
        match self.request(DaemonRequest::GetLogs {
            fqn,
            lines,
            since,
            level
        }).await? {
            DaemonResponse::Logs { logs } => Ok(logs),
            DaemonResponse::Error { code, message } => {
                Err(anyhow!("Daemon error {}: {}", code, message))
            }
            _ => Err(anyhow!("Unexpected response")),
        }
    }

    /// Stream logs from a service
    pub async fn stream_logs(
        &self,
        fqn: Option<String>,
        level: Option<String>,
    ) -> Result<LogStream> {
        match self.request(DaemonRequest::StreamLogs { fqn, level }).await? {
            DaemonResponse::StreamStarted { stream_id } => {
                Ok(LogStream {
                    stream_id,
                    client: self.clone(),
                })
            }
            DaemonResponse::Error { code, message } => {
                Err(anyhow!("Daemon error {}: {}", code, message))
            }
            _ => Err(anyhow!("Unexpected response")),
        }
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

/// Log stream handle
pub struct LogStream {
    stream_id: Uuid,
    client: DaemonClient,
}

impl LogStream {
    /// Stop the log stream
    pub async fn stop(self) -> Result<()> {
        match self.client.request(DaemonRequest::StopLogStream {
            stream_id: self.stream_id
        }).await? {
            DaemonResponse::StreamStopped { .. } => Ok(()),
            DaemonResponse::Error { code, message } => {
                Err(anyhow!("Daemon error {}: {}", code, message))
            }
            _ => Err(anyhow!("Unexpected response")),
        }
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
}
