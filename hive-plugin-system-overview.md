# Hive Plugin System: Comprehensive Overview

**Component:** Hive Cocoon Container Orchestration
**Scope:** crates/hive (core, plugins)
**Date:** 2026-01-31

---

## Executive Summary

Hive uses a **dual plugin system architecture** with a legacy registry and modern trait-based manager. The system supports **32 plugins across 6 categories**, with 8 bundled by default via Cargo feature flags. All plugins implement standardized traits from `lib-plugin-abi-orchestration` for cross-orchestrator compatibility.

**Key Stats:**
- 2 plugin systems (legacy + modern)
- 6 plugin categories (Runner, Env, Health, Proxy, Obs, Rollout)
- 32 total plugins (8 bundled, 24 external)
- 100% async Rust implementation
- Global singleton architecture

---

## 1. Plugin System Architecture

### 1.1 Dual System: plugin_system.rs vs plugins.rs

#### **Legacy System: plugin_system.rs**

**Purpose:** Metadata registry and auto-installation system

**Key Components:**
```rust
pub struct PluginRegistry {
    plugins: HashMap<String, PluginInfo>,
}

pub struct PluginInfo {
    pub id: String,              // e.g., "hive.runner.docker"
    pub plugin_type: PluginType, // Runner, Health, Proxy, etc.
    pub name: String,            // Human-readable name
    pub status: PluginStatus,    // BuiltIn or External
}

pub enum PluginType {
    Parse,      // Config parsers
    Runner,     // Service executors
    Env,        // Environment providers
    Health,     // Health checkers
    Rollout,    // Deployment strategies
    ProxySsl,   // SSL termination
    ProxyAuth,  // Authentication middleware
    Proxy,      // HTTP middleware
    Obs,        // Observability sinks
}
```

**Capabilities:**
- Plugin metadata storage (id, type, name, status)
- Built-in plugin registry (17 hardcoded definitions)
- Auto-installation via `adi plugin install <plugin-id>`
- Plugin discovery: `is_available()`, `is_builtin()`, `list_by_type()`
- Global singleton: `plugin_registry()`

**Status:** Being phased out - no actual trait implementations, metadata only

**Location:** `crates/hive/core/src/plugin_system.rs`

---

#### **Modern System: plugins.rs**

**Purpose:** Type-safe plugin instance management with trait implementations

**Key Components:**
```rust
pub struct PluginManager {
    runners: HashMap<String, Arc<dyn RunnerPlugin>>,
    envs: HashMap<String, Arc<dyn EnvPlugin>>,
    healths: HashMap<String, Arc<dyn HealthPlugin>>,
    proxies: HashMap<String, Arc<dyn ProxyPlugin>>,
    obs: HashMap<String, Arc<dyn ObsPlugin>>,
    rollouts: HashMap<String, Arc<dyn RolloutPlugin>>,
}

pub struct PluginMeta {
    pub id: String,
    pub name: String,
    pub version: String,
    pub category: String,
    pub load_status: LoadStatus,  // BuiltIn or External
}

pub enum LoadStatus {
    BuiltIn,
    External { path: PathBuf },
}
```

**Capabilities:**
- Type-safe plugin storage by category
- Actual trait implementations (not just metadata)
- Bundled plugin initialization at startup
- Plugin retrieval: `get_runner()`, `get_env()`, `get_health()`
- Metadata tracking: `list_plugins()`, `list_by_category()`
- Global singleton: `plugin_manager()`

**Status:** Modern, recommended approach - actively used by ServiceManager

**Location:** `crates/hive/core/src/plugins.rs`

---

### 1.2 Key Architectural Differences

| Aspect | plugin_system.rs | plugins.rs |
|--------|------------------|------------|
| **Purpose** | Metadata registry | Instance management |
| **Storage** | `PluginInfo` structs | `Arc<dyn Trait>` implementations |
| **Functionality** | Discovery, auto-install | Execution, lifecycle |
| **Type Safety** | String-based lookup | Trait-based, compile-time |
| **Initialization** | Static definitions | Runtime registration |
| **Usage** | Auto-install checks | Service execution |
| **Future** | Being deprecated | Active development |

---

## 2. Plugin Categories & Traits

All plugin traits are defined in `lib-plugin-abi-orchestration` for cross-orchestrator standardization.

### 2.1 Runner Plugin

**Purpose:** Execute services (containers, processes, scripts)

**Trait Definition:**
```rust
#[async_trait]
pub trait RunnerPlugin: Send + Sync {
    fn metadata(&self) -> PluginMetadata;

    async fn init(&mut self, defaults: &serde_json::Value) -> Result<()>;

    async fn start(
        &self,
        service_name: &str,
        config: &serde_json::Value,
        env: HashMap<String, String>,
        ctx: &RuntimeContext,
    ) -> Result<ProcessHandle>;

    async fn stop(&self, handle: &ProcessHandle) -> Result<()>;

    async fn is_running(&self, handle: &ProcessHandle) -> bool;

    async fn logs(&self, handle: &ProcessHandle, lines: Option<usize>) -> Result<Vec<String>>;

    fn supports_hooks(&self) -> bool { false }

    async fn run_hook(
        &self,
        config: &serde_json::Value,
        env: HashMap<String, String>,
        ctx: &RuntimeContext,
    ) -> Result<HookExitStatus>;
}
```

