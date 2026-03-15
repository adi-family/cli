//! Remote Control via Signaling Server
//!
//! Implements Section 21 of the Hive YAML spec - remote control of Hive
//! daemon via the signaling server WebSocket connection.

use crate::hive_config::ServiceConfig;
use crate::observability::LogLine;
use crate::source_manager::SourceInfo;
use crate::sqlite_backend::ServicePatch;
use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc, RwLock};
use tracing::{debug, info, warn};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum HiveRequest {
    // Daemon operations
    GetStatus,
    Shutdown { graceful: bool },

    // Source operations
    ListSources,
    AddSource {
        path: String,
        name: Option<String>,
    },
    RemoveSource {
        source_id: String,
    },
    ReloadSource {
        source_id: String,
    },
    EnableSource {
        source_id: String,
    },
    DisableSource {
        source_id: String,
    },

    // Service operations
    ListServices {
        source: Option<String>,
    },
    GetService {
        fqn: String,
    },
    StartService {
        fqn: String,
    },
    StopService {
        fqn: String,
    },
    RestartService {
        fqn: String,
    },
    StartSource {
        source_id: String,
    },
    StopSource {
        source_id: String,
    },

    // Logs
    GetLogs {
        fqn: String,
        lines: u32,
        since: Option<DateTime<Utc>>,
    },
    StreamLogs {
        fqn: String,
    },
    StopLogStream {
        stream_id: Uuid,
    },

    // Config (SQLite sources only)
    GetSourceConfig {
        source_id: String,
    },
    CreateService {
        source_id: String,
        name: String,
        config: ServiceConfig,
    },
    UpdateService {
        fqn: String,
        patch: ServicePatch,
    },
    DeleteService {
        fqn: String,
    },

    // Exposed services
    ListExposed,
    GetExposedUsers {
        name: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum HiveResponse {
    Status(DaemonStatus),
    Sources(Vec<SourceInfo>),
    Services(Vec<ServiceInfo>),
    Service(ServiceInfo),
    Config(SourceConfig),
    Logs(Vec<LogLine>),
    StreamStarted { stream_id: Uuid },
    StreamStopped { stream_id: Uuid },
    Exposed(Vec<ExposedInfo>),
    Ok,
    Error { code: ErrorCode, message: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonStatus {
    pub version: String,
    pub uptime_secs: u64,
    pub sources_count: usize,
    pub services_running: usize,
    pub services_total: usize,
    pub proxy_bind: Vec<String>,
    pub signaling_connected: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceInfo {
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceConfig {
    pub source_id: String,
    pub source_type: String,
    pub version: String,
    pub services: HashMap<String, ServiceConfig>,
    pub defaults: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExposedInfo {
    pub name: String,
    pub source: String,
    pub service: String,
    pub has_secret: bool,
    pub vars: Vec<String>,
    pub consumers: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    NotFound,
    InvalidRequest,
    SourceConflict,
    ServiceConflict,
    Unauthorized,
    InternalError,
    NotSupported,
}

pub struct LogStream {
    pub stream_id: Uuid,
    pub fqn: String,
    pub tx: mpsc::Sender<Vec<LogLine>>,
}

pub struct RemoteControlHandler {
    log_streams: Arc<RwLock<HashMap<Uuid, LogStream>>>,
    request_handler: Option<Arc<dyn RequestHandler + Send + Sync>>,
    shutdown_tx: Option<broadcast::Sender<()>>,
}

#[async_trait::async_trait]
pub trait RequestHandler {
    async fn handle(&self, request: HiveRequest) -> HiveResponse;
}

impl RemoteControlHandler {
    pub fn new() -> Self {
        Self {
            log_streams: Arc::new(RwLock::new(HashMap::new())),
            request_handler: None,
            shutdown_tx: None,
        }
    }

    pub fn set_handler<H: RequestHandler + Send + Sync + 'static>(&mut self, handler: H) {
        self.request_handler = Some(Arc::new(handler));
    }

    pub fn set_shutdown_tx(&mut self, tx: broadcast::Sender<()>) {
        self.shutdown_tx = Some(tx);
    }

    pub async fn handle_request(&self, request: HiveRequest) -> HiveResponse {
        debug!("Handling remote control request: {:?}", request);

        match &request {
            HiveRequest::StreamLogs { fqn } => {
                return self.start_log_stream(fqn.clone()).await;
            }
            HiveRequest::StopLogStream { stream_id } => {
                return self.stop_log_stream(*stream_id).await;
            }
            HiveRequest::Shutdown { graceful } => {
                return self.handle_shutdown(*graceful).await;
            }
            _ => {}
        }

        if let Some(handler) = &self.request_handler {
            handler.handle(request).await
        } else {
            HiveResponse::Error {
                code: ErrorCode::InternalError,
                message: "No request handler configured".to_string(),
            }
        }
    }

    async fn start_log_stream(&self, fqn: String) -> HiveResponse {
        let stream_id = Uuid::new_v4();
        let (tx, _rx) = mpsc::channel(100);

        let stream = LogStream {
            stream_id,
            fqn: fqn.clone(),
            tx,
        };

        let mut streams = self.log_streams.write().await;
        streams.insert(stream_id, stream);

        info!("Started log stream {} for {}", stream_id, fqn);
        HiveResponse::StreamStarted { stream_id }
    }

    async fn stop_log_stream(&self, stream_id: Uuid) -> HiveResponse {
        let mut streams = self.log_streams.write().await;

        if streams.remove(&stream_id).is_some() {
            info!("Stopped log stream {}", stream_id);
            HiveResponse::StreamStopped { stream_id }
        } else {
            HiveResponse::Error {
                code: ErrorCode::NotFound,
                message: format!("Log stream {} not found", stream_id),
            }
        }
    }

    async fn handle_shutdown(&self, graceful: bool) -> HiveResponse {
        if let Some(tx) = &self.shutdown_tx {
            info!(
                "Received remote shutdown request (graceful={})",
                graceful
            );
            let _ = tx.send(());
            HiveResponse::Ok
        } else {
            HiveResponse::Error {
                code: ErrorCode::InternalError,
                message: "Shutdown not configured".to_string(),
            }
        }
    }

    pub async fn push_logs(&self, fqn: &str, logs: Vec<LogLine>) {
        let streams = self.log_streams.read().await;

        for stream in streams.values() {
            if stream.fqn == fqn || stream.fqn == "*" {
                if stream.tx.send(logs.clone()).await.is_err() {
                    warn!("Failed to send logs to stream {}", stream.stream_id);
                }
            }
        }
    }

    pub async fn active_streams(&self) -> usize {
        self.log_streams.read().await.len()
    }
}

impl Default for RemoteControlHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HiveRegistration {
    pub secret: String,
    pub device_id: Option<String>,
    pub version: String,
    pub device_type: HiveDeviceType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HiveDeviceType {
    pub name: String,
    pub sources: Vec<SourceInfo>,
    /// Cocoon kinds this hive can spawn (empty if cocoon support is disabled).
    #[serde(default)]
    pub cocoon_kinds: Vec<CocoonKindInfo>,
}

/// Minimal cocoon kind info advertised by the hive.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CocoonKindInfo {
    pub id: String,
    pub image: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum HiveControlMessage {
    Request {
        request_id: Uuid,
        payload: HiveRequest,
    },
    Response {
        request_id: Uuid,
        payload: HiveResponse,
    },
    LogStream {
        stream_id: Uuid,
        lines: Vec<LogLine>,
    },
}

/// Parses `source:service` format.
pub fn parse_fqn(fqn: &str) -> Result<(String, String)> {
    let parts: Vec<&str> = fqn.splitn(2, ':').collect();

    if parts.len() == 2 {
        Ok((parts[0].to_string(), parts[1].to_string()))
    } else {
        Err(anyhow!(
            "Invalid FQN format: '{}'. Expected 'source:service'",
            fqn
        ))
    }
}

pub struct SimpleRequestHandler {
    daemon_status: Arc<RwLock<DaemonStatus>>,
    services: Arc<RwLock<HashMap<String, ServiceInfo>>>,
}

impl SimpleRequestHandler {
    pub fn new(status: DaemonStatus) -> Self {
        Self {
            daemon_status: Arc::new(RwLock::new(status)),
            services: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn add_service(&self, info: ServiceInfo) {
        let mut services = self.services.write().await;
        services.insert(info.fqn.clone(), info);
    }

    pub async fn update_status<F>(&self, f: F)
    where
        F: FnOnce(&mut DaemonStatus),
    {
        let mut status = self.daemon_status.write().await;
        f(&mut status);
    }
}

#[async_trait::async_trait]
impl RequestHandler for SimpleRequestHandler {
    async fn handle(&self, request: HiveRequest) -> HiveResponse {
        match request {
            HiveRequest::GetStatus => {
                let status = self.daemon_status.read().await.clone();
                HiveResponse::Status(status)
            }

            HiveRequest::ListSources => {
                // Return empty for now
                HiveResponse::Sources(vec![])
            }

            HiveRequest::ListServices { source } => {
                let services = self.services.read().await;
                let filtered: Vec<ServiceInfo> = services
                    .values()
                    .filter(|s| source.as_ref().map(|src| s.source == *src).unwrap_or(true))
                    .cloned()
                    .collect();
                HiveResponse::Services(filtered)
            }

            HiveRequest::GetService { fqn } => {
                let services = self.services.read().await;
                match services.get(&fqn) {
                    Some(service) => HiveResponse::Service(service.clone()),
                    None => HiveResponse::Error {
                        code: ErrorCode::NotFound,
                        message: format!("Service '{}' not found", fqn),
                    },
                }
            }

            HiveRequest::StartService { fqn } => {
                let mut services = self.services.write().await;
                if let Some(service) = services.get_mut(&fqn) {
                    service.state = ServiceState::Running;
                    service.started_at = Some(Utc::now());
                    HiveResponse::Ok
                } else {
                    HiveResponse::Error {
                        code: ErrorCode::NotFound,
                        message: format!("Service '{}' not found", fqn),
                    }
                }
            }

            HiveRequest::StopService { fqn } => {
                let mut services = self.services.write().await;
                if let Some(service) = services.get_mut(&fqn) {
                    service.state = ServiceState::Stopped;
                    HiveResponse::Ok
                } else {
                    HiveResponse::Error {
                        code: ErrorCode::NotFound,
                        message: format!("Service '{}' not found", fqn),
                    }
                }
            }

            HiveRequest::RestartService { fqn } => {
                let mut services = self.services.write().await;
                if let Some(service) = services.get_mut(&fqn) {
                    service.state = ServiceState::Restarting;
                    service.restart_count += 1;
                    HiveResponse::Ok
                } else {
                    HiveResponse::Error {
                        code: ErrorCode::NotFound,
                        message: format!("Service '{}' not found", fqn),
                    }
                }
            }

            HiveRequest::ListExposed => {
                // Return empty for now
                HiveResponse::Exposed(vec![])
            }

            _ => HiveResponse::Error {
                code: ErrorCode::NotSupported,
                message: "Operation not supported by simple handler".to_string(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_fqn() {
        let (source, service) = parse_fqn("default:postgres").unwrap();
        assert_eq!(source, "default");
        assert_eq!(service, "postgres");

        let (source, service) = parse_fqn("my-project:auth-service").unwrap();
        assert_eq!(source, "my-project");
        assert_eq!(service, "auth-service");

        assert!(parse_fqn("postgres").is_err());
    }

    #[tokio::test]
    async fn test_remote_control_handler() {
        let mut handler = RemoteControlHandler::new();

        let status = DaemonStatus {
            version: "0.1.0".to_string(),
            uptime_secs: 100,
            sources_count: 2,
            services_running: 5,
            services_total: 10,
            proxy_bind: vec!["0.0.0.0:80".to_string()],
            signaling_connected: true,
        };

        let simple_handler = SimpleRequestHandler::new(status.clone());
        handler.set_handler(simple_handler);

        let response = handler.handle_request(HiveRequest::GetStatus).await;
        match response {
            HiveResponse::Status(s) => {
                assert_eq!(s.version, "0.1.0");
                assert_eq!(s.services_running, 5);
            }
            _ => panic!("Expected Status response"),
        }
    }

    #[tokio::test]
    async fn test_log_stream() {
        let handler = RemoteControlHandler::new();

        let response = handler
            .handle_request(HiveRequest::StreamLogs {
                fqn: "default:auth".to_string(),
            })
            .await;

        let stream_id = match response {
            HiveResponse::StreamStarted { stream_id } => stream_id,
            _ => panic!("Expected StreamStarted response"),
        };

        assert_eq!(handler.active_streams().await, 1);

        let response = handler
            .handle_request(HiveRequest::StopLogStream { stream_id })
            .await;

        match response {
            HiveResponse::StreamStopped { stream_id: id } => {
                assert_eq!(id, stream_id);
            }
            _ => panic!("Expected StreamStopped response"),
        }

        assert_eq!(handler.active_streams().await, 0);
    }

    #[test]
    fn test_serialize_request() {
        let request = HiveRequest::StartService {
            fqn: "default:postgres".to_string(),
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("start_service"));
        assert!(json.contains("default:postgres"));

        let parsed: HiveRequest = serde_json::from_str(&json).unwrap();
        match parsed {
            HiveRequest::StartService { fqn } => assert_eq!(fqn, "default:postgres"),
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn test_serialize_response() {
        let response = HiveResponse::Error {
            code: ErrorCode::NotFound,
            message: "Service not found".to_string(),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("error"));
        assert!(json.contains("not_found"));
    }
}
