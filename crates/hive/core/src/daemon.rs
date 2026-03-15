use crate::daemon_defaults;
use crate::dns::{self, DnsConfig, DnsServer};
use crate::exposure::ExposureManager;
use crate::observability::{EventCollector, EventSubscription, LogBuffer, LogLevel, LogLine};
use crate::service_proxy::start_service_proxy_server;
use crate::source_manager::{SourceInfo, SourceManager, SourceStatus};
use anyhow::{anyhow, Result};
use lib_daemon_core::{
    DaemonConfig as BaseDaemonConfig, PidFile, ShutdownCoordinator, UnixSocketServer,
};
use std::collections::HashMap;
use std::net::Ipv4Addr;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

pub use lib_hive_daemon_client::{
    DaemonClient, DaemonRequest, DaemonResponse, DaemonStatus,
    ExposedServiceInfo as WireExposedServiceInfo, LogLine as WireLogLine, LogStreamHandle,
    ServiceStatus as WireServiceStatus, ServiceStreamHandle, SourceInfo as WireSourceInfo,
    SourceStatus as WireSourceStatus, SourceType as WireSourceType,
};

type Writer = Arc<tokio::sync::Mutex<tokio::net::unix::OwnedWriteHalf>>;

pub struct DaemonConfig {
    base: BaseDaemonConfig,
    pub proxy_bind: Vec<String>,
    pub activated_listeners: Vec<std::net::TcpListener>,
    pub dns: DnsConfig,
    /// Optional signaling server connection for remote cocoon spawning.
    pub signaling: Option<crate::hive_signaling::HiveSignalingConfig>,
}

impl DaemonConfig {
    pub fn new(base_dir: impl Into<PathBuf>) -> Self {
        let base_dir = base_dir.into();
        Self {
            base: BaseDaemonConfig::new(base_dir)
                .with_socket_name(daemon_defaults::SOCKET_NAME)
                .with_pid_name(daemon_defaults::PID_NAME),
            proxy_bind: Vec::new(),
            activated_listeners: Vec::new(),
            dns: DnsConfig::default(),
            signaling: None,
        }
    }

    pub fn from_paths(data_dir: PathBuf, pid_file: PathBuf, socket_path: PathBuf) -> Self {
        let pid_name = pid_file.file_name().expect("pid_file must have a filename");
        let socket_name = socket_path.file_name().expect("socket_path must have a filename");

        Self {
            base: BaseDaemonConfig::new(data_dir)
                .with_socket_name(socket_name.to_string_lossy())
                .with_pid_name(pid_name.to_string_lossy()),
            proxy_bind: Vec::new(),
            activated_listeners: Vec::new(),
            dns: DnsConfig::default(),
            signaling: None,
        }
    }

    pub fn with_activated_listeners(mut self, listeners: Vec<std::net::TcpListener>) -> Self {
        self.activated_listeners = listeners;
        self
    }

    pub fn with_dns(mut self, dns: DnsConfig) -> Self {
        self.dns = dns;
        self
    }

    pub fn with_signaling(mut self, config: crate::hive_signaling::HiveSignalingConfig) -> Self {
        self.signaling = Some(config);
        self
    }

    pub fn socket_path(&self) -> PathBuf {
        self.base.socket_path()
    }

    pub fn pid_path(&self) -> PathBuf {
        self.base.pid_path()
    }

    pub fn base_dir(&self) -> &std::path::Path {
        self.base.base_dir()
    }
}

/// Shared state passed to each client connection handler.
struct ClientContext {
    source_manager: Arc<SourceManager>,
    exposure_manager: Arc<ExposureManager>,
    event_collector: Arc<EventCollector>,
    log_buffer: Arc<LogBuffer>,
    shutdown_handle: lib_daemon_core::ShutdownHandle,
    start_time: std::time::Instant,
    proxy_addresses: Vec<String>,
}