**Available Runners:**
- **hive.runner.docker** (bundled) - Docker container execution
- **hive.runner.compose** (external) - Docker Compose orchestration
- **hive.runner.podman** (external) - Podman container execution

**ProcessHandle:**
```rust
pub struct ProcessHandle {
    pub id: String,           // Container ID, PID, etc.
    pub runner_type: String,  // "docker", "podman", etc.
    pub metadata: HashMap<String, String>,
}
```

**Hook Support:**
- Docker and compose runners support lifecycle hooks via `docker run --rm`
- Script runner supports hooks via shell execution
- Hooks enable pre-up, post-up, pre-down, post-down actions

**YAML Example:**
```yaml
services:
  api:
    runner:
      type: docker
      docker:
        image: myapp:latest
        ports:
          - "{{runtime.port.main}}:8080"
        volumes:
          - ./data:/data
```

---

### 2.2 Health Plugin

**Purpose:** Check service readiness and health

**Trait Definition:**
```rust
#[async_trait]
pub trait HealthPlugin: Send + Sync {
    fn metadata(&self) -> PluginMetadata;

    async fn init(&mut self, defaults: &serde_json::Value) -> Result<()>;

    async fn check(
        &self,
        config: &serde_json::Value,
        ctx: &RuntimeContext,
    ) -> Result<HealthResult>;

    async fn shutdown(&self) -> Result<()>;
}

pub struct HealthResult {
    pub healthy: bool,
    pub message: Option<String>,
    pub response_time_ms: u64,
    pub details: HashMap<String, String>,
}
```

**Available Health Checkers:**
- **hive.health.http** (bundled) - HTTP endpoint checks (GET, POST, status codes)
- **hive.health.tcp** (bundled) - TCP port connectivity checks
- **hive.health.cmd** (external) - Command execution (exit code 0 = healthy)
- **hive.health.grpc** (external) - gRPC health check protocol
- **hive.health.mysql** (external) - MySQL connection + query checks
- **hive.health.postgres** (external) - PostgreSQL connection + query checks
- **hive.health.redis** (external) - Redis PING command checks

**YAML Example:**
```yaml
services:
  api:
    health:
      - type: http
        http:
          port: "{{runtime.port.main}}"
          path: /health
          timeout: 5s
          expected_status: 200

      - type: tcp
        tcp:
          port: "{{runtime.port.main}}"
          timeout: 3s
```

**Usage Flow:**
1. ServiceManager starts service via runner
2. HealthChecker polls health plugins
3. All checks must pass before service is "healthy"
4. Healthy services receive traffic (proxy routes enabled)
5. Unhealthy services trigger rollback or alerts

---

### 2.3 Environment Plugin

**Purpose:** Load environment variables from various sources

**Trait Definition:**
```rust
#[async_trait]
pub trait EnvPlugin: Send + Sync {
    fn metadata(&self) -> PluginMetadata;

    async fn load(
        &self,
        config: &serde_json::Value,
    ) -> Result<HashMap<String, String>>;

    async fn refresh(
        &self,
        config: &serde_json::Value,
    ) -> Result<HashMap<String, String>>;
}
```

**Available Env Providers:**
- **hive.env.dotenv** (bundled) - Load from .env files
- **hive.env.vault** (external) - HashiCorp Vault secrets
- **hive.env.1password** (external) - 1Password secret references
- **hive.env.aws-secrets** (external) - AWS Secrets Manager

**Features:**
- Multiple env plugins can be chained (merged in order)
- `refresh()` supports hot-reload for dynamic secrets
- Variables are interpolated in service configs (e.g., `{{env.DATABASE_URL}}`)

**YAML Example:**
```yaml
services:
  api:
    env:
      - type: dotenv
        dotenv:
          files: [.env, .env.local]

      - type: vault
        vault:
          address: https://vault.example.com
          path: secret/data/myapp
          token: "{{env.VAULT_TOKEN}}"
```

---

### 2.4 Proxy Plugin

**Purpose:** HTTP middleware (CORS, auth, rate limiting, etc.)

**Trait Definition:**
```rust
#[async_trait]
pub trait ProxyPlugin: Send + Sync {
    fn metadata(&self) -> PluginMetadata;

    async fn init(&mut self, config: &serde_json::Value) -> Result<()>;

    async fn process_request(
        &self,
        req: ProxyRequest,
    ) -> Result<ProxyResult>;

    async fn process_response(
        &self,
        resp: ProxyResponse,
    ) -> Result<ProxyResponse>;

    async fn shutdown(&self) -> Result<()>;
}

pub enum ProxyResult {
    Continue(ProxyRequest),      // Pass to next middleware
    Response(Response<Body>),    // Short-circuit with response
}
```

**Available Proxy Middleware:**
- **hive.proxy.cors** (bundled) - CORS headers, preflight handling
- **hive.proxy.rate-limit** (bundled) - Rate limiting (IP/header/path-based)
- **hive.proxy.headers** (external) - Add/remove/set HTTP headers
- **hive.proxy.ip-filter** (external) - Allow/deny IP ranges
- **hive.proxy.auth-api-key** (external) - API key authentication
- **hive.proxy.auth-basic** (external) - HTTP Basic auth
- **hive.proxy.auth-jwt** (external) - JWT token validation
- **hive.proxy.auth-oidc** (external) - OpenID Connect authentication
- **hive.proxy.cache** (external) - HTTP caching (memory/redis)
- **hive.proxy.compress** (external) - Response compression (gzip, brotli)
- **hive.proxy.rewrite** (external) - URL rewriting

