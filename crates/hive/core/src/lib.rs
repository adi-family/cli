//! Hive Core Library
//!
//! Provides core functionality for local service orchestration:
//! - Hive YAML configuration parsing and management
//! - Service lifecycle management
//! - Service proxy based on hive.yaml routing
//! - Service exposure for cross-source dependencies
//! - Multi-source configuration management
//! - Daemon mode with Unix socket control
//! - Observability event streaming
//! - Plugin system with auto-install
//! - SQLite configuration backend
//! - Remote control via signaling server

pub mod core_plugins;
pub mod crypto;
pub mod daemon;
pub mod daemon_defaults;
pub mod dns;
pub mod defaults;
pub mod error_pages;
pub mod exposure;
pub mod global_registry;
pub mod hive_config;
pub mod hive_signaling;
pub mod observability;
pub mod observability_plugins;
pub mod plugin_system;
pub mod plugins;
pub mod proxy_plugins;
pub mod runtime_db;
pub mod service_manager;
pub mod service_proxy;
pub mod signaling_control;
pub mod source_manager;
pub mod sqlite_backend;

pub use core_plugins::{CorePlugin, CorePluginRegistry, DaemonEvent};
pub use crypto::hmac_sign;
pub use daemon::{
    DaemonClient, DaemonConfig, DaemonRequest, DaemonResponse, DaemonStatus, HiveDaemon,
    WireServiceStatus, WireExposedServiceInfo, WireLogLine, LogStreamHandle,
    WireSourceInfo, WireSourceType, WireSourceStatus,
};
pub use dns::{DnsConfig, DnsServer};
pub use defaults::{apply_all_defaults, apply_service_defaults, merge_json, DefaultsManager};
pub use exposure::{ExposedService, ExposureManager};
pub use hive_config::{
    find_project_root, get_rollout_ports, topological_sort, topological_sort_levels,
    validate_config, ConfigSource, ExposeConfig, HiveConfig, HiveConfigParser, ParseContext,
    ParsePlugin, RuntimeContext, ServiceConfig, ServiceInfo, ServiceState, SourceType, UsesConfig,
};
pub use observability::{
    EventCollector, EventSubscription, HealthStatus, LogBuffer, LogLevel, LogLine, LogStream,
    MetricValue, ObservabilityEvent, ServiceEventType, SpanStatus,
};
pub use observability_plugins::{
    FileObsPlugin, ObsPlugin, ObsPluginManager, OutputFormat, StdoutObsPlugin,
};
pub use plugin_system::{
    is_core_plugin, plugin_registry, resolve_plugin_id, PluginInfo, PluginRegistry, PluginStatus,
    PluginType,
};
pub use proxy_plugins::{
    CorsMiddleware, HeadersMiddleware, IpFilterMiddleware, MiddlewareChain, ProxyMiddleware,
    ProxyMiddlewareResult, RateLimitBy, RateLimitMiddleware,
};
pub use service_manager::{
    parse_duration, BlueGreenColor, BlueGreenDeployment, BlueGreenState, DotenvPlugin,
    EnvironmentResolver, HealthChecker, OnFailureAction,
    PortsParsePlugin, ProcessManager, ProcessType, RolloutManager, ServiceManager, ServicePhase,
};
pub use service_proxy::{
    create_service_proxy_router, start_service_proxy_server, Route, ServiceProxyState,
};
pub use signaling_control::{
    parse_fqn as parse_service_fqn, CocoonKindInfo, DaemonStatus as RemoteDaemonStatus,
    ErrorCode, ExposedInfo, HiveControlMessage, HiveDeviceType, HiveRegistration, HiveRequest,
    HiveResponse, RemoteControlHandler, RequestHandler, ServiceInfo as RemoteServiceInfo,
    ServiceState as RemoteServiceState, SimpleRequestHandler, SourceConfig,
};
pub use hive_signaling::HiveSignalingConfig;
pub use global_registry::{GlobalRegistry, RegisteredSource};
pub use runtime_db::RuntimeDb;
pub use source_manager::{read_sources_registry, SourceInfo, SourceManager, SourceStatus};
pub use sqlite_backend::{RuntimeState, ServicePatch, SqliteBackend};

// Plugin system
pub use plugins::{
    init_global_plugins, init_plugins, plugin_manager, PluginLoadStatus, PluginManager,
};