pub struct HiveDaemon {
    config: DaemonConfig,
    source_manager: Arc<SourceManager>,
    exposure_manager: Arc<ExposureManager>,
    event_collector: Arc<EventCollector>,
    log_buffer: Arc<LogBuffer>,
    shutdown_coordinator: tokio::sync::Mutex<Option<ShutdownCoordinator>>,
    start_time: std::time::Instant,
    dns_server: Option<Arc<DnsServer>>,
}

impl HiveDaemon {
    pub fn new(config: DaemonConfig) -> Self {
        let event_collector = Arc::new(EventCollector::new());
        let source_manager = Arc::new(SourceManager::new(event_collector.clone()));

        let dns_server = if config.dns.enabled {
            Some(Arc::new(DnsServer::new(config.dns.clone())))
        } else {
            None
        };

        Self {
            config,
            source_manager,
            exposure_manager: Arc::new(ExposureManager::new()),
            event_collector,
            log_buffer: Arc::new(LogBuffer::new(daemon_defaults::LOG_BUFFER_CAPACITY)),
            shutdown_coordinator: tokio::sync::Mutex::new(Some(ShutdownCoordinator::new())),
            start_time: std::time::Instant::now(),
            dns_server,
        }
    }

    pub fn event_collector(&self) -> Arc<EventCollector> {
        self.event_collector.clone()
    }

    pub fn log_buffer(&self) -> Arc<LogBuffer> {
        self.log_buffer.clone()
    }

    pub fn is_running(config: &DaemonConfig) -> Result<Option<u32>> {
        let pid_file = PidFile::new(config.pid_path());
        pid_file.is_running().map_err(|e| anyhow!(e))
    }