**Middleware Chain:**
```rust
pub struct MiddlewareChain {
    middlewares: Vec<Box<dyn ProxyMiddleware>>,
}

// Processes request through all middleware in order
impl MiddlewareChain {
    pub async fn process(&self, mut req: Request) -> Result<ProxyMiddlewareResult> {
        for middleware in &self.middlewares {
            match middleware.process(req).await? {
                ProxyMiddlewareResult::Continue(r) => req = r,
                ProxyMiddlewareResult::Response(resp) => {
                    return Ok(ProxyMiddlewareResult::Response(resp));
                }
            }
        }
        Ok(ProxyMiddlewareResult::Continue(req))
    }
}
```

**YAML Example:**
```yaml
services:
  api:
    proxy:
      middlewares:
        - type: cors
          cors:
            origins: ["https://example.com"]
            methods: [GET, POST, PUT, DELETE]
            headers: ["Content-Type", "Authorization"]

        - type: rate-limit
          rate-limit:
            requests_per_minute: 60
            key: ip
```

---

### 2.5 Observability Plugin

**Purpose:** Capture logs, metrics, and events

**Trait Definition:**
```rust
#[async_trait]
pub trait ObsPlugin: Send + Sync {
    fn metadata(&self) -> PluginMetadata;

    async fn init(&mut self, config: &serde_json::Value) -> Result<()>;

    async fn handle(&self, event: &ObservabilityEvent);

    async fn flush(&self) -> Result<()>;

    async fn shutdown(&self) -> Result<()>;
}

pub enum ObservabilityEvent {
    Log {
        timestamp: DateTime<Utc>,
        level: LogLevel,
        source: String,
        message: String,
        fields: HashMap<String, String>,
    },

    ServiceEvent {
        timestamp: DateTime<Utc>,
        service_name: String,
        event_type: ServiceEventType,
        details: HashMap<String, String>,
    },

    HealthCheck {
        timestamp: DateTime<Utc>,
        service_name: String,
        result: HealthResult,
    },

    Metric {
        timestamp: DateTime<Utc>,
        name: String,
        value: f64,
        labels: HashMap<String, String>,
    },
}
```

**Available Obs Plugins:**
- **hive.obs.stdout** (bundled) - Console output (pretty, JSON, compact)
- **hive.obs.file** (bundled) - File logging with rotation
- **hive.obs.loki** (external) - Grafana Loki integration
- **hive.obs.prometheus** (external) - Prometheus metrics exporter

**Event Distribution:**
- EventCollector broadcasts to all obs plugins
- Uses tokio broadcast channels
- Fire-and-forget (no backpressure)
- Plugins handle events asynchronously

**YAML Example:**
```yaml
observability:
  plugins:
    - type: stdout
      stdout:
        format: pretty
        level: info
        colors: true

    - type: loki
      loki:
        url: https://loki.example.com
        labels:
          environment: production
          service: hive
```

---

### 2.6 Rollout Plugin

**Purpose:** Deployment strategies (zero-downtime updates)

**Trait Definition:**
```rust
#[async_trait]
pub trait RolloutPlugin: Send + Sync {
    fn metadata(&self) -> PluginMetadata;

    async fn init(&mut self, config: &serde_json::Value) -> Result<()>;

    async fn plan(
        &self,
        config: &serde_json::Value,
    ) -> Result<Vec<RolloutStep>>;

    async fn execute_step(
        &self,
        step: &RolloutStep,
        context: &RolloutContext,
    ) -> Result<RolloutStepResult>;

    async fn rollback(
        &self,
        context: &RolloutContext,
    ) -> Result<()>;

    async fn shutdown(&self) -> Result<()>;
}

pub enum RolloutStep {
    Stop { instance: String },
    Start { instance: String },
    WaitHealthy { instance: String, timeout: Duration },
    SwitchTraffic { from: String, to: String },
    Wait { duration: Duration },
    Command { command: String, args: Vec<String> },
}

pub struct RolloutContext {
    pub service_name: String,
    pub old_version: String,
    pub new_version: String,
    pub instances: HashMap<String, ProcessHandle>,
    pub metadata: HashMap<String, String>,
}
```

**Available Rollout Strategies:**
- **hive.rollout.recreate** (built-in) - Stop old, start new (downtime)
- **hive.rollout.blue-green** (external) - Dual environment, instant switch

**Blue-Green Rollout Steps:**
1. Start new version (green) on different port
2. Wait for green to be healthy
3. Switch proxy traffic from blue → green
4. Stop old version (blue)
5. Rename green → blue for next deployment

**YAML Example:**
```yaml
services:
  api:
    rollout:
      type: blue-green
      blue-green:
        health_check_timeout: 30s
        traffic_switch_delay: 5s
```

---

## 3. Plugin Loading & Registration

### 3.1 Bundled Plugin Initialization

**Feature-Gated Compilation** (`Cargo.toml`):
```toml
[features]
default = ["bundled-plugins"]
bundled-plugins = [
    "plugin-docker",
    "plugin-obs-stdout",
    "plugin-obs-file",
    "plugin-proxy-cors",
    "plugin-proxy-rate-limit",
    "plugin-health-http",
    "plugin-health-tcp",
    "plugin-env-dotenv",
]

plugin-docker = ["dep:hive-runner-docker"]
plugin-obs-stdout = ["dep:hive-obs-stdout"]
plugin-obs-file = ["dep:hive-obs-file"]
# ... etc
```

