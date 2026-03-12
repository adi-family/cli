//! Hive YAML Configuration Types
//!
//! Data structures for representing hive.yaml configuration according to the spec.

use lib_plugin_abi_v3::hooks::HooksConfig;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Built-in rollout strategy: recreate (stop-then-start)
pub const ROLLOUT_TYPE_RECREATE: &str = "recreate";

/// Built-in rollout strategy: blue-green deployment
pub const ROLLOUT_TYPE_BLUE_GREEN: &str = "blue-green";

/// Root configuration structure for hive.yaml
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HiveConfig {
    /// Must be "1"
    pub version: String,

    /// Plugin registry URL for this project.
    /// Overrides the global default when set; $ADI_REGISTRY_URL still takes
    /// precedence over this value.
    #[serde(default)]
    pub registry_url: Option<String>,

    #[serde(default)]
    pub defaults: HashMap<String, serde_json::Value>,

    #[serde(default)]
    pub proxy: Option<ProxyConfig>,

    #[serde(default)]
    pub environment: Option<EnvironmentConfig>,

    #[serde(default)]
    pub observability: Option<ObservabilityConfig>,

    /// Run before/after all services
    #[serde(default)]
    pub hooks: Option<HooksConfig>,

    pub services: HashMap<String, ServiceConfig>,
}