    pub async fn run(mut self) -> Result<()> {
        if let Some(pid) = Self::is_running(&self.config)? {
            return Err(anyhow!(
                "Daemon is already running (pid: {}). Use 'adi hive daemon stop' to stop it.",
                pid
            ));
        }

        let mut pid_file = PidFile::new(self.config.pid_path());
        pid_file
            .write()
            .map_err(|e| anyhow!("Failed to write PID file: {}", e))?;

        self.source_manager.init().await?;

        // Create virtual source for dynamic cocoon services if signaling is configured
        if let Some(ref signaling_config) = self.config.signaling {
            self.source_manager
                .add_virtual_source(&signaling_config.cocoon_source_id)
                .await?;
            info!(
                "Cocoon support enabled (source: '{}', kinds: {})",
                signaling_config.cocoon_source_id,
                signaling_config.cocoon_kinds.len()
            );
        }

        let proxy_state = self.source_manager.proxy_state().clone();
        proxy_state.set_log_buffer(self.log_buffer.clone());

        let activated = std::mem::take(&mut self.config.activated_listeners);
        if !activated.is_empty() || !self.config.proxy_bind.is_empty() {
            let bind_addrs: Vec<&str> = self.config.proxy_bind.iter().map(|s| s.as_str()).collect();
            start_service_proxy_server(proxy_state.clone(), &bind_addrs, activated).await?;
            info!("Proxy server started on {:?}", self.config.proxy_bind);
        }

        let mut dns_tlds = std::collections::HashSet::new();
        let dns_handle = if let Some(dns_server) = &self.dns_server {
            let routes = proxy_state.list_routes();
            let hosts: Vec<String> = routes.iter().filter_map(|r| r.host.clone()).collect();
            dns_server.sync_records(&hosts, Ipv4Addr::LOCALHOST);

            match dns::start_dns_server(dns_server.clone()) {
                Ok(handle) => {
                    dns_tlds = dns::collect_tlds(&hosts);
                    let port = extract_dns_port(&self.config.dns.bind)?;
                    if let Err(e) = dns::ensure_resolver_files(&dns_tlds, port) {
                        warn!("Failed to create DNS resolver files: {}", e);
                    }
                    Some(handle)
                }
                Err(e) => {
                    warn!("DNS server failed to start (continuing without DNS): {}", e);
                    None
                }
            }
        } else {
            None
        };

        if let Some(dns_server) = &self.dns_server {
            let dns = dns_server.clone();
            let dns_port = extract_dns_port(&self.config.dns.bind)?;
            proxy_state.set_route_sync(Arc::new(move |routes| {
                let hosts: Vec<String> = routes.iter().filter_map(|r| r.host.clone()).collect();
                dns.sync_records(&hosts, Ipv4Addr::LOCALHOST);

                let tlds = dns::collect_tlds(&hosts);
                if let Err(e) = dns::ensure_resolver_files(&tlds, dns_port) {
                    warn!("Failed to create DNS resolver files: {}", e);
                }
                dns::flush_dns_cache();
            }));
        }

        let log_buffer = self.log_buffer.clone();
        let event_collector = self.event_collector.clone();
        tokio::spawn(populate_log_buffer(event_collector, log_buffer));

        let socket_path = self.config.socket_path();
        info!("Hive daemon starting on socket: {}", socket_path.display());

        let server = UnixSocketServer::bind(&socket_path)
            .await
            .map_err(|e| anyhow!(e))?;

        info!("Hive daemon listening on {}", socket_path.display());

        let mut shutdown_coordinator = self
            .shutdown_coordinator
            .lock()
            .await
            .take()
            .ok_or_else(|| anyhow!("Daemon already started"))?;
        let shutdown_handle = shutdown_coordinator.handle();

        // Spawn signaling connection for remote cocoon management
        let signaling_handle = if let Some(signaling_config) = self.config.signaling.clone() {
            let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
            let sm = self.source_manager.clone();
            let handle = tokio::spawn(async move {
                crate::hive_signaling::run_signaling_loop(signaling_config, sm, shutdown_rx).await;
            });
            Some((handle, shutdown_tx))
        } else {
            None
        };

        let ctx = Arc::new(ClientContext {
            source_manager: self.source_manager.clone(),
            exposure_manager: self.exposure_manager.clone(),
            event_collector: self.event_collector.clone(),
            log_buffer: self.log_buffer.clone(),
            shutdown_handle,
            start_time: self.start_time,
            proxy_addresses: self.config.proxy_bind.clone(),
        });

        loop {
            tokio::select! {
                result = server.accept() => {
                    match result {
                        Ok(stream) => {
                            let ctx = ctx.clone();
                            tokio::spawn(async move {
                                if let Err(e) = handle_client(stream, &ctx).await {
                                    error!("Client handler error: {}", e);
                                }
                            });
                        }
                        Err(e) => {
                            error!("Accept error: {:?}", e);
                        }
                    }
                }
                _ = shutdown_coordinator.wait() => {
                    info!("Shutdown signal received");
                    break;
                }
            }
        }

        // Shut down signaling connection
        if let Some((handle, shutdown_tx)) = signaling_handle {
            let _ = shutdown_tx.send(true);
            let _ = handle.await;
        }

        if let Some(handle) = dns_handle {
            handle.abort();
        }
        if !dns_tlds.is_empty() {
            if let Err(e) = dns::cleanup_resolver_files(&dns_tlds) {
                warn!("Failed to cleanup DNS resolver files: {}", e);
            }
        }

        drop(pid_file);

        self.shutdown().await?;
        Ok(())
    }

    async fn shutdown(&self) -> Result<()> {
        info!("Shutting down daemon...");

        for source in self.source_manager.list_sources().await {
            if let Err(e) = self.source_manager.stop_source(&source.name).await {
                warn!("Failed to stop source {}: {}", source.name, e);
            }
        }

        info!("Daemon shutdown complete");
        Ok(())
    }
}

struct ActiveStreams {
    streams: HashMap<Uuid, tokio::sync::mpsc::Sender<()>>,
}

impl ActiveStreams {
    fn new() -> Self {
        Self {
            streams: HashMap::new(),
        }
    }