**Initialization Flow** (`plugins.rs`):
```rust
pub async fn init_bundled_plugins() -> PluginManager {
    let manager = PluginManager::new();

    #[cfg(feature = "plugin-docker")]
    {
        let mut plugin = hive_runner_docker::DockerRunnerPlugin::new();
        plugin.init(&serde_json::json!({})).await.ok();
        manager.register_runner(plugin).await;
    }

    #[cfg(feature = "plugin-obs-stdout")]
    {
        let mut plugin = hive_obs_stdout::StdoutObsPlugin::new();
        plugin.init(&serde_json::json!({})).await.ok();
        manager.register_obs(plugin).await;
    }

    // ... repeat for all bundled plugins

    manager
}
```

**Global Singleton Access:**
```rust
static PLUGIN_MANAGER: std::sync::OnceLock<PluginManager> = std::sync::OnceLock::new();

pub fn plugin_manager() -> &'static PluginManager {
    PLUGIN_MANAGER.get_or_init(|| {
        tokio::runtime::Handle::current()
            .block_on(init_bundled_plugins())
    })
}
```

---

### 3.2 Dynamic Plugin Loading (Infrastructure)

**Plugin Loader** (`lib-plugin-abi-orchestration::loader`):
```rust
pub struct PluginLoader {
    plugin_dir: PathBuf,
}

impl PluginLoader {
    pub fn new(plugin_dir: PathBuf) -> Self {
        Self { plugin_dir }
    }

    pub fn discover_plugins(&self) -> Result<Vec<PluginManifest>> {
        // Scans plugin_dir for .so/.dylib files
        // Reads plugin metadata from embedded JSON
        // Returns list of available plugins
    }

    pub unsafe fn load_plugin<T>(&self, path: &Path) -> Result<Box<T>> {
        // Loads shared library via dlopen/LoadLibrary
        // Calls plugin_entry() to get trait implementation
        // Returns boxed plugin instance
    }
}
```

**Status:** Infrastructure exists but not fully integrated into `plugins.rs`

**Future Work:**
- Automatic discovery in `~/.local/share/adi/plugins/`
- Hot-reload support (unload/reload on file change)
- Plugin versioning and compatibility checks

---

### 3.3 Plugin Directory Structure

**Bundled Plugins** (in-tree):
```
crates/hive/plugins/
├── hive-runner-docker/
│   ├── Cargo.toml
│   └── src/lib.rs          # RunnerPlugin implementation
├── hive-obs-stdout/
│   ├── Cargo.toml
│   └── src/lib.rs          # ObsPlugin implementation
├── hive-proxy-cors/
│   ├── Cargo.toml
│   └── src/lib.rs          # ProxyPlugin implementation
└── ... (8 total bundled)
```

**External Plugins** (via `adi plugin install`):
```
~/.local/share/adi/plugins/
├── hive.runner.podman.so
├── hive.health.grpc.so
├── hive.proxy.auth-jwt.so
└── ... (installed via registry)
```

---

## 4. Plugin Usage in ServiceManager

### 4.1 Runner Integration

**Starting a Service:**
```rust
// In service_manager/mod.rs
pub async fn start_service(&self, name: &str) -> Result<()> {
    let config = self.config.services.get(name)?;
    let runner_type = &config.runner.runner_type;

    // Get runner plugin
    let runner = plugin_manager()
        .get_runner(runner_type)
        .await
        .ok_or_else(|| anyhow!("Runner '{}' not found", runner_type))?;

    // Build environment
    let env = self.build_environment(name, config).await?;

    // Create runtime context
    let ctx = RuntimeContext {
        service_name: name.to_string(),
        ports: self.allocate_ports(config)?,
        env: env.clone(),
        working_dir: self.project_root.clone(),
    };

    // Start service via runner
    let handle = runner.start(
        name,
        &serde_json::to_value(&config.runner)?,
        env,
        &ctx,
    ).await?;

    // Store handle for later stop/restart
    self.services.insert(name.to_string(), ServiceRuntime {
        state: ServiceState::Running,
        process: Some(handle),
        healthy: None,
    });

    Ok(())
}
```

**Stopping a Service:**
```rust
pub async fn stop_service(&self, name: &str) -> Result<()> {
    let runtime = self.services.get(name)?;
    let handle = runtime.process.as_ref()?;

    // Get runner plugin
    let runner = plugin_manager()
        .get_runner(&handle.runner_type)
        .await?;

    // Stop via runner
    runner.stop(handle).await?;

    // Update state
    self.services.insert(name.to_string(), ServiceRuntime {
        state: ServiceState::Stopped,
        process: None,
        healthy: None,
    });

    Ok(())
}
```

---

### 4.2 Health Check Integration

**Health Checker** (`service_manager/health.rs`):
```rust
pub struct HealthChecker {
    // ...
}

impl HealthChecker {
    pub async fn run_checks(
        &self,
        service_name: &str,
        config: &HealthCheckConfig,
        ctx: &RuntimeContext,
    ) -> Result<bool> {
        let mut all_healthy = true;

        for check in &config.checks {
            // Get health plugin
            let plugin = plugin_manager()
                .get_health(&check.check_type)
                .await?;

            // Run health check
            let result = plugin.check(
                &serde_json::to_value(check)?,
                ctx,
            ).await?;

            if !result.healthy {
                warn!(
                    "Service {} health check '{}' failed: {}",
                    service_name,
                    check.check_type,
                    result.message.unwrap_or_default()
                );
                all_healthy = false;
            }
        }

        Ok(all_healthy)
    }
}
```

