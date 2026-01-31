# lib-plugin-abi-v3: Unified Plugin ABI Design

**Date:** 2026-01-31
**Status:** Design Phase
**Replaces:** lib-plugin-abi (v2), lib-plugin-abi-orchestration

---

## Goals

1. **Unify** general and orchestration plugin ABIs into single system
2. **Simplify** by using native Rust async traits (no FFI complexity)
3. **Maintain** all current functionality across 86+ plugins
4. **Enable** easier plugin development and maintenance

---

## Core Design Principles

### 1. Native Rust Async Traits

**Before (v2 - FFI-safe):**
```rust
// Complex FFI-safe callback pattern
pub type AsyncCheckFn = extern "C" fn(
    handle: ServiceHandle,
    config: RString,
    callback: extern "C" fn(*mut c_void, RResult<RString, RString>),
    callback_data: *mut c_void,
);
```

**After (v3 - Native):**
```rust
#[async_trait]
pub trait HealthCheck: Plugin {
    async fn check(&self, config: &Value) -> Result<HealthResult>;
}
```

### 2. Trait Composition

All plugins implement base `Plugin` trait, then add service-specific traits:

```rust
// Base trait (required for all plugins)
#[async_trait]
pub trait Plugin: Send + Sync {
    fn metadata(&self) -> PluginMetadata;
    async fn init(&mut self, ctx: &PluginContext) -> Result<()>;
    async fn shutdown(&self) -> Result<()>;
}

// Service traits (opt-in)
#[async_trait]
pub trait CliCommands: Plugin {
    async fn list_commands(&self) -> Vec<CliCommand>;
    async fn run_command(&self, ctx: &CliContext) -> Result<CliResult>;
}

#[async_trait]
pub trait HealthCheck: Plugin {
    async fn check(&self, config: &Value, ctx: &RuntimeContext) -> Result<HealthResult>;
}
```

### 3. Dynamic Dispatch via Trait Objects

Plugins are loaded as trait objects:

```rust
// Plugin manager stores Arc<dyn Trait>
pub struct PluginManager {
    cli_plugins: HashMap<String, Arc<dyn CliCommands>>,
    health_plugins: HashMap<String, Arc<dyn HealthCheck>>,
    // ...
}
```

### 4. Type-Safe Contexts

Replace JSON strings with proper types:

```rust
// v2: Everything is RString (JSON)
run_command(handle, context: RString) -> RResult<RString, RString>

// v3: Strongly typed
async fn run_command(&self, ctx: &CliContext) -> Result<CliResult>

pub struct CliContext {
    pub command: String,
    pub subcommand: Option<String>,
    pub args: Vec<String>,
    pub options: HashMap<String, Value>,
    pub cwd: PathBuf,
}
```

---

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    lib-plugin-abi-v3                        │
├─────────────────────────────────────────────────────────────┤
│                                                               │
│  Core Traits                                                 │
│  ├── Plugin (base)                                           │
│  ├── PluginMetadata                                          │
│  └── PluginContext                                           │
│                                                               │
│  Service Traits (General)                                    │
│  ├── CliCommands                                             │
│  ├── HttpRoutes                                              │
│  ├── McpTools                                                │
│  ├── McpResources                                            │
│  └── McpPrompts                                              │
│                                                               │
│  Service Traits (Orchestration)                              │
│  ├── Runner                                                  │
│  ├── HealthCheck                                             │
│  ├── EnvProvider                                             │
│  ├── ProxyMiddleware                                         │
│  ├── ObservabilitySink                                       │
│  └── RolloutStrategy                                         │
│                                                               │
│  Utilities                                                    │
│  ├── Error types                                             │
│  ├── Result aliases                                          │
│  └── Common types                                            │
│                                                               │
└─────────────────────────────────────────────────────────────┘
```

---

## Trait Definitions

### Core: Plugin Trait

```rust
use async_trait::async_trait;
use serde_json::Value;
use std::path::PathBuf;

/// Base trait that all plugins must implement
#[async_trait]
pub trait Plugin: Send + Sync {
    /// Returns plugin metadata
    fn metadata(&self) -> PluginMetadata;