impl Default for HiveConfig {
    fn default() -> Self {
        Self {
            version: "1".to_string(),
            registry_url: None,
            defaults: HashMap::new(),
            proxy: None,
            environment: None,
            observability: None,
            hooks: None,
            services: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsProxyConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Upstream server for forwarding unknown queries
    #[serde(default)]
    pub upstream: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProxyConfig {
    #[serde(default)]
    pub bind: ProxyBind,

    #[serde(default)]
    pub ssl: Option<SslConfig>,

    #[serde(default)]
    pub dns: Option<DnsProxyConfig>,

    #[serde(default)]
    pub plugins: Vec<ProxyPluginConfig>,

    /// Show recent service logs on 502 error pages
    #[serde(default = "default_true")]
    pub show_error_logs: bool,

    /// Inject debug timing headers into every proxied response
    #[serde(default)]
    pub debug: bool,
}

/// Single string or array of addresses
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ProxyBind {
    Single(String),
    Multiple(Vec<String>),
}

impl Default for ProxyBind {
    fn default() -> Self {
        ProxyBind::Single("127.0.0.1:8080".to_string())
    }
}

impl ProxyBind {
    pub fn addresses(&self) -> Vec<&str> {
        match self {
            ProxyBind::Single(s) => vec![s.as_str()],
            ProxyBind::Multiple(v) => v.iter().map(|s| s.as_str()).collect(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SslConfig {
    #[serde(rename = "type")]
    pub ssl_type: String,

    /// Plugin-specific configuration (flattened)
    #[serde(flatten)]
    pub config: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyPluginConfig {
    #[serde(rename = "type")]
    pub plugin_type: String,

    /// Plugin-specific configuration (flattened)
    #[serde(flatten)]
    pub config: HashMap<String, serde_json::Value>,
}

/// Multiple providers can be combined
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EnvironmentConfig {
    /// Built-in static key-value pairs
    #[serde(default, rename = "static")]
    pub static_env: Option<HashMap<String, String>>,

    /// Other providers (e.g., dotenv, vault)
    #[serde(flatten)]
    pub providers: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ObservabilityConfig {
    #[serde(default)]
    pub resource_interval: Option<String>,

    #[serde(default)]
    pub buffer_size: Option<u64>,

    #[serde(default)]
    pub collectors: Option<CollectorsConfig>,

    #[serde(default)]
    pub plugins: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CollectorsConfig {
    #[serde(default = "default_true")]
    pub logs: bool,
    #[serde(default = "default_true")]
    pub metrics: bool,
    #[serde(default = "default_true")]
    pub traces: bool,
    #[serde(default = "default_true")]
    pub health: bool,
    #[serde(default = "default_true")]
    pub resources: bool,
    #[serde(default = "default_true")]
    pub proxy: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceConfig {
    pub runner: RunnerConfig,

    /// Required if proxy or healthcheck is set
    #[serde(default)]
    pub rollout: Option<RolloutConfig>,

    #[serde(default)]
    pub proxy: Option<ServiceProxyConfig>,

    #[serde(default)]
    pub depends_on: Vec<String>,

    #[serde(default)]
    pub healthcheck: Option<HealthCheckConfig>,

    #[serde(default)]
    pub environment: Option<EnvironmentConfig>,

    #[serde(default)]
    pub build: Option<BuildConfig>,

    #[serde(default)]
    pub restart: RestartPolicy,

    #[serde(default)]
    pub expose: Option<ExposeConfig>,

    /// Dependencies on exposed services
    #[serde(default)]
    pub uses: Vec<UsesConfig>,

    /// Lifecycle hooks (pre-up, post-up, pre-down, post-down)
    #[serde(default)]
    pub hooks: Option<HooksConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunnerConfig {
    /// e.g., "script", "docker"
    #[serde(rename = "type")]
    pub runner_type: String,

    /// Plugin-specific configuration (flattened)
    #[serde(flatten)]
    pub config: HashMap<String, serde_json::Value>,
}

/// Built-in script runner
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptRunnerConfig {
    pub run: String,

    #[serde(default)]
    pub working_dir: Option<String>,

    /// e.g., "zsh", "bash", "sh"
    #[serde(default)]
    pub shell: Option<String>,
}

/// External docker runner plugin
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockerRunnerConfig {
    pub image: String,

    /// Auto-generated if not provided
    #[serde(default)]
    pub container_name: Option<String>,

    #[serde(default)]
    pub ports: Vec<String>,

    #[serde(default)]
    pub volumes: Vec<String>,

    #[serde(default)]
    pub environment: HashMap<String, String>,

    #[serde(default)]
    pub socket: Option<String>,

    /// Skip automatic image pull before container creation
    #[serde(default)]
    pub deny_pull: bool,

    /// Defaults to secure settings
    #[serde(default)]
    pub security: DockerSecurityConfig,

    /// Default: "127.0.0.1" (local-only access)
    #[serde(default = "default_bind_ip")]
    pub bind_ip: String,
}

fn default_bind_ip() -> String {
    "127.0.0.1".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockerSecurityConfig {
    /// Drop all Linux capabilities (recommended)
    #[serde(default = "default_true")]
    pub cap_drop_all: bool,
    /// e.g., ["NET_BIND_SERVICE"]
    #[serde(default)]
    pub cap_add: Vec<String>,
    #[serde(default = "default_true")]
    pub no_new_privileges: bool,
    #[serde(default)]
    pub read_only: bool,
    /// e.g., "1000:1000" or "nobody"
    #[serde(default)]
    pub user: Option<String>,
    /// e.g., ["seccomp=unconfined"]
    #[serde(default)]
    pub security_opt: Vec<String>,
}

impl Default for DockerSecurityConfig {
    fn default() -> Self {
        Self {
            cap_drop_all: true,
            cap_add: Vec::new(),
            no_new_privileges: true,
            read_only: false,
            user: None,
            security_opt: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RolloutConfig {
    #[serde(rename = "type")]
    pub rollout_type: String,

    /// Plugin-specific configuration (flattened)
    #[serde(flatten)]
    pub config: HashMap<String, serde_json::Value>,
}

/// Built-in recreate rollout
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecreateRolloutConfig {
    pub ports: HashMap<String, PortValue>,
}

/// External blue-green rollout plugin
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlueGreenRolloutConfig {
    pub ports: HashMap<String, BlueGreenPort>,

    /// Duration new instance must be healthy
    #[serde(default)]
    pub healthy_duration: Option<String>,

    /// Max time to wait for new instance
    #[serde(default)]
    pub timeout: Option<String>,

    /// Action if new instance fails
    #[serde(default)]
    pub on_failure: Option<String>,
}

/// Simple port number or blue-green pair
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PortValue {
    Simple(u16),
    BlueGreen(BlueGreenPort),
}

impl PortValue {
    pub fn get_port(&self) -> u16 {
        match self {
            PortValue::Simple(p) => *p,
            PortValue::BlueGreen(bg) => bg.blue, // Default to blue
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlueGreenPort {
    pub blue: u16,
    pub green: u16,
}

/// Single endpoint or multiple
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ServiceProxyConfig {
    Single(ProxyEndpoint),
    Multiple(Vec<ProxyEndpoint>),
}

impl ServiceProxyConfig {
    pub fn endpoints(&self) -> Vec<&ProxyEndpoint> {
        match self {
            ServiceProxyConfig::Single(e) => vec![e],
            ServiceProxyConfig::Multiple(v) => v.iter().collect(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyEndpoint {
    #[serde(default)]
    pub host: Option<String>,

    /// HTTP path prefix
    pub path: String,

    #[serde(default)]
    pub port: Option<String>,

    #[serde(default)]
    pub strip_prefix: bool,

    #[serde(default)]
    pub timeout: Option<String>,

    #[serde(default)]
    pub buffer_size: Option<String>,

    #[serde(default)]
    pub headers: Option<ProxyHeaders>,

    /// Per-endpoint plugin overrides
    #[serde(default)]
    pub plugins: Vec<ProxyPluginConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProxyHeaders {
    #[serde(default)]
    pub add: HashMap<String, String>,

    #[serde(default)]
    pub remove: Vec<String>,

    /// Overwrites existing headers
    #[serde(default)]
    pub set: HashMap<String, String>,
}

/// Single check or multiple
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum HealthCheckConfig {
    Single(HealthCheck),
    Multiple(Vec<HealthCheck>),
}

impl HealthCheckConfig {
    pub fn checks(&self) -> Vec<&HealthCheck> {
        match self {
            HealthCheckConfig::Single(c) => vec![c],
            HealthCheckConfig::Multiple(v) => v.iter().collect(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheck {
    /// e.g., "http", "tcp", "cmd"
    #[serde(rename = "type")]
    pub check_type: String,

    /// Plugin-specific configuration (flattened)
    #[serde(flatten)]
    pub config: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpHealthCheckConfig {
    pub port: String,

    #[serde(default = "default_health_path")]
    pub path: String,

    #[serde(default = "default_http_method")]
    pub method: String,

    #[serde(default)]
    pub status: Option<u16>,

    #[serde(default)]
    pub interval: Option<String>,

    #[serde(default)]
    pub timeout: Option<String>,

    /// Retries before marking unhealthy
    #[serde(default)]
    pub retries: Option<u32>,

    /// Grace period before first check
    #[serde(default)]
    pub start_period: Option<String>,
}

fn default_health_path() -> String {
    "/health".to_string()
}

fn default_http_method() -> String {
    "GET".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TcpHealthCheckConfig {
    pub port: String,

    #[serde(default)]
    pub interval: Option<String>,

    #[serde(default)]
    pub timeout: Option<String>,

    /// Retries before marking unhealthy
    #[serde(default)]
    pub retries: Option<u32>,

    /// Grace period before first check
    #[serde(default)]
    pub start_period: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CmdHealthCheckConfig {
    pub command: String,

    #[serde(default)]
    pub working_dir: Option<String>,

    #[serde(default)]
    pub interval: Option<String>,

    #[serde(default)]
    pub timeout: Option<String>,

    /// Retries before marking unhealthy
    #[serde(default)]
    pub retries: Option<u32>,

    /// Grace period before first check
    #[serde(default)]
    pub start_period: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildConfig {
    pub command: String,

    #[serde(default)]
    pub working_dir: Option<String>,

    #[serde(default, rename = "when")]
    pub build_when: BuildTrigger,

    /// File/directory to check for `when: missing`; if unset, always builds
    #[serde(default)]
    pub output: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum BuildTrigger {
    /// Build only if output doesn't exist
    #[default]
    Missing,
    /// Build every time before starting
    Always,
    Never,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum RestartPolicy {
    #[default]
    Never,
    /// Restart only on non-zero exit code
    OnFailure,
    Always,
    /// Like Always, but respects manual stop
    UnlessStopped,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExposeConfig {
    /// Globally unique name
    pub name: String,

    /// Required to consume this service
    #[serde(default)]
    pub secret: Option<String>,

    pub vars: HashMap<String, String>,
}

/// Cross-source dependency on an exposed service
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsesConfig {
    pub name: String,

    #[serde(default)]
    pub secret: Option<String>,

    #[serde(default, rename = "as")]
    pub alias: Option<String>,

    #[serde(default)]
    pub vars: HashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ServiceState {
    Stopped,
    Starting,
    Running,
    Unhealthy,
    Stopping,
    Crashed,
    Exited,
    /// Port occupied by an unmanaged process
    PortConflict,
}

impl std::fmt::Display for ServiceState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ServiceState::Stopped => write!(f, "stopped"),
            ServiceState::Starting => write!(f, "starting"),
            ServiceState::Running => write!(f, "running"),
            ServiceState::Unhealthy => write!(f, "unhealthy"),
            ServiceState::Stopping => write!(f, "stopping"),
            ServiceState::Crashed => write!(f, "crashed"),
            ServiceState::Exited => write!(f, "exited"),
            ServiceState::PortConflict => write!(f, "port conflict"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ServiceInfo {
    pub name: String,
    pub state: ServiceState,
    /// Set when process is running
    pub pid: Option<u32>,
    /// Set for docker-based services
    pub container_id: Option<String>,
    /// Resolved port assignments
    pub ports: HashMap<String, u16>,
    pub healthy: Option<bool>,
    pub last_error: Option<String>,
    pub restart_count: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SourceType {
    Yaml,
    Sqlite,
}

#[derive(Debug, Clone)]
pub struct ConfigSource {
    /// e.g., "default", "project-name"
    pub name: String,
    pub path: PathBuf,
    pub source_type: SourceType,
    pub enabled: bool,
}