**Usage in ServiceManager:**
```rust
// After starting service, wait for healthy
let healthy = tokio::time::timeout(
    Duration::from_secs(30),
    async {
        loop {
            if self.health_checker.run_checks(name, &config.health, &ctx).await? {
                return Ok(true);
            }
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    }
).await??;

if healthy {
    // Enable proxy routing
    self.proxy_state.update_service_port(name, ctx.ports["main"]);
}
```

---

### 4.3 Environment Resolution

**EnvironmentResolver** (`service_manager/environment.rs`):
```rust
pub struct EnvironmentResolver {
    plugins: Vec<Box<dyn EnvPlugin>>,
}

impl EnvironmentResolver {
    pub async fn resolve(&self, service_config: &ServiceConfig) -> Result<HashMap<String, String>> {
        let mut env = HashMap::new();

        // Load from env plugins in order
        for env_config in &service_config.env {
            let plugin = plugin_manager()
                .get_env(&env_config.env_type)
                .await?;

            let vars = plugin.load(
                &serde_json::to_value(env_config)?
            ).await?;

            // Merge (later plugins override earlier)
            env.extend(vars);
        }

        // Add service-specific env vars
        env.extend(service_config.environment.clone());

        Ok(env)
    }
}
```

---

### 4.4 Lifecycle Hooks

**Hook Execution** (`service_manager/mod.rs`):
```rust
pub async fn execute_hook(
    &self,
    service_name: &str,
    hook_type: HookType,
) -> Result<()> {
    let config = self.config.services.get(service_name)?;
    let hooks = match hook_type {
        HookType::PreUp => &config.hooks.pre_up,
        HookType::PostUp => &config.hooks.post_up,
        HookType::PreDown => &config.hooks.pre_down,
        HookType::PostDown => &config.hooks.post_down,
    };

    for hook in hooks {
        // Get runner that supports hooks
        let runner = plugin_manager()
            .get_runner(&hook.runner)
            .await?;

        if !runner.supports_hooks() {
            return Err(anyhow!("Runner '{}' does not support hooks", hook.runner));
        }

        // Execute hook
        let status = runner.run_hook(
            &serde_json::to_value(hook)?,
            self.build_environment(service_name, config).await?,
            &self.build_context(service_name)?,
        ).await?;

        // Handle failure
        if !status.success {
            match hook.on_failure {
                OnFailure::Abort => return Err(anyhow!("Hook failed: {}", status.output)),
                OnFailure::Warn => warn!("Hook failed (continuing): {}", status.output),
            }
        }
    }

    Ok(())
}
```

**YAML Example:**
```yaml
services:
  api:
    hooks:
      pre_up:
        - runner: docker
          docker:
            image: migrate:latest
            command: ["migrate", "up"]
          on_failure: abort

      post_down:
        - runner: docker
          docker:
            image: cleanup:latest
            command: ["cleanup"]
          on_failure: warn
```

---

## 5. Configuration System

### 5.1 Plugin Configuration Format

**General Pattern:**
```yaml
services:
  <service-name>:
    <plugin-category>:
      type: <plugin-id>
      <plugin-id>:
        # Plugin-specific config
```

**Example (Full Service Config):**
```yaml
services:
  api:
    runner:
      type: docker
      docker:
        image: myapp:v1.2.3
        ports:
          - "{{runtime.port.main}}:8080"
          - "{{runtime.port.metrics}}:9090"
        volumes:
          - ./data:/data
        environment:
          LOG_LEVEL: info

    health:
      - type: http
        http:
          port: "{{runtime.port.main}}"
          path: /health
          timeout: 5s

      - type: tcp
        tcp:
          port: "{{runtime.port.metrics}}"

    env:
      - type: dotenv
        dotenv:
          files: [.env]

      - type: vault
        vault:
          address: https://vault.example.com
          path: secret/data/myapp

    proxy:
      middlewares:
        - type: cors
          cors:
            origins: ["*"]

        - type: rate-limit
          rate-limit:
            requests_per_minute: 100

    rollout:
      type: blue-green
      blue-green:
        health_check_timeout: 30s
```

---

### 5.2 Variable Interpolation

**Supported Variables:**
- `{{runtime.port.<name>}}` - Dynamically allocated port
- `{{env.<VAR>}}` - Environment variable
- `{{service.<name>.port.<port>}}` - Port from another service
- `{{expose.<source>.<service>.<key>}}` - Cross-source dependencies

**RuntimeContext:**
```rust
pub struct RuntimeContext {
    pub service_name: String,
    pub ports: HashMap<String, u16>,       // Allocated ports
    pub env: HashMap<String, String>,      // Resolved environment
    pub working_dir: PathBuf,
    pub metadata: HashMap<String, String>,
}
```

**Port Allocation:**
```rust
// In service_manager/mod.rs
fn allocate_ports(&self, config: &ServiceConfig) -> Result<HashMap<String, u16>> {
    let mut ports = HashMap::new();

    // Main port (required)
    ports.insert("main".to_string(), self.port_allocator.allocate()?);

    // Additional named ports
    for port_name in &config.additional_ports {
        ports.insert(port_name.clone(), self.port_allocator.allocate()?);
    }

    Ok(ports)
}
```