    /// Initialize plugin with context
    /// Called once when plugin is loaded
    async fn init(&mut self, ctx: &PluginContext) -> Result<()>;

    /// Shutdown plugin gracefully
    /// Called before plugin is unloaded
    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    /// Optional: Handle custom events/messages
    async fn handle_event(&self, event: &PluginEvent) -> Result<()> {
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct PluginMetadata {
    pub id: String,              // "adi.tasks"
    pub name: String,            // "ADI Tasks"
    pub version: String,         // "0.8.8"
    pub plugin_type: PluginType, // Core, Extension, Theme
    pub author: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluginType {
    Core,
    Extension,
    Theme,
    Font,
}

pub struct PluginContext {
    pub plugin_id: String,
    pub data_dir: PathBuf,       // ~/.local/share/adi/<plugin-id>/
    pub config_dir: PathBuf,     // ~/.config/adi/<plugin-id>/
    pub config: Value,           // Plugin config from config.toml
}

pub enum PluginEvent {
    ConfigChanged(Value),
    HostShutdown,
    Custom { event_type: String, data: Value },
}
```

---

### Service: CLI Commands

```rust
#[async_trait]
pub trait CliCommands: Plugin {
    /// List all CLI commands provided by this plugin
    async fn list_commands(&self) -> Vec<CliCommand>;

    /// Execute a CLI command
    async fn run_command(&self, ctx: &CliContext) -> Result<CliResult>;
}

#[derive(Debug, Clone)]
pub struct CliCommand {
    pub name: String,            // "list"
    pub description: String,     // "List all tasks"
    pub usage: String,           // "tasks list [--filter <filter>]"
    pub has_subcommands: bool,
}

pub struct CliContext {
    pub command: String,         // "tasks"
    pub subcommand: Option<String>, // "list"
    pub args: Vec<String>,       // Positional arguments
    pub options: HashMap<String, Value>, // Parsed flags/options
    pub cwd: PathBuf,
    pub env: HashMap<String, String>,
}

#[derive(Debug)]
pub struct CliResult {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
}

impl CliResult {
    pub fn success(output: impl Into<String>) -> Self {
        Self {
            exit_code: 0,
            stdout: output.into(),
            stderr: String::new(),
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self {
            exit_code: 1,
            stdout: String::new(),
            stderr: message.into(),
        }
    }
}
```

---

### Service: HTTP Routes

```rust
use axum::http::{Request, Response, StatusCode};
use bytes::Bytes;

#[async_trait]
pub trait HttpRoutes: Plugin {
    /// List all HTTP routes provided by this plugin
    async fn list_routes(&self) -> Vec<HttpRoute>;

    /// Handle an HTTP request
    async fn handle_request(&self, req: HttpRequest) -> Result<HttpResponse>;
}

#[derive(Debug, Clone)]
pub struct HttpRoute {
    pub method: HttpMethod,      // GET, POST, PUT, DELETE, etc.
    pub path: String,            // "/api/tasks"
    pub handler_id: String,      // Internal identifier
    pub description: String,     // "List all tasks"
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Delete,
    Patch,
    Head,
    Options,
    Trace,
}

pub struct HttpRequest {
    pub method: HttpMethod,
    pub path: String,
    pub query: HashMap<String, String>,
    pub headers: HashMap<String, String>,
    pub body: Bytes,
    pub params: HashMap<String, String>, // Path parameters
}

pub struct HttpResponse {
    pub status: StatusCode,
    pub headers: HashMap<String, String>,
    pub body: Bytes,
}

impl HttpResponse {
    pub fn ok(body: impl Into<Bytes>) -> Self {
        Self {
            status: StatusCode::OK,
            headers: HashMap::new(),
            body: body.into(),
        }
    }

    pub fn json<T: serde::Serialize>(data: &T) -> Result<Self> {
        let body = serde_json::to_vec(data)?;
        Ok(Self {
            status: StatusCode::OK,
            headers: [("content-type".to_string(), "application/json".to_string())].into(),
            body: body.into(),
        })
    }

    pub fn error(status: StatusCode, message: impl Into<String>) -> Self {
        Self {
            status,
            headers: HashMap::new(),
            body: message.into().into(),
        }
    }
}
```

---

### Service: MCP Tools

```rust
#[async_trait]
pub trait McpTools: Plugin {
    /// List all MCP tools provided by this plugin
    async fn list_tools(&self) -> Vec<McpTool>;

    /// Call an MCP tool
    async fn call_tool(&self, name: &str, arguments: Value) -> Result<McpToolResult>;
}

#[derive(Debug, Clone)]
pub struct McpTool {
    pub name: String,            // "search_code"
    pub description: String,     // "Search codebase semantically"
    pub input_schema: Value,     // JSON Schema for arguments
}

#[derive(Debug)]
pub struct McpToolResult {
    pub content: String,
    pub content_type: McpContentType,
    pub is_error: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum McpContentType {
    Text,
    Json,
    Error,
}
```

---

### Service: MCP Resources

```rust
#[async_trait]
pub trait McpResources: Plugin {
    /// List all resources provided by this plugin
    async fn list_resources(&self) -> Vec<McpResource>;

    /// Read a resource by URI
    async fn read_resource(&self, uri: &str) -> Result<McpResourceContent>;
}

#[derive(Debug, Clone)]
pub struct McpResource {
    pub uri: String,             // "file:///project/src/main.rs"
    pub name: String,            // "main.rs"
    pub description: String,     // "Main entry point"
    pub mime_type: String,       // "text/x-rust"
}

#[derive(Debug)]
pub struct McpResourceContent {
    pub uri: String,
    pub content: Vec<u8>,        // Binary or text
    pub mime_type: String,
}
```

---

### Service: MCP Prompts

```rust
#[async_trait]
pub trait McpPrompts: Plugin {
    /// List all prompts provided by this plugin
    async fn list_prompts(&self) -> Vec<McpPrompt>;

    /// Get a prompt with arguments
    async fn get_prompt(&self, name: &str, arguments: Value) -> Result<Vec<McpPromptMessage>>;
}

#[derive(Debug, Clone)]
pub struct McpPrompt {
    pub name: String,            // "code_review"
    pub description: String,     // "Review code for issues"
    pub arguments_schema: Value, // JSON Schema
}

#[derive(Debug, Clone)]
pub struct McpPromptMessage {
    pub role: McpPromptRole,
    pub content: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum McpPromptRole {
    User,
    Assistant,
    System,
}
```

---

### Orchestration: Runner

```rust
#[async_trait]
pub trait Runner: Plugin {
    /// Start a service
    async fn start(
        &self,
        service_name: &str,
        config: &Value,
        env: HashMap<String, String>,
        ctx: &RuntimeContext,
    ) -> Result<ProcessHandle>;

    /// Stop a running service
    async fn stop(&self, handle: &ProcessHandle) -> Result<()>;

    /// Check if service is running
    async fn is_running(&self, handle: &ProcessHandle) -> bool;

    /// Get service logs
    async fn logs(&self, handle: &ProcessHandle, lines: Option<usize>) -> Result<Vec<String>>;

    /// Check if this runner supports lifecycle hooks
    fn supports_hooks(&self) -> bool {
        false
    }

    /// Run a lifecycle hook (if supported)
    async fn run_hook(
        &self,
        config: &Value,
        env: HashMap<String, String>,
        ctx: &RuntimeContext,
    ) -> Result<HookExitStatus> {
        Err(anyhow::anyhow!("Hooks not supported"))
    }
}

#[derive(Debug, Clone)]
pub struct ProcessHandle {
    pub id: String,              // Container ID, PID, etc.
    pub runner_type: String,     // "docker", "script", etc.
    pub metadata: HashMap<String, String>,
}

#[derive(Debug)]
pub struct HookExitStatus {
    pub success: bool,
    pub exit_code: i32,
    pub output: String,
}

pub struct RuntimeContext {
    pub service_name: String,
    pub ports: HashMap<String, u16>,
    pub env: HashMap<String, String>,
    pub working_dir: PathBuf,
}
```

---

### Orchestration: Health Check

```rust
#[async_trait]
pub trait HealthCheck: Plugin {
    /// Perform a health check
    async fn check(&self, config: &Value, ctx: &RuntimeContext) -> Result<HealthResult>;
}

#[derive(Debug)]
pub struct HealthResult {
    pub healthy: bool,
    pub message: Option<String>,
    pub response_time_ms: u64,
    pub details: HashMap<String, String>,
}
```

---

### Orchestration: Environment Provider

```rust
#[async_trait]
pub trait EnvProvider: Plugin {
    /// Load environment variables
    async fn load(&self, config: &Value) -> Result<HashMap<String, String>>;

    /// Refresh environment variables (for dynamic secrets)
    async fn refresh(&self, config: &Value) -> Result<HashMap<String, String>>;
}
```

---

### Orchestration: Proxy Middleware

```rust
#[async_trait]
pub trait ProxyMiddleware: Plugin {
    /// Process an incoming request
    async fn process_request(&self, req: ProxyRequest) -> Result<ProxyResult>;

    /// Process an outgoing response
    async fn process_response(&self, resp: ProxyResponse) -> Result<ProxyResponse>;
}

pub enum ProxyResult {
    Continue(ProxyRequest),      // Pass to next middleware
    Response(ProxyResponse),     // Short-circuit with response
}

pub struct ProxyRequest {
    pub method: HttpMethod,
    pub path: String,
    pub headers: HashMap<String, String>,
    pub body: Bytes,
}

pub struct ProxyResponse {
    pub status: StatusCode,
    pub headers: HashMap<String, String>,
    pub body: Bytes,
}
```

---

### Orchestration: Observability Sink

```rust
#[async_trait]
pub trait ObservabilitySink: Plugin {
    /// Handle an observability event
    async fn handle(&self, event: &ObservabilityEvent);

    /// Flush buffered events
    async fn flush(&self) -> Result<()>;
}

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Notice,
    Warn,
    Error,
    Fatal,
}

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
```

---

### Orchestration: Rollout Strategy

```rust
#[async_trait]
pub trait RolloutStrategy: Plugin {
    /// Plan rollout steps
    async fn plan(&self, config: &Value) -> Result<Vec<RolloutStep>>;

    /// Execute a single rollout step
    async fn execute_step(&self, step: &RolloutStep, ctx: &RolloutContext) -> Result<RolloutStepResult>;

    /// Rollback deployment
    async fn rollback(&self, ctx: &RolloutContext) -> Result<()>;
}

#[derive(Debug, Clone)]
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

#[derive(Debug)]
pub struct RolloutStepResult {
    pub success: bool,
    pub message: Option<String>,
}
```

---

## Plugin Entry Point

### v2 (FFI-safe):
```rust
#[no_mangle]
pub extern "C" fn plugin_entry() -> *const PluginVTable {
    &PLUGIN_VTABLE
}
```

### v3 (Native):
```rust
use lib_plugin_abi_v3::*;
use inventory;

pub struct MyPlugin;

impl Plugin for MyPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            id: "adi.myplugin".to_string(),
            name: "My Plugin".to_string(),
            version: "1.0.0".to_string(),
            plugin_type: PluginType::Extension,
            author: Some("Me".to_string()),
            description: Some("Does things".to_string()),
        }
    }

    async fn init(&mut self, ctx: &PluginContext) -> Result<()> {
        // Initialize plugin
        Ok(())
    }
}

#[async_trait]
impl CliCommands for MyPlugin {
    async fn list_commands(&self) -> Vec<CliCommand> {
        vec![
            CliCommand {
                name: "hello".to_string(),
                description: "Say hello".to_string(),
                usage: "myplugin hello".to_string(),
                has_subcommands: false,
            }
        ]
    }

    async fn run_command(&self, ctx: &CliContext) -> Result<CliResult> {
        Ok(CliResult::success("Hello!"))
    }
}

// Export plugin via inventory (plugin discovery)
inventory::submit! {
    PluginDescriptor {
        id: "adi.myplugin",
        constructor: || Box::new(MyPlugin),
        provides: &[
            ServiceType::CliCommands,
        ],
    }
}
```

**Alternative (simpler):**
```rust
#[no_mangle]
pub fn plugin_create() -> Box<dyn Plugin> {
    Box::new(MyPlugin)
}
```

---

## Plugin Manager

```rust
pub struct PluginManager {
    plugins: HashMap<String, Arc<dyn Plugin>>,

    // Service-specific lookups
    cli_commands: HashMap<String, Arc<dyn CliCommands>>,
    http_routes: HashMap<String, Arc<dyn HttpRoutes>>,
    mcp_tools: HashMap<String, Arc<dyn McpTools>>,
    runners: HashMap<String, Arc<dyn Runner>>,
    health_checks: HashMap<String, Arc<dyn HealthCheck>>,
    // ... etc
}

impl PluginManager {
    pub fn load_plugin(&mut self, path: &Path) -> Result<()> {
        // 1. Load dynamic library
        let lib = unsafe { Library::new(path)? };

        // 2. Get plugin_create symbol
        let create_fn: Symbol<fn() -> Box<dyn Plugin>> =
            unsafe { lib.get(b"plugin_create")? };

        // 3. Create plugin instance
        let mut plugin = create_fn();

        // 4. Initialize
        let ctx = self.create_context(&plugin.metadata())?;
        plugin.init(&ctx).await?;

        // 5. Store plugin
        let plugin_id = plugin.metadata().id.clone();
        let plugin = Arc::from(plugin);

        self.plugins.insert(plugin_id.clone(), plugin.clone());

        // 6. Register services
        if let Some(cli) = plugin.clone().downcast_arc::<dyn CliCommands>().ok() {
            self.cli_commands.insert(plugin_id.clone(), cli);
        }

        if let Some(runner) = plugin.clone().downcast_arc::<dyn Runner>().ok() {
            self.runners.insert(plugin_id.clone(), runner);
        }

        // ... etc

        Ok(())
    }

    pub fn get_cli_commands(&self, plugin_id: &str) -> Option<Arc<dyn CliCommands>> {
        self.cli_commands.get(plugin_id).cloned()
    }

    pub fn get_runner(&self, runner_type: &str) -> Option<Arc<dyn Runner>> {
        self.runners.get(runner_type).cloned()
    }
}
```

---

## Error Handling

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum PluginError {
    #[error("Plugin initialization failed: {0}")]
    InitFailed(String),

    #[error("Plugin not found: {0}")]
    NotFound(String),

    #[error("Service not provided by plugin")]
    ServiceNotProvided,

    #[error("Command execution failed: {0}")]
    CommandFailed(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

pub type Result<T> = std::result::Result<T, PluginError>;
```

---

## Migration Strategy

### Phase 1: Create v3 ABI (Week 1-2)
- Implement lib-plugin-abi-v3 crate
- All trait definitions
- Error types
- Documentation

### Phase 2: Update Plugin Host (Week 3-4)
- Support loading v3 plugins
- Maintain v2 compatibility
- Dual-mode service registry

### Phase 3: Migrate Plugins (Week 5-12)
- Start with simple plugins (translation, linter)
- Move to complex plugins (tasks, indexer)
- Finish with orchestration plugins (hive)

### Phase 4: Deprecate v2 (Month 4+)
- Remove v2 support
- Clean up dual-mode code
- Update documentation

---

## Benefits

1. **Simpler Development**
   - No FFI complexity
   - Familiar Rust patterns
   - Better IDE support
   - Clear error messages

2. **Better Performance**
   - No FFI overhead
   - Direct function calls
   - Zero-cost abstractions

3. **Unified Architecture**
   - Single ABI for all plugins
   - Consistent patterns
   - Easier maintenance

4. **Type Safety**
   - Compile-time checks
   - No runtime JSON parsing
   - Better refactoring support

---

## Trade-offs

### Accepted:
- ✅ Plugins must use same Rust version as host
- ✅ Plugins must recompile on ABI changes
- ✅ No cross-language plugins (Rust only)

### Mitigated:
- Registry auto-rebuilds on Rust updates
- Clear versioning and migration guides
- WASM option for third-party plugins (future)

---

## Next Steps

1. Implement lib-plugin-abi-v3 crate
2. Create example plugin using v3
3. Update lib-plugin-host for v3 loading
4. Write migration guide
5. Migrate first plugin (adi-cli-lang-en)
6. Measure performance and developer experience
7. Iterate on design based on feedback

---

**Status:** Ready for implementation