    fn add(&mut self, stream_id: Uuid, cancel_tx: tokio::sync::mpsc::Sender<()>) {
        self.streams.insert(stream_id, cancel_tx);
    }

    fn remove(&mut self, stream_id: &Uuid) -> bool {
        self.streams.remove(stream_id).is_some()
    }

    async fn cancel_all(&mut self) {
        for (_, cancel_tx) in self.streams.drain() {
            let _ = cancel_tx.send(()).await;
        }
    }
}

// --- Wire conversion helpers ---

fn to_wire_log_line(line: &LogLine) -> WireLogLine {
    WireLogLine {
        timestamp: line.timestamp,
        level: line.level.to_string(),
        service_fqn: line.service_fqn.clone(),
        message: line.message.clone(),
        fields: None,
    }
}

fn to_wire_source_info(s: SourceInfo) -> WireSourceInfo {
    WireSourceInfo {
        name: s.name,
        path: s.path,
        source_type: match s.source_type {
            crate::hive_config::SourceType::Yaml => WireSourceType::Yaml,
            crate::hive_config::SourceType::Sqlite => WireSourceType::Sqlite,
        },
        enabled: s.enabled,
        service_count: s.service_count,
        status: match s.status {
            SourceStatus::Loaded => WireSourceStatus::Loaded,
            SourceStatus::Running => WireSourceStatus::Running,
            SourceStatus::Stopped => WireSourceStatus::Stopped,
            SourceStatus::Error(e) => WireSourceStatus::Error(e),
        },
    }
}

fn build_wire_service_status(
    source_name: &str,
    info: &crate::hive_config::ServiceInfo,
) -> WireServiceStatus {
    WireServiceStatus {
        fqn: format!("{}:{}", source_name, info.name),
        source: source_name.to_string(),
        name: info.name.clone(),
        state: info.state.to_string(),
        healthy: info.healthy,
        pid: info.pid,
        container_id: None,
        started_at: None,
        ports: info.ports.clone(),
        restart_count: info.restart_count,
    }
}

// --- Response helpers ---

/// Serialize and send a response over the writer.
async fn send_response(writer: &Writer, response: &DaemonResponse) -> Result<()> {
    let json = serde_json::to_string(response)?;
    let mut w = writer.lock().await;
    w.write_all(json.as_bytes()).await?;
    w.write_all(b"\n").await?;
    Ok(())
}

/// Map a `Result<()>` into a `DaemonResponse` with consistent error formatting.
fn ok_or_error(result: Result<()>, code: &str, ok_msg: String) -> DaemonResponse {
    match result {
        Ok(()) => DaemonResponse::Ok {
            message: Some(ok_msg),
        },
        Err(e) => DaemonResponse::Error {
            code: code.to_string(),
            message: e.to_string(),
        },
    }
}

// --- Client handling ---