---

## 6. Observability Integration

### 6.1 Event Collection

**EventCollector** (`observability.rs`):
```rust
pub struct EventCollector {
    sender: broadcast::Sender<ObservabilityEvent>,
}

impl EventCollector {
    pub fn emit(&self, event: ObservabilityEvent) {
        let _ = self.sender.send(event);  // Fire-and-forget
    }
}
```

**ServiceManager Integration:**
```rust
// Emit events during service lifecycle
self.event_collector.emit(ObservabilityEvent::ServiceEvent {
    timestamp: Utc::now(),
    service_name: name.to_string(),
    event_type: ServiceEventType::Starting,
    details: HashMap::new(),
});

// ... start service ...

self.event_collector.emit(ObservabilityEvent::ServiceEvent {
    timestamp: Utc::now(),
    service_name: name.to_string(),
    event_type: ServiceEventType::Healthy,
    details: [("port".to_string(), port.to_string())].into(),
});
```

---

### 6.2 Obs Plugin Distribution

**ObsPlugins Manager** (`observability_plugins.rs`):
```rust
pub struct ObsPlugins {
    plugins: Vec<Arc<dyn ObsPlugin>>,
    receiver: broadcast::Receiver<ObservabilityEvent>,
}

impl ObsPlugins {
    pub async fn start(mut self) {
        tokio::spawn(async move {
            while let Ok(event) = self.receiver.recv().await {
                // Distribute to all obs plugins
                for plugin in &self.plugins {
                    plugin.handle(&event).await;
                }
            }
        });
    }
}
```

---

## 7. Plugin Development Guide

### 7.1 Creating a New Plugin

**Step 1: Create Plugin Crate**
```bash
cargo new --lib hive-health-mongodb
cd hive-health-mongodb
```

**Step 2: Add Dependencies** (`Cargo.toml`):
```toml
[package]
name = "hive-health-mongodb"
version = "0.1.0"
edition = "2021"

[dependencies]
lib-plugin-abi-orchestration = { path = "../../lib/lib-plugin-abi-orchestration" }
async-trait = "0.1"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
mongodb = "2.0"
anyhow = "1.0"
```

**Step 3: Implement Plugin Trait** (`src/lib.rs`):
```rust
use async_trait::async_trait;
use lib_plugin_abi_orchestration::{
    health::{HealthPlugin, HealthResult},
    PluginMetadata,
};
use serde::Deserialize;

#[derive(Deserialize)]
struct MongoConfig {
    uri: String,
    database: String,
    timeout_ms: Option<u64>,
}

pub struct MongoHealthPlugin;

impl MongoHealthPlugin {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl HealthPlugin for MongoHealthPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            id: "hive.health.mongodb".to_string(),
            name: "MongoDB Health Check".to_string(),
            version: "0.1.0".to_string(),
            description: Some("Check MongoDB connection health".to_string()),
            author: Some("Your Name".to_string()),
        }
    }

    async fn init(&mut self, _defaults: &serde_json::Value) -> anyhow::Result<()> {
        Ok(())
    }

    async fn check(
        &self,
        config: &serde_json::Value,
        _ctx: &lib_plugin_abi_orchestration::RuntimeContext,
    ) -> anyhow::Result<HealthResult> {
        let config: MongoConfig = serde_json::from_value(config.clone())?;
        let start = std::time::Instant::now();

        // Connect to MongoDB
        let client = mongodb::Client::with_uri_str(&config.uri).await?;
        let db = client.database(&config.database);

        // Run ping command
        db.run_command(bson::doc! { "ping": 1 }, None).await?;

        let elapsed = start.elapsed().as_millis() as u64;

        Ok(HealthResult {
            healthy: true,
            message: Some(format!("MongoDB {} is reachable", config.database)),
            response_time_ms: elapsed,
            details: [
                ("database".to_string(), config.database),
            ].into(),
        })
    }

    async fn shutdown(&self) -> anyhow::Result<()> {
        Ok(())
    }
}
```

**Step 4: Register in PluginManager** (for bundled):
```rust
// In hive-core/src/plugins.rs
#[cfg(feature = "plugin-health-mongodb")]
{
    use hive_health_mongodb::MongoHealthPlugin;
    let mut plugin = MongoHealthPlugin::new();
    plugin.init(&serde_json::json!({})).await?;
    manager.register_health(plugin).await;
}
```

**Step 5: Add Feature Flag** (`hive-core/Cargo.toml`):
```toml
[features]
plugin-health-mongodb = ["dep:hive-health-mongodb"]

[dependencies]
hive-health-mongodb = { path = "../plugins/hive-health-mongodb", optional = true }
```

---

### 7.2 Testing a Plugin

**Unit Test:**
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use lib_plugin_abi_orchestration::RuntimeContext;

    #[tokio::test]
    async fn test_mongo_health_check() {
        let plugin = MongoHealthPlugin::new();

        let config = serde_json::json!({
            "uri": "mongodb://localhost:27017",
            "database": "test",
        });

        let ctx = RuntimeContext {
            service_name: "test-service".to_string(),
            ports: HashMap::new(),
            env: HashMap::new(),
            working_dir: PathBuf::from("."),
        };

        let result = plugin.check(&config, &ctx).await.unwrap();
        assert!(result.healthy);
    }
}
```

**Integration Test** (`hive.yaml`):
```yaml
services:
  mongo-test:
    runner:
      type: docker
      docker:
        image: mongo:7

    health:
      - type: mongodb
        mongodb:
          uri: "mongodb://localhost:{{runtime.port.main}}"
          database: test
          timeout_ms: 5000
```

---

## 8. Complete Plugin Inventory

### 8.1 All 32 Plugins

| Category | Plugin ID | Status | Description |
|----------|-----------|--------|-------------|
| **Runner** (3) ||||
| | hive.runner.docker | Bundled | Docker container execution |
| | hive.runner.compose | External | Docker Compose orchestration |
| | hive.runner.podman | External | Podman container execution |
| **Env** (4) ||||
| | hive.env.dotenv | Bundled | Load .env files |
| | hive.env.vault | External | HashiCorp Vault secrets |
| | hive.env.1password | External | 1Password secret references |
| | hive.env.aws-secrets | External | AWS Secrets Manager |
| **Health** (7) ||||
| | hive.health.http | Bundled | HTTP endpoint checks |
| | hive.health.tcp | Bundled | TCP port connectivity |
| | hive.health.cmd | External | Command exit code checks |
| | hive.health.grpc | External | gRPC health protocol |
| | hive.health.mysql | External | MySQL connection checks |
| | hive.health.postgres | External | PostgreSQL connection checks |
| | hive.health.redis | External | Redis PING checks |
| **Proxy** (11) ||||
| | hive.proxy.cors | Bundled | CORS headers |
| | hive.proxy.rate-limit | Bundled | Rate limiting |
| | hive.proxy.headers | External | HTTP header manipulation |
| | hive.proxy.ip-filter | External | IP allow/deny lists |
| | hive.proxy.auth-api-key | External | API key authentication |
| | hive.proxy.auth-basic | External | HTTP Basic auth |
| | hive.proxy.auth-jwt | External | JWT token validation |
| | hive.proxy.auth-oidc | External | OpenID Connect auth |
| | hive.proxy.cache | External | HTTP caching |
| | hive.proxy.compress | External | Response compression |
| | hive.proxy.rewrite | External | URL rewriting |
| **Obs** (4) ||||
| | hive.obs.stdout | Bundled | Console output |
| | hive.obs.file | Bundled | File logging with rotation |
| | hive.obs.loki | External | Grafana Loki integration |
| | hive.obs.prometheus | External | Prometheus metrics |
| **Rollout** (2) ||||
| | hive.rollout.recreate | Built-in | Stop-then-start deployment |
| | hive.rollout.blue-green | External | Zero-downtime blue-green |
| **Other** (1) ||||
| | hive.orchestrator | Special | Orchestrator metadata |

**Total: 32 plugins (8 bundled, 24 external)**

---

### 8.2 Bundled by Default

These 8 plugins are compiled into hive-core when `bundled-plugins` feature is enabled (default):

1. **hive.runner.docker** - Essential for container execution
2. **hive.obs.stdout** - Console logging
3. **hive.obs.file** - File logging
4. **hive.proxy.cors** - CORS headers
5. **hive.proxy.rate-limit** - Rate limiting
6. **hive.health.http** - HTTP health checks
7. **hive.health.tcp** - TCP connectivity checks
8. **hive.env.dotenv** - .env file loading

**Why Bundled:**
- Core functionality (most deployments need these)
- Zero external dependencies
- Fast initialization (no dynamic loading)
- Single binary deployment

---

## 9. Architecture Analysis

### 9.1 Strengths

1. **Type Safety**: Trait-based system provides compile-time guarantees
2. **Extensibility**: Easy to add new plugins (implement trait + register)
3. **Separation of Concerns**: Each plugin category has dedicated trait
4. **Feature Flags**: Optional bundling reduces binary size
5. **Async-First**: All plugin methods are async (tokio-native)
6. **Global Singleton**: Easy access via `plugin_manager()` anywhere
7. **Standardized ABI**: `lib-plugin-abi-orchestration` enables cross-orchestrator plugins
8. **Configuration-Driven**: YAML-based plugin config (no code changes)

---

### 9.2 Weaknesses

1. **Dual System Confusion**: Legacy `plugin_system.rs` vs modern `plugins.rs`
2. **No Schema Validation**: Plugins receive `serde_json::Value` (runtime errors)
3. **Manual Registration**: Bundled plugins require manual init code
4. **No Hot-Reload**: Dynamic loading infrastructure exists but not integrated
5. **Global State**: Singleton pattern makes testing harder
6. **No Versioning**: No plugin compatibility checks
7. **Limited Documentation**: Plugin development guide needs expansion
8. **Auto-Install Complexity**: Mixed blessing (convenient but unpredictable)

---

### 9.3 Recommendations

#### **Short-Term (Next 2 Months)**

1. **Unify Plugin Systems**
   - Deprecate `plugin_system.rs`
   - Move auto-install logic to `plugins.rs`
   - Single `PluginManager` as source of truth

2. **Add Schema Validation**
   - Plugins export JSON Schema for their config
   - Validate at config load time (not runtime)
   - Better error messages for misconfiguration

3. **Improve Documentation**
   - Expand plugin development guide
   - Add example plugins for each category
   - Document configuration schema for all bundled plugins

#### **Medium-Term (3-6 Months)**

4. **Dynamic Loading Integration**
   - Complete integration of `PluginLoader` into `PluginManager`
   - Auto-discovery in `~/.local/share/adi/plugins/`
   - Plugin versioning and compatibility checks

5. **Plugin Registry Enhancements**
   - Centralized plugin registry (like crates.io)
   - `adi plugin search`, `adi plugin publish`
   - Automated plugin builds and distribution

6. **Testing Infrastructure**
   - Mock plugin implementations for testing
   - Plugin test harness (run all plugins against test suite)
   - Integration tests for plugin interactions

#### **Long-Term (6-12 Months)**

7. **Hot-Reload Support**
   - Watch plugin directory for changes
   - Reload plugins without restarting Hive
   - Graceful plugin upgrade (drain connections, reload)

8. **Plugin Marketplace**
   - Web UI for browsing plugins
   - Plugin ratings and reviews
   - Automated security scanning

9. **Plugin Composition**
   - Higher-level plugins built from primitives
   - Plugin dependencies (plugin A requires plugin B)
   - Plugin templates for common patterns

---

## 10. Migration Path

### Phase 1: Unify Plugin Systems (Immediate)

**Goal:** Single plugin system with clear responsibilities

**Actions:**
1. Create unified `PluginLifecycle` struct:
   ```rust
   pub struct PluginLifecycle {
       manager: PluginManager,      // Execution
       registry: PluginMetadata,    // Discovery
       installer: PluginInstaller,  // Auto-install
   }
   ```

2. Migrate auto-install from `plugin_system.rs` to new `PluginInstaller`

3. Mark `plugin_system.rs` as `#[deprecated]` with migration timeline