async fn handle_client(stream: UnixStream, ctx: &ClientContext) -> Result<()> {
    let (reader, writer) = stream.into_split();
    let mut reader = BufReader::new(reader);
    let writer: Writer = Arc::new(tokio::sync::Mutex::new(writer));
    let mut line = String::new();
    let mut active_streams = ActiveStreams::new();

    loop {
        line.clear();
        let bytes_read = reader.read_line(&mut line).await?;

        if bytes_read == 0 {
            active_streams.cancel_all().await;
            break;
        }

        let request: DaemonRequest = match serde_json::from_str(line.trim()) {
            Ok(req) => req,
            Err(e) => {
                let response = DaemonResponse::Error {
                    code: "INVALID_REQUEST".to_string(),
                    message: format!("Invalid JSON: {}", e),
                };
                send_response(&writer, &response).await?;
                continue;
            }
        };

        match request {
            DaemonRequest::StreamLogs { fqn, level } => {
                let stream_id = Uuid::new_v4();
                let (cancel_tx, cancel_rx) = tokio::sync::mpsc::channel(1);
                active_streams.add(stream_id, cancel_tx);

                send_response(&writer, &DaemonResponse::StreamStarted { stream_id }).await?;

                let writer = writer.clone();
                let event_collector = ctx.event_collector.clone();
                tokio::spawn(stream_logs(
                    stream_id,
                    fqn,
                    level,
                    event_collector,
                    writer,
                    cancel_rx,
                ));
                continue;
            }

            DaemonRequest::SubscribeServices { source } => {
                let stream_id = Uuid::new_v4();
                let (cancel_tx, cancel_rx) = tokio::sync::mpsc::channel(1);
                active_streams.add(stream_id, cancel_tx);

                send_response(&writer, &DaemonResponse::StreamStarted { stream_id }).await?;

                let writer = writer.clone();
                let event_collector = ctx.event_collector.clone();
                let source_manager = ctx.source_manager.clone();
                tokio::spawn(stream_service_status(
                    stream_id,
                    source,
                    source_manager,
                    event_collector,
                    writer,
                    cancel_rx,
                ));
                continue;
            }

            DaemonRequest::StopLogStream { stream_id }
            | DaemonRequest::StopServiceStream { stream_id } => {
                let response = if active_streams.remove(&stream_id) {
                    DaemonResponse::StreamEnded { stream_id }
                } else {
                    DaemonResponse::Error {
                        code: "STREAM_NOT_FOUND".to_string(),
                        message: format!("No active stream with id: {}", stream_id),
                    }
                };
                send_response(&writer, &response).await?;
                continue;
            }

            _ => {}
        }

        let response = process_request(
            request,
            &ctx.source_manager,
            &ctx.exposure_manager,
            &ctx.log_buffer,
            &ctx.shutdown_handle,
            ctx.start_time,
            &ctx.proxy_addresses,
        )
        .await;

        send_response(&writer, &response).await?;
    }

    Ok(())
}

// --- Streaming ---

async fn stream_logs(
    stream_id: Uuid,
    fqn: Option<String>,
    level: Option<String>,
    event_collector: Arc<EventCollector>,
    writer: Writer,
    mut cancel_rx: tokio::sync::mpsc::Receiver<()>,
) {
    let min_level = level.and_then(|l| l.parse::<LogLevel>().ok());

    let subscription = EventSubscription {
        event_types: vec!["log".to_string()],
        services: fqn.map(|f| vec![f]).unwrap_or_default(),
        min_log_level: min_level,
    };

    let mut receiver = event_collector.subscribe(subscription);

    loop {
        tokio::select! {
            result = receiver.recv() => {
                match result {
                    Ok(event) => {
                        if let Some(log_line) = Option::<LogLine>::from(&event) {
                            let response = DaemonResponse::LogStream {
                                stream_id,
                                line: to_wire_log_line(&log_line),
                            };
                            if send_response(&writer, &response).await.is_err() {
                                break;
                            }
                        }
                    }
                    Err(_) => break,
                }
            }
            _ = cancel_rx.recv() => break,
        }
    }

    let _ = send_response(&writer, &DaemonResponse::StreamEnded { stream_id }).await;
}

async fn send_service_snapshot(
    writer: &Writer,
    source_manager: &SourceManager,
    source: Option<&str>,
    stream_id: Uuid,
) -> bool {
    let services: Vec<WireServiceStatus> = source_manager
        .list_services(source)
        .await
        .into_iter()
        .map(|(source_name, info)| build_wire_service_status(&source_name, &info))
        .collect();

    let response = DaemonResponse::ServiceStatusUpdate {
        stream_id,
        services,
    };
    send_response(writer, &response).await.is_ok()
}