4. Update all references to use `PluginLifecycle`

**Timeline:** 2-4 weeks

---

### Phase 2: Schema Validation (Next)

**Goal:** Catch configuration errors at load time

**Actions:**
1. Add `schema()` method to all plugin traits:
   ```rust
   fn schema(&self) -> serde_json::Value {
       serde_json::json!({
           "type": "object",
           "properties": {
               "uri": { "type": "string" },
               "timeout": { "type": "integer" }
           },
           "required": ["uri"]
       })
   }
   ```

2. Validate plugin configs when loading `hive.yaml`

3. Generate documentation from schemas

**Timeline:** 3-4 weeks

---

### Phase 3: Complete Dynamic Loading (Later)

**Goal:** Full runtime plugin loading support

**Actions:**
1. Integrate `PluginLoader` into `PluginManager`
2. Auto-discover plugins in standard directory
3. Add plugin version compatibility checks
4. Support plugin updates without Hive restart

**Timeline:** 6-8 weeks

---

## Appendix A: Plugin Trait Reference

### Common Types

```rust
pub struct PluginMetadata {
    pub id: String,              // e.g., "hive.runner.docker"
    pub name: String,            // Human-readable name
    pub version: String,         // Semantic version
    pub description: Option<String>,
    pub author: Option<String>,
}

pub struct RuntimeContext {
    pub service_name: String,
    pub ports: HashMap<String, u16>,
    pub env: HashMap<String, String>,
    pub working_dir: PathBuf,
}
```

---

## Appendix B: Plugin Configuration Examples

### Runner (Docker)
```yaml
runner:
  type: docker
  docker:
    image: myapp:latest
    ports:
      - "{{runtime.port.main}}:8080"
    volumes:
      - ./data:/data
    environment:
      LOG_LEVEL: debug
```

### Health (HTTP)
```yaml
health:
  - type: http
    http:
      port: "{{runtime.port.main}}"
      path: /health
      method: GET
      timeout: 5s
      expected_status: 200
      expected_body: "OK"
```

### Env (Vault)
```yaml
env:
  - type: vault
    vault:
      address: https://vault.example.com
      path: secret/data/myapp
      token: "{{env.VAULT_TOKEN}}"
      fields:
        - DATABASE_URL
        - API_KEY
```

### Proxy (CORS)
```yaml
proxy:
  middlewares:
    - type: cors
      cors:
        origins:
          - https://example.com
          - https://app.example.com
        methods: [GET, POST, PUT, DELETE]
        headers: [Content-Type, Authorization]
        max_age: 3600
```

### Obs (Loki)
```yaml
observability:
  plugins:
    - type: loki
      loki:
        url: https://loki.example.com
        batch_size: 100
        flush_interval: 5s
        labels:
          environment: production
          service: hive
```

### Rollout (Blue-Green)
```yaml
rollout:
  type: blue-green
  blue-green:
    health_check_timeout: 30s
    traffic_switch_delay: 5s
    keep_old_version: true
```

---

## Appendix C: Glossary

- **ABI (Application Binary Interface)**: Standardized trait definitions for plugins
- **Bundled Plugin**: Compiled into hive-core, available without installation
- **Dynamic Plugin**: Loaded at runtime from shared library (.so/.dylib)
- **External Plugin**: Installed via `adi plugin install` (may be bundled or dynamic)
- **Feature Flag**: Cargo feature that controls conditional compilation
- **Middleware**: Proxy plugin that intercepts HTTP requests/responses
- **Plugin Category**: Group of plugins with same trait (Runner, Health, etc.)
- **Plugin Manager**: Central registry of loaded plugin instances
- **Plugin Registry**: Metadata database of available plugins
- **RuntimeContext**: Context passed to plugins with service info
- **Singleton**: Global instance accessible via static function
- **Trait**: Rust interface that plugins implement

---

**Document Version:** 1.0
**Last Updated:** 2026-01-31
**Maintainer:** ADI Development Team