async fn stream_service_status(
    stream_id: Uuid,
    source: Option<String>,
    source_manager: Arc<SourceManager>,
    event_collector: Arc<EventCollector>,
    writer: Writer,
    mut cancel_rx: tokio::sync::mpsc::Receiver<()>,
) {
    let service_filter = source
        .as_ref()
        .map(|s| vec![format!("{}:*", s)])
        .unwrap_or_default();

    let subscription = EventSubscription {
        event_types: vec!["service_event".to_string()],
        services: service_filter,
        min_log_level: None,
    };

    let mut receiver = event_collector.subscribe(subscription);

    if !send_service_snapshot(&writer, &source_manager, source.as_deref(), stream_id).await {
        return;
    }

    loop {
        tokio::select! {
            result = receiver.recv() => {
                match result {
                    Ok(_event) => {
                        if !send_service_snapshot(
                            &writer,
                            &source_manager,
                            source.as_deref(),
                            stream_id,
                        ).await {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
            _ = cancel_rx.recv() => break,
        }
    }

    let _ = send_response(&writer, &DaemonResponse::StreamEnded { stream_id }).await;
}

// --- Request processing ---

async fn process_request(
    request: DaemonRequest,
    source_manager: &SourceManager,
    exposure_manager: &ExposureManager,
    log_buffer: &LogBuffer,
    shutdown_handle: &lib_daemon_core::ShutdownHandle,
    start_time: std::time::Instant,
    proxy_addresses: &[String],
) -> DaemonResponse {
    match request {
        DaemonRequest::Ping => DaemonResponse::Pong,

        DaemonRequest::Status => {
            let sources = source_manager.list_sources().await;
            let running_services = sources
                .iter()
                .filter(|s| s.status == SourceStatus::Running)
                .map(|s| s.service_count)
                .sum();
            let total_services: usize = sources.iter().map(|s| s.service_count).sum();

            DaemonResponse::Status(DaemonStatus {
                running: true,
                pid: Some(std::process::id()),
                version: env!("CARGO_PKG_VERSION").to_string(),
                source_count: sources.len(),
                running_services,
                total_services,
                proxy_addresses: proxy_addresses.to_vec(),
                uptime_secs: start_time.elapsed().as_secs(),
            })
        }

        DaemonRequest::Shutdown { graceful } => {
            if graceful {
                info!("Graceful shutdown requested - stopping all sources first");
                let sources = source_manager.list_sources().await;
                for source in sources {
                    if source.status == SourceStatus::Running {
                        if let Err(e) = source_manager.stop_source(&source.name).await {
                            warn!(
                                "Failed to stop source '{}' during graceful shutdown: {}",
                                source.name, e
                            );
                        }
                    }
                }
                info!("All sources stopped, initiating daemon shutdown");
            } else {
                info!("Immediate shutdown requested");
            }
            shutdown_handle.shutdown();
            DaemonResponse::Ok {
                message: Some(if graceful {
                    "Graceful shutdown completed".to_string()
                } else {
                    "Shutdown initiated".to_string()
                }),
            }
        }

        DaemonRequest::ListSources => {
            let sources = source_manager.list_sources().await;
            DaemonResponse::Sources {
                sources: sources.into_iter().map(to_wire_source_info).collect(),
            }
        }

        DaemonRequest::AddSource { path, name } => {
            let path = PathBuf::from(&path);
            match source_manager.add_source(&path, name.as_deref()).await {
                Ok(name) => DaemonResponse::SourceAdded { name },
                Err(e) => DaemonResponse::Error {
                    code: "ADD_SOURCE_FAILED".to_string(),
                    message: e.to_string(),
                },
            }
        }

        DaemonRequest::RemoveSource { name } => ok_or_error(
            source_manager.remove_source(&name).await,
            "REMOVE_SOURCE_FAILED",
            format!("Removed source: {}", name),
        ),

        DaemonRequest::ReloadSource { name } => ok_or_error(
            source_manager.reload_source(&name).await,
            "RELOAD_SOURCE_FAILED",
            format!("Reloaded source: {}", name),
        ),

        DaemonRequest::EnableSource { name } => ok_or_error(
            source_manager.enable_source(&name).await,
            "ENABLE_SOURCE_FAILED",
            format!("Enabled source: {}", name),
        ),

        DaemonRequest::DisableSource { name } => ok_or_error(
            source_manager.disable_source(&name).await,
            "DISABLE_SOURCE_FAILED",
            format!("Disabled source: {}", name),
        ),

        DaemonRequest::StartSource { name } => ok_or_error(
            source_manager.start_source(&name).await,
            "START_SOURCE_FAILED",
            format!("Started source: {}", name),
        ),

        DaemonRequest::StopSource { name } => ok_or_error(
            source_manager.stop_source(&name).await,
            "STOP_SOURCE_FAILED",
            format!("Stopped source: {}", name),
        ),

        DaemonRequest::StartService { fqn } => ok_or_error(
            source_manager.start_service(&fqn).await,
            "START_SERVICE_FAILED",
            format!("Started service: {}", fqn),
        ),

        DaemonRequest::StopService { fqn } => ok_or_error(
            source_manager.stop_service(&fqn).await,
            "STOP_SERVICE_FAILED",
            format!("Stopped service: {}", fqn),
        ),

        DaemonRequest::RestartService { fqn } => ok_or_error(
            source_manager.restart_service(&fqn).await,
            "RESTART_SERVICE_FAILED",
            format!("Restarted service: {}", fqn),
        ),

        DaemonRequest::GetServiceStatus { fqn } => match source_manager.get_service(&fqn).await {
            Ok(Some((source_name, info))) => DaemonResponse::Services {
                services: vec![build_wire_service_status(&source_name, &info)],
            },
            Ok(None) => DaemonResponse::Error {
                code: "NOT_FOUND".to_string(),
                message: format!("Service '{}' not found", fqn),
            },
            Err(e) => DaemonResponse::Error {
                code: "INVALID_FQN".to_string(),
                message: e.to_string(),
            },
        },

        DaemonRequest::ListServices { source } => {
            let services: Vec<WireServiceStatus> = source_manager
                .list_services(source.as_deref())
                .await
                .into_iter()
                .map(|(source_name, info)| build_wire_service_status(&source_name, &info))
                .collect();

            DaemonResponse::Services { services }
        }

        DaemonRequest::CreateService {
            source_id,
            name,
            config,
        } => {
            match serde_json::from_value::<crate::hive_config::ServiceConfig>(config) {
                Ok(service_config) => ok_or_error(
                    source_manager.create_service(&source_id, &name, service_config).await,
                    "CREATE_SERVICE_FAILED",
                    format!("Created service {}:{}", source_id, name),
                ),
                Err(e) => DaemonResponse::Error {
                    code: "INVALID_CONFIG".to_string(),
                    message: format!("Invalid service config: {}", e),
                },
            }
        }

        DaemonRequest::UpdateService { fqn, patch: _ } => DaemonResponse::Error {
            code: "NOT_IMPLEMENTED".to_string(),
            message: format!("UpdateService not yet implemented for '{}'", fqn),
        },

        DaemonRequest::DeleteService { fqn } => ok_or_error(
            source_manager.delete_service(&fqn).await,
            "DELETE_SERVICE_FAILED",
            format!("Deleted service: {}", fqn),
        ),

        DaemonRequest::ListExposed => {
            let exposed = exposure_manager.list_exposed().await;
            let info: Vec<WireExposedServiceInfo> = exposed
                .into_iter()
                .map(|e| WireExposedServiceInfo {
                    name: e.name,
                    source: e.source_name,
                    service: e.service_name,
                    healthy: e.healthy,
                    var_names: e.vars.keys().cloned().collect(),
                    port_names: e.ports.keys().cloned().collect(),
                })
                .collect();
            DaemonResponse::Exposed { exposed: info }
        }

        DaemonRequest::GetLogs {
            fqn,
            lines,
            since,
            level,
        } => {
            let limit = lines.map(|l| l as usize).unwrap_or(daemon_defaults::LOG_LINES_LIMIT);
            let min_level = level.and_then(|l| l.parse::<LogLevel>().ok());

            let mut logs = if let Some(service_fqn) = fqn {
                let mut result = log_buffer.get(&service_fqn, Some(limit));

                if result.is_empty() && !service_fqn.contains(':') {
                    let all_logs = log_buffer.get_all(None, None);
                    let suffix = format!(":{}", service_fqn);
                    result = all_logs
                        .into_iter()
                        .filter(|l| {
                            l.service_fqn.ends_with(&suffix) || l.service_fqn == service_fqn
                        })
                        .collect();
                    let len = result.len();
                    if len > limit {
                        result = result.into_iter().skip(len - limit).collect();
                    }
                }
                result
            } else {
                log_buffer.get_all(None, Some(limit))
            };

            if let Some(since_time) = since {
                logs.retain(|l| l.timestamp >= since_time);
            }

            if let Some(min) = min_level {
                logs.retain(|l| l.level >= min);
            }

            DaemonResponse::Logs {
                logs: logs.iter().map(to_wire_log_line).collect(),
            }
        }

        DaemonRequest::StreamLogs { .. }
        | DaemonRequest::StopLogStream { .. }
        | DaemonRequest::SubscribeServices { .. }
        | DaemonRequest::StopServiceStream { .. } => DaemonResponse::Error {
            code: "INTERNAL_ERROR".to_string(),
            message: "Streaming requests should be handled separately".to_string(),
        },
    }
}

async fn populate_log_buffer(event_collector: Arc<EventCollector>, log_buffer: Arc<LogBuffer>) {
    let subscription = EventSubscription::logs();
    let mut receiver = event_collector.subscribe(subscription);

    debug!("Log buffer population task started");

    loop {
        match receiver.recv().await {
            Ok(event) => {
                if let Some(log_line) = Option::<LogLine>::from(&event) {
                    log_buffer.add(log_line);
                }
            }
            Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                debug!("Event collector closed, stopping log buffer population");
                break;
            }
            Err(tokio::sync::broadcast::error::RecvError::Lagged(count)) => {
                warn!("Log buffer population lagged by {} events", count);
            }
        }
    }
}

fn extract_dns_port(bind: &str) -> Result<u16> {
    bind.rsplit(':')
        .next()
        .and_then(|p| p.parse().ok())
        .ok_or_else(|| anyhow!("DNS bind address must contain a valid port: {}", bind))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_dns_port() {
        assert_eq!(extract_dns_port("127.0.0.1:15353").unwrap(), 15353);
        assert_eq!(extract_dns_port("0.0.0.0:5353").unwrap(), 5353);
    }

    #[test]
    fn test_extract_dns_port_invalid() {
        assert!(extract_dns_port("invalid").is_err());
    }

    #[test]
    fn test_daemon_config_paths() {
        let config = DaemonConfig::new("/tmp/test-hive");
        assert!(config.socket_path().ends_with("adi-hive.sock"));
        assert!(config.pid_path().ends_with("adi-hive.pid"));
        assert_eq!(config.base_dir(), std::path::Path::new("/tmp/test-hive"));
    }

    #[test]
    fn test_request_serialization() {
        let request = DaemonRequest::Status;
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("status"));

        let request = DaemonRequest::AddSource {
            path: "/foo/bar".to_string(),
            name: Some("test".to_string()),
        };
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("add_source"));
        assert!(json.contains("/foo/bar"));
    }

    #[test]
    fn test_response_serialization() {
        let response = DaemonResponse::Ok {
            message: Some("test".to_string()),
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("ok"));

        let response = DaemonResponse::Status(DaemonStatus {
            running: true,
            pid: Some(1234),
            version: "0.1.0".to_string(),
            source_count: 2,
            running_services: 5,
            total_services: 10,
            proxy_addresses: vec!["127.0.0.1:8080".to_string()],
            uptime_secs: 3600,
        });
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("status"));
        assert!(json.contains("1234"));
    }
}
