# ADI Plugin System: Complete Architecture Overview

**Scope:** Entire ADI CLI Ecosystem
**Date:** 2026-01-31
**Version:** 1.0

---

## Executive Summary

ADI uses a **dual-ABI plugin architecture** supporting 86+ plugins across two domains:

1. **General Plugin ABI** (`lib-plugin-abi`) - CLI commands, HTTP routes, MCP tools/resources/prompts
2. **Orchestration ABI** (`lib-plugin-abi-orchestration`) - Hive container orchestration (Runner, Health, Proxy, etc.)

**Key Features:**
- FFI-safe plugin interface via `abi_stable` (stable across Rust versions)
- Dynamic loading via shared libraries (.dylib/.so/.dll)
- Service registry for inter-plugin communication
- Version-aware dependency management
- HTTP-based plugin registry for distribution
- Translation plugin system (9 languages)
- 86+ production plugins

---

## Table of Contents

1. [Architecture Overview](#1-architecture-overview)
2. [General Plugin ABI (lib-plugin-abi)](#2-general-plugin-abi)
3. [Plugin Service Types](#3-plugin-service-types)
4. [Service Registry](#4-service-registry)
5. [Plugin Manifest Format](#5-plugin-manifest-format)
6. [Plugin Loading & Hosting](#6-plugin-loading--hosting)
7. [Orchestration Plugin ABI](#7-orchestration-plugin-abi)
8. [Translation Plugin System](#8-translation-plugin-system)
9. [Plugin Registry Service](#9-plugin-registry-service)
10. [Complete Plugin Inventory](#10-complete-plugin-inventory)
11. [Plugin Development Guide](#11-plugin-development-guide)
12. [Deployment & Distribution](#12-deployment--distribution)
13. [Recommendations](#13-recommendations)

---

## 1. Architecture Overview

### 1.1 Dual-ABI Design Philosophy

ADI separates plugin concerns into two distinct ABIs:

```
┌─────────────────────────────────────────────────────────────┐
│                     ADI Plugin Ecosystem                     │
├─────────────────────────────────────────────────────────────┤
│                                                               │
│  ┌───────────────────────────┐  ┌───────────────────────┐  │
│  │   lib-plugin-abi          │  │ lib-plugin-abi-       │  │
│  │   (General Purpose)       │  │ orchestration         │  │
│  ├───────────────────────────┤  ├───────────────────────┤  │
│  │ - CLI Commands            │  │ - Runner Plugins      │  │
│  │ - HTTP Routes             │  │ - Health Checks       │  │
│  │ - MCP Tools/Resources     │  │ - Environment Loaders │  │
│  │ - Service Registry        │  │ - Proxy Middleware    │  │
│  │ - FFI-Safe (abi_stable)   │  │ - Observability       │  │
│  │ - API Version: 2          │  │ - Rollout Strategies  │  │
│  │ - 75+ plugins             │  │ - Async Traits        │  │
│  └───────────────────────────┘  │ - 32+ plugins         │  │
│                                  └───────────────────────┘  │
│                                                               │
│  ┌───────────────────────────────────────────────────────┐  │
│  │           Translation Plugin System (i18n)            │  │
│  │  - Service Pattern: adi.i18n.<namespace>.<language>  │  │
│  │  - 9 language plugins (en, zh-CN, uk-UA, es, fr...)  │  │
│  │  - Mozilla Fluent (.ftl) message format              │  │
│  └───────────────────────────────────────────────────────┘  │
│                                                               │
└─────────────────────────────────────────────────────────────┘
```

**Why Two ABIs?**
- **General ABI**: Broad extensibility (any command, HTTP endpoint, MCP tool)
- **Orchestration ABI**: Domain-specific traits for container orchestration
- **Separation of Concerns**: Different update cycles, versioning, stability guarantees

---

### 1.2 Plugin Lifecycle

```
┌──────────────────────────────────────────────────────────────┐
│                     Plugin Lifecycle                         │
└──────────────────────────────────────────────────────────────┘

1. DISCOVER
   └─> Scan ~/.local/share/adi/plugins/ for plugin.toml files

2. INSTALL (if not present)
   ├─> Query registry: https://adi-plugin-registry.the-ihor.com
   ├─> Download binary (.tar.gz)
   ├─> Verify SHA2-256 checksum
   └─> Extract to plugin directory

3. ENABLE
   └─> Mark as enabled in user config (~/.config/adi/config.toml)

4. LOAD
   ├─> Use libloading to open shared library
   ├─> Resolve platform-specific binary (libplugin.dylib/.so/.dll)
   └─> Call plugin_entry() → returns *const PluginVTable

5. INIT
   ├─> Create PluginContext with HostVTable
   ├─> Call plugin.init(context) → plugin registers services
   └─> Add to service registry

6. OPERATE
   ├─> CLI: Dispatch commands to adi.cli.commands service
   ├─> HTTP: Route requests to adi.http.routes service
   └─> MCP: Invoke tools via adi.mcp.tools service

7. UPDATE (optional)
   └─> Periodic plugin.update() calls for tick-based logic

8. CLEANUP
   ├─> Call plugin.cleanup(context)
   └─> Unregister services from registry

9. UNLOAD
   └─> Release library handle, free memory
```

---

## 2. General Plugin ABI (lib-plugin-abi)

**Location:** `/crates/lib/lib-plugin-abi/`

### 2.1 Core Interface: PluginVTable

**File:** `src/vtable.rs`

All plugins must export a `plugin_entry()` function returning a pointer to this vtable:

```rust
#[repr(C)]
pub struct PluginVTable {
    /// Returns plugin metadata (id, name, version, type)
    pub info: extern "C" fn() -> PluginInfo,

    /// Initialize plugin, register services (return 0 on success)
    pub init: extern "C" fn(ctx: *mut PluginContext) -> i32,

    /// Optional: Periodic update/tick callback
    pub update: ROption<extern "C" fn(ctx: *mut PluginContext) -> i32>,

    /// Cleanup before unload
    pub cleanup: extern "C" fn(ctx: *mut PluginContext),

    /// Optional: Custom message handling
    pub handle_message: ROption<extern "C" fn(
        ctx: *mut PluginContext,
        message_type: RStr<'_>,
        data: RSlice<'_, u8>,
    ) -> RResult<RVec<u8>, RString>>,
}
```

**Entry Point:**
```rust
#[no_mangle]
pub extern "C" fn plugin_entry() -> *const PluginVTable {
    &PLUGIN_VTABLE
}
```

---

### 2.2 PluginInfo (Metadata)

**File:** `src/types.rs`

```rust
#[repr(C)]
pub struct PluginInfo {
    pub id: RString,              // e.g., "adi.tasks"
    pub name: RString,            // "ADI Tasks"
    pub version: RString,         // "0.8.8"
    pub plugin_type: RString,     // "core", "extension", "theme", "font"
    pub author: ROption<RString>, // Optional author
    pub description: ROption<RString>,
}
```

**Plugin Types:**
- `core` - Essential functionality (tasks, indexer, knowledgebase)
- `extension` - Optional enhancements (llm-uzu, browser-debug)
- `theme` - UI themes
- `font` - Custom fonts

---

### 2.3 HostVTable (Host Capabilities)

Plugins receive a `PluginContext` containing a `HostVTable` with these capabilities:

```rust
#[repr(C)]
pub struct HostVTable {
    // === LOGGING ===
    pub log: extern "C" fn(level: u8, message: RStr<'_>),

    // === CONFIGURATION ===
    pub config_get: extern "C" fn(key: RStr<'_>) -> ROption<RString>,
    pub config_set: extern "C" fn(key: RStr<'_>, value: RStr<'_>) -> i32,

    // === FILE SYSTEM ===
    pub data_dir: extern "C" fn() -> RString,  // ~/.local/share/adi/

    // === UI ===
    pub toast: ROption<extern "C" fn(level: u8, message: RStr<'_>)>,

    // === ACTIONS ===
    pub host_action: ROption<extern "C" fn(
        action: RStr<'_>,
        data: RSlice<'_, u8>,
    ) -> RResult<RVec<u8>, RString>>,

    // === SERVICE REGISTRY V2 ===
    pub register_service: extern "C" fn(
        desc: ServiceDescriptor,
        handle: ServiceHandle,
    ) -> i32,

    pub lookup_service: extern "C" fn(
        service_id: RStr<'_>,
    ) -> ROption<ServiceHandle>,

    pub lookup_service_versioned: extern "C" fn(
        service_id: RStr<'_>,
        min_version: ServiceVersion,
    ) -> ROption<ServiceHandle>,

    pub list_services: extern "C" fn() -> RVec<ServiceDescriptor>,
}
```

**Usage Example:**
```rust
fn plugin_init(ctx: *mut PluginContext) -> i32 {
    unsafe {
        let host = (*ctx).host();

        // Log initialization
        (host.log)(2, "Initializing plugin".into());

        // Get config value
        if let Some(api_key) = (host.config_get)("api_key".into()).into_option() {
            // Use API key
        }

        // Register service
        let service_desc = ServiceDescriptor {
            id: "adi.myplugin.service".into(),
            version: ServiceVersion::new(1, 0, 0),
            description: "My custom service".into(),
        };
        (host.register_service)(service_desc, service_handle);
    }

    0 // success
}
```

---

### 2.4 FFI-Safe Types

All types crossing the FFI boundary use `abi_stable` crate:

| Rust Type | FFI Type | Purpose |
|-----------|----------|---------|
| `String` | `RString` | Owned string |
| `&str` | `RStr<'_>` | Borrowed string slice |
| `Vec<T>` | `RVec<T>` | Owned vector |
| `&[T]` | `RSlice<'_, T>` | Borrowed slice |
| `Option<T>` | `ROption<T>` | Optional value |
| `Result<T, E>` | `RResult<T, E>` | Result type |
| `HashMap<K, V>` | `RHashMap<K, V>` | Hash map |

**Why abi_stable?**
- Stable ABI across Rust compiler versions
- No recompilation needed when host updates
- Plugins can use different Rust versions than host

---

## 3. Plugin Service Types

Plugins provide functionality by implementing **service vtables**. Each service type has a dedicated interface.

### 3.1 CLI Commands Service

**Service ID:** `adi.cli.commands`
**File:** `src/cli.rs`

**Purpose:** Extend CLI with custom commands

**VTable:**
```rust
#[repr(C)]
pub struct CliCommandsVTable {
    /// List available commands
    pub list_commands: extern "C" fn(
        handle: ServiceHandle,
    ) -> RVec<CliCommand>,

    /// Execute a command
    pub run_command: extern "C" fn(
        handle: ServiceHandle,
        context: CliContext,
    ) -> RResult<CliResult, RString>,
}
```

**Types:**
```rust
#[repr(C)]
pub struct CliCommand {
    pub name: RString,           // "list", "create", "delete"
    pub description: RString,    // "List all tasks"
    pub usage: RString,          // "tasks list [--filter <filter>]"
    pub has_subcommands: bool,
}

#[repr(C)]
pub struct CliContext {
    pub command: RString,        // "tasks"
    pub subcommand: ROption<RString>,  // "list"
    pub args: RVec<RString>,     // ["--filter", "open"]
    pub options: RString,        // JSON: {"filter": "open"}
    pub cwd: RString,            // Current working directory
}

#[repr(C)]
pub struct CliResult {
    pub exit_code: i32,
    pub stdout: RString,
    pub stderr: RString,
}
```

**Example Plugin:**
```rust
// adi-tasks-plugin provides: adi tasks list, adi tasks create, etc.

extern "C" fn list_commands(_handle: ServiceHandle) -> RVec<CliCommand> {
    vec![
        CliCommand {
            name: "list".into(),
            description: "List all tasks".into(),
            usage: "tasks list [--status <status>]".into(),
            has_subcommands: false,
        },
        CliCommand {
            name: "create".into(),
            description: "Create a new task".into(),
            usage: "tasks create <title>".into(),
            has_subcommands: false,
        },
    ].into()
}

extern "C" fn run_command(
    _handle: ServiceHandle,
    context: CliContext,
) -> RResult<CliResult, RString> {
    match context.subcommand.as_ref().map(|s| s.as_str()) {
        Some("list") => {
            // Execute task listing logic
            RResult::ROk(CliResult {
                exit_code: 0,
                stdout: "Task 1\nTask 2\n".into(),
                stderr: "".into(),
            })
        }
        Some("create") => {
            // Execute task creation logic
            // ...
        }
        _ => RResult::RErr("Unknown command".into()),
    }
}
```

**CLI Integration:**
```bash
adi tasks list          # Dispatched to adi.tasks plugin
adi indexer search foo  # Dispatched to adi.indexer plugin
adi workflow run dev    # Dispatched to adi.workflow plugin
```

---

### 3.2 HTTP Routes Service

**Service ID:** `adi.http.routes`
**File:** `src/http.rs`

**Purpose:** Expose HTTP API endpoints

**VTable:**
```rust
#[repr(C)]
pub struct HttpRoutesVTable {
    /// List available routes
    pub list_routes: extern "C" fn(
        handle: ServiceHandle,
    ) -> RVec<HttpRoute>,

    /// Handle HTTP request
    pub handle_request: extern "C" fn(
        handle: ServiceHandle,
        handler_id: RString,
        request: HttpRequest,
    ) -> RResult<HttpResponse, RString>,
}
```

**Types:**
```rust
#[repr(C)]
pub struct HttpRoute {
    pub method: HttpMethod,      // GET, POST, PUT, DELETE, etc.
    pub path: RString,           // "/api/tasks"
    pub handler_id: RString,     // Internal handler identifier
    pub description: RString,    // "List all tasks"
}

#[repr(C)]
pub enum HttpMethod {
    Get = 0,
    Post = 1,
    Put = 2,
    Delete = 3,
    Patch = 4,
    Head = 5,
    Options = 6,
    Trace = 7,
}

#[repr(C)]
pub struct HttpRequest {
    pub method: HttpMethod,
    pub path: RString,
    pub query: RString,          // "?filter=open&sort=date"
    pub headers: RString,        // JSON: {"content-type": "application/json"}
    pub body: RVec<u8>,          // Raw body bytes
    pub params: RString,         // JSON: {"id": "123"} (path params)
}

#[repr(C)]
pub struct HttpResponse {
    pub status_code: u16,
    pub headers: RString,        // JSON
    pub body: RVec<u8>,
}
```

**Example:**
```rust
// adi-tasks-plugin exposes: GET /api/tasks, POST /api/tasks

extern "C" fn list_routes(_handle: ServiceHandle) -> RVec<HttpRoute> {
    vec![
        HttpRoute {
            method: HttpMethod::Get,
            path: "/api/tasks".into(),
            handler_id: "list_tasks".into(),
            description: "List all tasks".into(),
        },
        HttpRoute {
            method: HttpMethod::Post,
            path: "/api/tasks".into(),
            handler_id: "create_task".into(),
            description: "Create a new task".into(),
        },
    ].into()
}

extern "C" fn handle_request(
    _handle: ServiceHandle,
    handler_id: RString,
    request: HttpRequest,
) -> RResult<HttpResponse, RString> {
    match handler_id.as_str() {
        "list_tasks" => {
            let tasks_json = r#"[{"id": 1, "title": "Task 1"}]"#;
            RResult::ROk(HttpResponse {
                status_code: 200,
                headers: r#"{"content-type": "application/json"}"#.into(),
                body: tasks_json.as_bytes().to_vec().into(),
            })
        }
        _ => RResult::RErr("Unknown handler".into()),
    }
}
```

---

### 3.3 MCP Tools Service

**Service ID:** `adi.mcp.tools`
**File:** `src/mcp.rs`

**Purpose:** Model Context Protocol tools for LLMs

**VTable:**
```rust
#[repr(C)]
pub struct McpToolsVTable {
    pub list_tools: extern "C" fn(
        handle: ServiceHandle,
    ) -> RVec<McpTool>,

    pub call_tool: extern "C" fn(
        handle: ServiceHandle,
        name: RString,
        arguments_json: RString,
    ) -> RResult<McpToolResult, RString>,
}
```

**Types:**
```rust
#[repr(C)]
pub struct McpTool {
    pub name: RString,               // "search_code"
    pub description: RString,        // "Search codebase semantically"
    pub input_schema: RString,       // JSON Schema for arguments
}

#[repr(C)]
pub struct McpToolResult {
    pub content: RString,            // Tool output
    pub content_type: RString,       // "text", "json", "error"
    pub is_error: bool,
}
```

**Example:**
```rust
// adi-indexer provides: search_code, get_definition, find_references

extern "C" fn list_tools(_handle: ServiceHandle) -> RVec<McpTool> {
    vec![
        McpTool {
            name: "search_code".into(),
            description: "Semantic code search".into(),
            input_schema: r#"{
                "type": "object",
                "properties": {
                    "query": {"type": "string"},
                    "language": {"type": "string"}
                },
                "required": ["query"]
            }"#.into(),
        },
    ].into()
}

extern "C" fn call_tool(
    _handle: ServiceHandle,
    name: RString,
    arguments_json: RString,
) -> RResult<McpToolResult, RString> {
    match name.as_str() {
        "search_code" => {
            // Parse arguments, execute search
            let results = r#"[{"file": "main.rs", "line": 42}]"#;
            RResult::ROk(McpToolResult {
                content: results.into(),
                content_type: "json".into(),
                is_error: false,
            })
        }
        _ => RResult::RErr("Unknown tool".into()),
    }
}
```

---

### 3.4 MCP Resources Service

**Service ID:** `adi.mcp.resources`
**File:** `src/mcp.rs`

**Purpose:** Provide resources (files, data) to LLMs

**VTable:**
```rust
#[repr(C)]
pub struct McpResourcesVTable {
    pub list_resources: extern "C" fn(
        handle: ServiceHandle,
    ) -> RVec<McpResource>,

    pub read_resource: extern "C" fn(
        handle: ServiceHandle,
        uri: RString,
    ) -> RResult<McpResourceContent, RString>,
}
```

**Types:**
```rust
#[repr(C)]
pub struct McpResource {
    pub uri: RString,            // "file:///project/src/main.rs"
    pub name: RString,           // "main.rs"
    pub description: RString,    // "Main entry point"
    pub mime_type: RString,      // "text/x-rust"
}

#[repr(C)]
pub struct McpResourceContent {
    pub uri: RString,
    pub content: RString,        // Text or base64-encoded binary
    pub mime_type: RString,
    pub is_binary: bool,         // If true, content is base64
}
```

---

### 3.5 MCP Prompts Service

**Service ID:** `adi.mcp.prompts`
**File:** `src/mcp.rs`

**Purpose:** Provide prompt templates for LLMs

**VTable:**
```rust
#[repr(C)]
pub struct McpPromptsVTable {
    pub list_prompts: extern "C" fn(
        handle: ServiceHandle,
    ) -> RVec<McpPrompt>,

    pub get_prompt: extern "C" fn(
        handle: ServiceHandle,
        name: RString,
        arguments_json: RString,
    ) -> RResult<RVec<McpPromptMessage>, RString>,
}
```

**Types:**
```rust
#[repr(C)]
pub struct McpPrompt {
    pub name: RString,               // "code_review"
    pub description: RString,        // "Review code for issues"
    pub arguments_schema: RString,   // JSON Schema
}

#[repr(C)]
pub struct McpPromptMessage {
    pub role: RString,               // "user", "assistant", "system"
    pub content: RString,
}
```

---

## 4. Service Registry

**File:** `src/service.rs`

### 4.1 Service Discovery Pattern

Plugins communicate via a **service registry** managed by the host.

**Service ID Naming:**
```
adi.<namespace>.<service>[.<subservice>]

Examples:
  adi.cli.commands              (Core CLI service)
  adi.http.routes               (Core HTTP service)
  adi.mcp.tools                 (MCP tools service)
  adi.i18n.cli.en-US            (Translation service)
  adi.tasks.api                 (Custom service)
  adi.llm.inference             (LLM inference)
```

---

### 4.2 Service Types

```rust
#[repr(C)]
pub struct ServiceDescriptor {
    pub id: ServiceId,           // e.g., "adi.cli.commands"
    pub version: ServiceVersion, // Semantic version
    pub description: RString,
}

#[repr(C)]
pub struct ServiceId {
    pub namespace: RString,      // "adi"
    pub component: RString,      // "cli"
    pub name: RString,           // "commands"
}

#[repr(C)]
pub struct ServiceVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

#[repr(C)]
pub struct ServiceHandle {
    pub service_id: ServiceId,
    pub ptr: *const c_void,          // Opaque plugin state
    pub vtable: *const ServiceVTable, // Method dispatch
}
```

---

### 4.3 Version Compatibility

**Rule:** `provided.major == required.major && provided.minor >= required.minor`

**Example:**
- Plugin requires: `adi.cli.commands` v1.2.0
- Host provides: `adi.cli.commands` v1.5.3 ✅ Compatible
- Host provides: `adi.cli.commands` v2.0.0 ❌ Incompatible (major mismatch)
- Host provides: `adi.cli.commands` v1.1.0 ❌ Incompatible (minor too old)

---

### 4.4 Generic Service Invocation

For custom services, use generic invocation:

```rust
#[repr(C)]
pub struct ServiceVTable {
    /// Invoke method by name with JSON arguments
    pub invoke: extern "C" fn(
        handle: ServiceHandle,
        method_name: RStr<'_>,
        arguments_json: RStr<'_>,
    ) -> RResult<RString, RString>,

    /// List available methods
    pub list_methods: extern "C" fn(
        handle: ServiceHandle,
    ) -> RVec<ServiceMethod>,
}

#[repr(C)]
pub struct ServiceMethod {
    pub name: RString,
    pub description: RString,
    pub parameters_schema: RString,  // JSON Schema
    pub returns_schema: RString,     // JSON Schema
}
```

**Usage:**
```rust
// Plugin A registers custom service
fn init(ctx: *mut PluginContext) -> i32 {
    let service_desc = ServiceDescriptor {
        id: "adi.myplugin.api".into(),
        version: ServiceVersion { major: 1, minor: 0, patch: 0 },
        description: "My plugin API".into(),
    };

    let service_vtable = ServiceVTable {
        invoke: my_invoke,
        list_methods: my_list_methods,
    };

    let service_handle = ServiceHandle {
        service_id: service_desc.id.clone(),
        ptr: std::ptr::null(),
        vtable: &service_vtable,
    };

    unsafe {
        ((*ctx).host().register_service)(service_desc, service_handle);
    }
    0
}

// Plugin B calls Plugin A's service
fn use_service(ctx: *mut PluginContext) {
    unsafe {
        let host = (*ctx).host();
        let handle = (host.lookup_service)("adi.myplugin.api".into());

        if let Some(svc) = handle.into_option() {
            let result = (svc.vtable.invoke)(
                svc,
                "get_data".into(),
                r#"{"id": 123}"#.into(),
            );
        }
    }
}
```

---

## 5. Plugin Manifest Format

**File:** `plugin.toml`
**Spec:** `/crates/lib/lib-plugin-manifest/`

### 5.1 Single Plugin Manifest

```toml
[plugin]
id = "adi.tasks"                  # Unique identifier (reverse domain)
name = "ADI Tasks"                # Display name
version = "0.8.8"                 # Semantic version
type = "core"                     # "core", "extension", "theme", "font"
author = "ADI Team"               # Optional
description = "Task management and execution"
min_host_version = "0.8.0"        # Minimum adi-cli version

[compatibility]
api_version = 2                   # Plugin ABI version
depends_on = ["adi.embed"]        # Plugin dependencies

[[requires]]
id = "adi.embed"                  # Required plugin
version = "0.1.0"                 # Minimum version

[[provides]]
id = "adi.cli.commands"           # Services this plugin provides
version = "1.0.0"

[[provides]]
id = "adi.http.routes"
version = "1.0.0"

[[provides]]
id = "adi.mcp.tools"
version = "1.0.0"

[cli]                             # Optional CLI metadata
command = "tasks"                 # Command name
description = "Manage tasks"
aliases = ["t", "todo"]           # Alternative names

[tags]
categories = ["productivity", "task-management"]
keywords = ["tasks", "todo", "gtd"]

[binary]
name = "plugin"                   # Binary filename (no extension)
# Platform-specific names auto-detected:
#   macOS: libplugin.dylib
#   Linux: libplugin.so
#   Windows: plugin.dll
```

---

### 5.2 Multi-Plugin Package

**File:** `package.toml`

```toml
[package]
id = "vendor.theme-pack"
name = "Theme Collection"
version = "2.0.0"
description = "Multiple themes"

[[plugins]]
id = "vendor.theme-dark"
name = "Dark Theme"
version = "2.0.0"
type = "theme"
binary = "dark_theme"

[[plugins]]
id = "vendor.theme-light"
name = "Light Theme"
version = "2.0.0"
type = "theme"
binary = "light_theme"
```

---

## 6. Plugin Loading & Hosting

**Location:** `/crates/lib/lib-plugin-host/`

### 6.1 PluginHost

**File:** `src/host.rs`

**Responsibilities:**
- Plugin discovery (scan directories)
- Plugin installation (download from registry)
- Plugin loading (dynamic library loading)
- Service registry management
- Plugin lifecycle (init, update, cleanup, unload)

```rust
pub struct PluginHost {
    config: PluginHostConfig,
    installed_plugins: HashMap<String, InstalledPlugin>,
    loaded_plugins: HashMap<String, LoadedPlugin>,
    service_registry: Arc<RwLock<ServiceRegistry>>,
    callbacks: Arc<CallbackBridge>,
}

impl PluginHost {
    pub fn new(config: PluginHostConfig) -> Self { /* ... */ }

    pub fn scan_plugins(&mut self) -> Result<Vec<PluginManifest>> { /* ... */ }

    pub fn install_plugin(&mut self, plugin_id: &str) -> Result<()> { /* ... */ }

    pub fn enable_plugin(&mut self, plugin_id: &str) -> Result<()> { /* ... */ }

    pub fn load_plugin(&mut self, plugin_id: &str) -> Result<()> { /* ... */ }

    pub fn unload_plugin(&mut self, plugin_id: &str) -> Result<()> { /* ... */ }

    pub fn get_service(&self, service_id: &str) -> Option<ServiceHandle> { /* ... */ }
}
```

---

### 6.2 PluginLoader

**File:** `src/loader.rs`

**Responsibilities:**
- Load shared library via `libloading`
- Call `plugin_entry()` to get PluginVTable
- Create PluginContext with HostVTable
- Call plugin init/cleanup

```rust
pub struct LoadedPlugin {
    pub manifest: PluginManifest,
    pub library: Library,              // libloading::Library
    pub vtable: &'static PluginVTable,
    pub context: Box<PluginContext>,
}

impl LoadedPlugin {
    pub fn load(manifest: PluginManifest, host: &HostVTable) -> Result<Self> {
        // Resolve binary path
        let lib_path = resolve_plugin_binary(&manifest)?;

        // Load library
        let library = unsafe { Library::new(&lib_path)? };

        // Get plugin_entry symbol
        let entry: Symbol<extern "C" fn() -> *const PluginVTable> =
            unsafe { library.get(b"plugin_entry")? };

        let vtable_ptr = entry();
        let vtable = unsafe { &*vtable_ptr };

        // Create context
        let mut context = Box::new(PluginContext::new(host));

        // Initialize plugin
        let result = (vtable.init)(context.as_mut() as *mut _);
        if result != 0 {
            return Err(anyhow!("Plugin init failed: {}", result));
        }

        Ok(Self {
            manifest,
            library,
            vtable,
            context,
        })
    }

    pub fn unload(mut self) -> Result<()> {
        // Call cleanup
        (self.vtable.cleanup)(self.context.as_mut() as *mut _);

        // Drop library (automatic unload)
        drop(self.library);

        Ok(())
    }
}
```

---

### 6.3 ServiceRegistry

**File:** `src/service_registry.rs`

**Thread-safe service registry:**

```rust
pub struct ServiceRegistry {
    services: HashMap<String, RegisteredService>,
}

struct RegisteredService {
    descriptor: ServiceDescriptor,
    handle: ServiceHandle,
    provider_plugin_id: String,
}

impl ServiceRegistry {
    pub fn register(&mut self, desc: ServiceDescriptor, handle: ServiceHandle, plugin_id: String) {
        self.services.insert(desc.id.to_string(), RegisteredService {
            descriptor: desc,
            handle,
            provider_plugin_id: plugin_id,
        });
    }

    pub fn lookup(&self, service_id: &str) -> Option<&ServiceHandle> {
        self.services.get(service_id).map(|s| &s.handle)
    }

    pub fn lookup_versioned(&self, service_id: &str, min_version: ServiceVersion) -> Option<&ServiceHandle> {
        self.services.get(service_id).and_then(|s| {
            if is_compatible(&s.descriptor.version, &min_version) {
                Some(&s.handle)
            } else {
                None
            }
        })
    }

    pub fn list_services(&self) -> Vec<ServiceDescriptor> {
        self.services.values().map(|s| s.descriptor.clone()).collect()
    }
}

fn is_compatible(provided: &ServiceVersion, required: &ServiceVersion) -> bool {
    provided.major == required.major && provided.minor >= required.minor
}
```

---

### 6.4 Plugin Directory Structure

```
~/.local/share/adi/plugins/
├── adi.tasks/
│   ├── plugin.toml                   # Manifest
│   ├── libadi_tasks_plugin.dylib     # macOS binary
│   ├── libadi_tasks_plugin.so        # Linux binary
│   └── adi_tasks_plugin.dll          # Windows binary
│
├── adi.indexer/
│   ├── plugin.toml
│   └── libadi_indexer_plugin.dylib
│
├── adi.workflow/
│   ├── plugin.toml
│   └── libadi_workflow_plugin.dylib
│
└── adi-cli-lang-zh-CN/
    ├── plugin.toml
    └── libadi_cli_lang_zh_cn.dylib
```

---

## 7. Orchestration Plugin ABI

**Location:** `/crates/lib/lib-plugin-abi-orchestration/`

### 7.1 Design Philosophy

Unlike general plugins (FFI-safe), orchestration plugins use **native Rust async traits** for:
- Type-safe async/await
- Zero-cost abstractions
- Tokio integration
- Simpler implementation

**Trade-off:** Not stable across Rust versions (requires same compiler version as Hive)

---

### 7.2 Plugin Categories

#### Runner Plugins (`hive.runner.*`)

**File:** `src/runner.rs`

**Purpose:** Execute services (Docker, scripts, processes)

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

**Examples:**
- `hive.runner.docker` - Docker containers
- `hive.runner.script` - Shell scripts
- `hive.runner.compose` - Docker Compose
- `hive.runner.podman` - Podman containers

---

#### Health Plugins (`hive.health.*`)

**File:** `src/health.rs`

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

**Examples:**
- `hive.health.http` - HTTP GET checks
- `hive.health.tcp` - TCP connectivity
- `hive.health.grpc` - gRPC health protocol
- `hive.health.mysql` - MySQL connection

---

#### Environment Plugins (`hive.env.*`)

**File:** `src/env.rs`

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

**Examples:**
- `hive.env.dotenv` - .env files
- `hive.env.vault` - HashiCorp Vault
- `hive.env.1password` - 1Password CLI
- `hive.env.aws-secrets` - AWS Secrets Manager

---

#### Proxy Middleware Plugins (`hive.proxy.*`)

**File:** `src/proxy.rs`

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
    Continue(ProxyRequest),
    Response(Response<Body>),
}
```

**Examples:**
- `hive.proxy.cors` - CORS headers
- `hive.proxy.rate-limit` - Rate limiting
- `hive.proxy.auth-jwt` - JWT validation
- `hive.proxy.ip-filter` - IP filtering

---

#### Observability Plugins (`hive.obs.*`)

**File:** `src/obs.rs`

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
    Log { timestamp, level, source, message, fields },
    ServiceEvent { timestamp, service_name, event_type, details },
    HealthCheck { timestamp, service_name, result },
    Metric { timestamp, name, value, labels },
}
```

**Examples:**
- `hive.obs.stdout` - Console output
- `hive.obs.file` - File logging
- `hive.obs.loki` - Grafana Loki
- `hive.obs.prometheus` - Prometheus metrics

---

#### Rollout Strategy Plugins (`hive.rollout.*`)

**File:** `src/rollout.rs`

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
}
```

**Examples:**
- `hive.rollout.recreate` - Stop-then-start
- `hive.rollout.blue-green` - Dual environment
- `hive.rollout.canary` - Gradual traffic shift

---

### 7.3 Common Types

```rust
pub struct PluginMetadata {
    pub id: String,              // "hive.runner.docker"
    pub name: String,            // "Docker Runner"
    pub version: String,         // "1.0.0"
    pub description: Option<String>,
    pub author: Option<String>,
}

pub struct RuntimeContext {
    pub service_name: String,
    pub ports: HashMap<String, u16>,
    pub env: HashMap<String, String>,
    pub working_dir: PathBuf,
}

pub struct ProcessHandle {
    pub id: String,              // Container ID, PID, etc.
    pub runner_type: String,     // "docker", "script", etc.
    pub metadata: HashMap<String, String>,
}
```

---

## 8. Translation Plugin System

**Location:** `/crates/lib/lib-i18n-core/`

### 8.1 Architecture

Translation plugins use **service registry pattern**:

```
Service ID Pattern: adi.i18n.<namespace>.<language-code>

Examples:
  adi.i18n.cli.en-US         (English CLI translations)
  adi.i18n.cli.zh-CN         (Simplified Chinese CLI)
  adi.i18n.tasks.en-US       (English Task messages)
  adi.i18n.tasks.uk-UA       (Ukrainian Task messages)
```

---

### 8.2 Translation Service Interface

Each translation plugin registers a service with two methods:

```rust
// Generic service invocation
pub trait TranslationService {
    fn get_messages(&self) -> String;           // Returns .ftl file content
    fn get_metadata(&self) -> TranslationMeta;  // Returns metadata JSON
}

pub struct TranslationMeta {
    pub plugin_id: String,       // "adi-cli-lang-en"
    pub language: String,        // "en-US"
    pub language_name: String,   // "English (United States)"
    pub namespace: String,       // "cli"
    pub version: String,         // "1.0.0"
}
```

---

### 8.3 Mozilla Fluent Format

Translation files use **Mozilla Fluent** (.ftl) syntax:

```fluent
# Self-update messages
self-update-checking = Checking for updates...
self-update-found = Update available: {$version}
self-update-downloading = Downloading update...
self-update-installing = Installing update...
self-update-success = Successfully updated to {$version}
self-update-failed = Update failed: {$error}
self-update-already-latest = Already using latest version ({$version})

# Shell completions
completions-generating = Generating {$shell} completions...
completions-success = Completions installed to {$path}
completions-init-bash = Run: echo 'eval "$(adi completions bash)"' >> ~/.bashrc
completions-init-zsh = Run: echo 'eval "$(adi completions zsh)"' >> ~/.zshrc

# Plugin management
plugin-installing = Installing plugin: {$id}
plugin-installed = Plugin {$id} installed successfully
plugin-install-failed = Failed to install plugin {$id}: {$error}
plugin-uninstalling = Uninstalling plugin: {$id}
plugin-not-found = Plugin {$id} not found
```

**Variable Interpolation:**
```rust
t!("self-update-found", "version" => "1.2.3")
// Output: "Update available: 1.2.3"
```

---

### 8.4 Discovery & Fallback

**Discovery Process:**
1. Scan service registry for `adi.i18n.<namespace>.*` services
2. Load all discovered language plugins
3. Parse .ftl files using Fluent bundle
4. Store in memory (keyed by language code)

**Fallback Chain:**
```
User's chosen language (e.g., zh-CN)
  ↓ (if message not found)
English (en-US)
  ↓ (if still not found)
Message key (raw string)
```

**Example:**
```rust
use lib_i18n_core::{init_global, t};

// Discovery happens during init
let i18n = I18n::new(service_registry);
i18n.discover_translations()?;
i18n.set_language("zh-CN")?;
init_global(i18n);

// Usage anywhere in codebase
println!("{}", t!("self-update-checking"));
// If Chinese translation exists: "正在检查更新..."
// Otherwise falls back to English: "Checking for updates..."
```

---

### 8.5 Available Language Plugins

| Plugin ID | Language | Locale | Status |
|-----------|----------|--------|--------|
| `adi-cli-lang-en` | English | en-US | ✅ Complete |
| `adi-cli-lang-zh-CN` | Simplified Chinese | zh-CN | ✅ Complete |
| `adi-cli-lang-uk-UA` | Ukrainian | uk-UA | ✅ Complete |
| `adi-cli-lang-es-ES` | Spanish | es-ES | ✅ Complete |
| `adi-cli-lang-fr-FR` | French | fr-FR | ✅ Complete |
| `adi-cli-lang-de-DE` | German | de-DE | ✅ Complete |
| `adi-cli-lang-ja-JP` | Japanese | ja-JP | ✅ Complete |
| `adi-cli-lang-ko-KR` | Korean | ko-KR | ✅ Complete |
| `adi-cli-lang-ru-RU` | Russian | ru-RU | ✅ Complete |

**Total:** 9 language plugins

---

## 9. Plugin Registry Service

**Location:** `/crates/adi-plugin-registry-http/`

### 9.1 HTTP API

**Base URL:** `https://adi-plugin-registry.the-ihor.com`

**Endpoints:**
```
GET  /api/plugins                  # List all plugins
GET  /api/plugins/{id}             # Get plugin metadata
GET  /api/plugins/{id}/download    # Download plugin binary
GET  /api/search?q={query}         # Search plugins
POST /api/publish                  # Publish new plugin (authenticated)
GET  /api/registry/status          # Registry health
```

---

### 9.2 Plugin Distribution Format

**Package Structure:**
```
plugin.tar.gz
├── plugin.toml                    # Manifest
├── libplugin.dylib                # macOS binary
├── libplugin.so                   # Linux binary
├── plugin.dll                     # Windows binary
├── README.md                      # Optional documentation
└── checksums.txt                  # SHA2-256 checksums
```

**Checksum Verification:**
```bash
# Registry stores SHA2-256 checksums
sha256sum libplugin.dylib
# Compare with registry checksum before loading
```

---

### 9.3 CLI Integration

**Plugin Management Commands:**
```bash
# List available plugins in registry
adi plugin search <query>

# Install plugin from registry
adi plugin install adi.tasks

# Install local plugin
adi plugin install --local ./adi_tasks_plugin.dylib

# List installed plugins
adi plugin list

# Update plugin
adi plugin update adi.tasks

# Update all plugins
adi plugin update-all

# Uninstall plugin
adi plugin uninstall adi.tasks

# Show plugin info
adi plugin info adi.tasks
```

---

## 10. Complete Plugin Inventory

### 10.1 Summary

**Total:** 86+ plugins across ecosystem

| Category | Count | Examples |
|----------|-------|----------|
| Core Plugins | 10+ | tasks, indexer, knowledgebase, agent-loop, hive |
| Language Analysis | 11 | rust, python, typescript, go, java, cpp, etc. |
| Translation | 9 | en-US, zh-CN, uk-UA, es-ES, fr-FR, etc. |
| Orchestration Runner | 3 | docker, compose, podman |
| Orchestration Env | 4 | dotenv, vault, 1password, aws-secrets |
| Orchestration Health | 7 | http, tcp, grpc, mysql, postgres, redis, cmd |
| Orchestration Proxy | 11 | cors, rate-limit, auth-jwt, auth-oidc, cache, etc. |
| Orchestration Obs | 4 | stdout, file, loki, prometheus |
| Orchestration Rollout | 2 | recreate, blue-green |
| Extension Plugins | 5+ | llm-uzu, browser-debug, audio, coolify, linter |

---

### 10.2 Core Feature Plugins

| Plugin ID | Type | Provides | Description |
|-----------|------|----------|-------------|
| `adi.tasks` | core | CLI, HTTP, MCP | Task management and execution |
| `adi.indexer` | core | CLI, HTTP, MCP | Code indexing, semantic search |
| `adi.knowledgebase` | core | CLI, HTTP | Graph DB + embeddings |
| `adi.agent-loop` | core | CLI, HTTP | Autonomous LLM agents with tools |
| `adi.api-proxy` | core | CLI, HTTP | LLM API proxy (BYOK/Platform) |
| `adi.hive` | core | CLI, HTTP | Cocoon container orchestration |
| `adi.workflow` | core | CLI | Workflow orchestration |
| `adi.audio` | core | CLI | Audio processing |
| `adi.coolify` | core | CLI | Coolify deployment integration |
| `adi.linter` | core | CLI | Code linting |
| `adi.embed` | extension | Service | Embedding service |

---

### 10.3 Language Analysis Plugins

| Plugin ID | Language | Purpose |
|-----------|----------|---------|
| `adi.lang.rust` | Rust | AST parsing, symbol extraction |
| `adi.lang.python` | Python | AST parsing, type inference |
| `adi.lang.typescript` | TypeScript/JS | AST parsing, type checking |
| `adi.lang.go` | Go | AST parsing, package analysis |
| `adi.lang.java` | Java | AST parsing, class hierarchy |
| `adi.lang.csharp` | C# | AST parsing, namespace resolution |
| `adi.lang.cpp` | C++ | AST parsing, header analysis |
| `adi.lang.ruby` | Ruby | AST parsing, module analysis |
| `adi.lang.php` | PHP | AST parsing, namespace analysis |
| `adi.lang.lua` | Lua | AST parsing, table analysis |
| `adi.lang.swift` | Swift | AST parsing, protocol analysis |

---

### 10.4 Orchestration Plugins (Hive)

#### Runners (3)
- `hive.runner.docker` ✅ Bundled
- `hive.runner.compose` (External)
- `hive.runner.podman` (External)

#### Environment (4)
- `hive.env.dotenv` ✅ Bundled
- `hive.env.vault` (External)
- `hive.env.1password` (External)
- `hive.env.aws-secrets` (External)

#### Health Checks (7)
- `hive.health.http` ✅ Bundled
- `hive.health.tcp` ✅ Bundled
- `hive.health.cmd` (External)
- `hive.health.grpc` (External)
- `hive.health.mysql` (External)
- `hive.health.postgres` (External)
- `hive.health.redis` (External)

#### Proxy Middleware (11)
- `hive.proxy.cors` ✅ Bundled
- `hive.proxy.rate-limit` ✅ Bundled
- `hive.proxy.headers` (External)
- `hive.proxy.ip-filter` (External)
- `hive.proxy.auth-api-key` (External)
- `hive.proxy.auth-basic` (External)
- `hive.proxy.auth-jwt` (External)
- `hive.proxy.auth-oidc` (External)
- `hive.proxy.cache` (External)
- `hive.proxy.compress` (External)
- `hive.proxy.rewrite` (External)

#### Observability (4)
- `hive.obs.stdout` ✅ Bundled
- `hive.obs.file` ✅ Bundled
- `hive.obs.loki` (External)
- `hive.obs.prometheus` (External)

#### Rollout Strategies (2)
- `hive.rollout.recreate` (Built-in)
- `hive.rollout.blue-green` (External)

**Note:** ✅ = Bundled by default with Hive

---

## 11. Plugin Development Guide

### 11.1 Creating a CLI Plugin

**Step 1: Create Plugin Crate**
```bash
cargo new --lib my-plugin
cd my-plugin
```

**Step 2: Configure Cargo.toml**
```toml
[package]
name = "my-plugin"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]  # Dynamic library

[dependencies]
lib-plugin-abi = { path = "../lib-plugin-abi" }
abi_stable = "0.11"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

**Step 3: Implement Plugin**
```rust
// src/lib.rs
use lib_plugin_abi::*;
use abi_stable::std_types::*;

// Plugin metadata
fn plugin_info() -> PluginInfo {
    PluginInfo {
        id: "vendor.myplugin".into(),
        name: "My Plugin".into(),
        version: "0.1.0".into(),
        plugin_type: "extension".into(),
        author: ROption::RSome("Your Name".into()),
        description: ROption::RSome("Does something useful".into()),
    }
}

// Initialize plugin
extern "C" fn plugin_init(ctx: *mut PluginContext) -> i32 {
    unsafe {
        let host = (*ctx).host();

        // Log initialization
        (host.log)(2, "Initializing my-plugin".into());

        // Register CLI commands service
        let cli_vtable = CliCommandsVTable {
            list_commands: my_list_commands,
            run_command: my_run_command,
        };

        let service_desc = ServiceDescriptor {
            id: "adi.cli.commands".into(),
            version: ServiceVersion { major: 1, minor: 0, patch: 0 },
            description: "My plugin CLI commands".into(),
        };

        let service_handle = ServiceHandle {
            service_id: service_desc.id.clone(),
            ptr: std::ptr::null(),
            vtable: &cli_vtable as *const _ as *const _,
        };

        (host.register_service)(service_desc, service_handle);
    }

    0  // Success
}

// Cleanup
extern "C" fn plugin_cleanup(_ctx: *mut PluginContext) {
    // Clean up resources
}

// CLI commands implementation
extern "C" fn my_list_commands(_handle: ServiceHandle) -> RVec<CliCommand> {
    vec![
        CliCommand {
            name: "hello".into(),
            description: "Say hello".into(),
            usage: "myplugin hello [name]".into(),
            has_subcommands: false,
        },
    ].into()
}

extern "C" fn my_run_command(
    _handle: ServiceHandle,
    context: CliContext,
) -> RResult<CliResult, RString> {
    let name = context.args.get(0)
        .map(|s| s.as_str())
        .unwrap_or("World");

    RResult::ROk(CliResult {
        exit_code: 0,
        stdout: format!("Hello, {}!", name).into(),
        stderr: "".into(),
    })
}

// Export vtable
static PLUGIN_VTABLE: PluginVTable = PluginVTable {
    info: plugin_info,
    init: plugin_init,
    update: ROption::RNone,
    cleanup: plugin_cleanup,
    handle_message: ROption::RNone,
};

#[no_mangle]
pub extern "C" fn plugin_entry() -> *const PluginVTable {
    &PLUGIN_VTABLE
}
```

**Step 4: Create plugin.toml**
```toml
[plugin]
id = "vendor.myplugin"
name = "My Plugin"
version = "0.1.0"
type = "extension"
author = "Your Name"
description = "Does something useful"

[compatibility]
api_version = 2

[[provides]]
id = "adi.cli.commands"
version = "1.0.0"

[cli]
command = "myplugin"
description = "My custom plugin"

[binary]
name = "plugin"
```

**Step 5: Build**
```bash
cargo build --release

# Binary at: target/release/libplugin.dylib (macOS)
#            target/release/libplugin.so (Linux)
#            target/release/plugin.dll (Windows)
```

**Step 6: Install Locally**
```bash
mkdir -p ~/.local/share/adi/plugins/vendor.myplugin
cp plugin.toml ~/.local/share/adi/plugins/vendor.myplugin/
cp target/release/libplugin.dylib ~/.local/share/adi/plugins/vendor.myplugin/
```

**Step 7: Test**
```bash
adi myplugin hello Alice
# Output: Hello, Alice!
```

---

### 11.2 Creating an Orchestration Plugin

**Example: Custom Health Check Plugin**

```rust
// Cargo.toml
[dependencies]
lib-plugin-abi-orchestration = { path = "../lib-plugin-abi-orchestration" }
async-trait = "0.1"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
reqwest = "0.11"

// src/lib.rs
use lib_plugin_abi_orchestration::{
    health::{HealthPlugin, HealthResult},
    PluginMetadata,
};
use async_trait::async_trait;
use serde::Deserialize;

#[derive(Deserialize)]
struct GraphQLConfig {
    endpoint: String,
    query: String,
    timeout_ms: Option<u64>,
}

pub struct GraphQLHealthPlugin;

impl GraphQLHealthPlugin {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl HealthPlugin for GraphQLHealthPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            id: "hive.health.graphql".to_string(),
            name: "GraphQL Health Check".to_string(),
            version: "1.0.0".to_string(),
            description: Some("Check GraphQL endpoint health".to_string()),
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
        let config: GraphQLConfig = serde_json::from_value(config.clone())?;
        let start = std::time::Instant::now();

        let client = reqwest::Client::new();
        let response = client
            .post(&config.endpoint)
            .json(&serde_json::json!({
                "query": config.query,
            }))
            .send()
            .await?;

        let elapsed = start.elapsed().as_millis() as u64;
        let is_success = response.status().is_success();

        Ok(HealthResult {
            healthy: is_success,
            message: Some(format!("GraphQL endpoint status: {}", response.status())),
            response_time_ms: elapsed,
            details: [
                ("endpoint".to_string(), config.endpoint),
                ("status".to_string(), response.status().to_string()),
            ].into(),
        })
    }

    async fn shutdown(&self) -> anyhow::Result<()> {
        Ok(())
    }
}
```

---

## 12. Deployment & Distribution

### 12.1 Release Workflow

**Via ADI Workflow:**
```bash
adi workflow release-plugin --plugin adi.tasks --registry production
```

**Manual Build:**
```bash
# 1. Build for all platforms
cargo build --release --target x86_64-unknown-linux-musl
cargo build --release --target x86_64-apple-darwin
cargo build --release --target x86_64-pc-windows-msvc

# 2. Package
tar -czf adi.tasks.tar.gz \
    plugin.toml \
    libadi_tasks_plugin.so \
    libadi_tasks_plugin.dylib \
    adi_tasks_plugin.dll \
    checksums.txt

# 3. Generate checksums
sha256sum libadi_tasks_plugin.* > checksums.txt

# 4. Publish to registry
curl -X POST https://adi-plugin-registry.the-ihor.com/api/publish \
    -H "Authorization: Bearer $TOKEN" \
    -F "plugin=@adi.tasks.tar.gz"
```

---

### 12.2 Installation Flow

```
User runs: adi plugin install adi.tasks
  ↓
1. Query registry API: GET /api/plugins/adi.tasks
   └─> Returns metadata, download URL, checksum
  ↓
2. Download binary: GET /api/plugins/adi.tasks/download
   └─> Downloads adi.tasks.tar.gz
  ↓
3. Verify checksum (SHA2-256)
   └─> Compare computed hash with registry checksum
  ↓
4. Extract to ~/.local/share/adi/plugins/adi.tasks/
   ├─> plugin.toml
   ├─> libadi_tasks_plugin.dylib (platform-specific)
   └─> checksums.txt
  ↓
5. Mark as installed in ~/.config/adi/config.toml
  ↓
6. Load plugin (if enabled)
   └─> Call plugin_entry(), init(), register services
```

---

## 13. Recommendations

### 13.1 Current Strengths

1. **Dual-ABI Design** - Clean separation of general vs orchestration concerns
2. **FFI Stability** - `abi_stable` ensures cross-version compatibility
3. **Service Registry** - Powerful inter-plugin communication
4. **Rich Ecosystem** - 86+ plugins covering broad functionality
5. **Translation System** - 9 languages with Fluent integration
6. **Type Safety** - Both FFI-safe and native Rust trait options

---

### 13.2 Areas for Improvement

#### **Short-Term (1-3 Months)**

1. **Schema Validation**
   - Plugins currently receive `serde_json::Value` (runtime errors)
   - Add JSON Schema validation at config load time
   - Generate documentation from schemas

2. **Plugin Sandboxing**
   - Current plugins have full host access
   - Add permission system (file access, network, etc.)
   - Require manifest declarations for capabilities

3. **Hot-Reload**
   - Support plugin updates without restarting host
   - Graceful service migration during reload
   - Version-aware service replacement

#### **Medium-Term (3-6 Months)**

4. **Plugin Marketplace UI**
   - Web interface for browsing plugins
   - Ratings, reviews, download stats
   - Plugin screenshots and demos

5. **Automated Testing**
   - Plugin test harness
   - Integration test suite for all bundled plugins
   - CI/CD for plugin releases

6. **Plugin Dependencies**
   - Runtime dependency resolution
   - Automatic installation of required plugins
   - Version conflict detection

#### **Long-Term (6-12 Months)**

7. **WebAssembly Plugins**
   - WASM-based plugins for browser/edge
   - Sandboxed execution environment
   - Cross-platform compatibility (no native compilation)

8. **Plugin Composition**
   - Higher-level plugins built from primitives
   - Plugin templates for common patterns
   - Visual plugin builder

9. **Distributed Plugin Registry**
   - Decentralized plugin distribution
   - Mirror support for high availability
   - IPFS/content-addressed storage

---

### 13.3 Migration Recommendations

#### **For New Plugins:**
- Use **lib-plugin-abi** for CLI/HTTP/MCP functionality
- Use **lib-plugin-abi-orchestration** for Hive-specific features
- Follow naming conventions strictly
- Include comprehensive plugin.toml metadata

#### **For Existing Plugins:**
- Audit for security vulnerabilities
- Add version constraints to dependencies
- Document all services provided/required
- Add integration tests

#### **For Host (adi-cli):**
- Implement permission system before 1.0 release
- Add plugin verification (signatures, checksums)
- Improve error messages for plugin failures
- Add plugin profiling/metrics

---

## Appendix A: File Locations

```
ADI Plugin System Files
│
├── Plugin ABIs
│   ├── /crates/lib/lib-plugin-abi/                (General ABI)
│   │   ├── src/vtable.rs                          (PluginVTable, HostVTable)
│   │   ├── src/types.rs                           (PluginInfo, errors)
│   │   ├── src/cli.rs                             (CLI commands service)
│   │   ├── src/http.rs                            (HTTP routes service)
│   │   ├── src/mcp.rs                             (MCP tools/resources/prompts)
│   │   └── src/service.rs                         (Service registry types)
│   │
│   └── /crates/lib/lib-plugin-abi-orchestration/  (Orchestration ABI)
│       ├── src/runner.rs                          (RunnerPlugin trait)
│       ├── src/health.rs                          (HealthPlugin trait)
│       ├── src/env.rs                             (EnvPlugin trait)
│       ├── src/proxy.rs                           (ProxyPlugin trait)
│       ├── src/obs.rs                             (ObsPlugin trait)
│       └── src/rollout.rs                         (RolloutPlugin trait)
│
├── Plugin Infrastructure
│   ├── /crates/lib/lib-plugin-host/               (Plugin loading)
│   │   ├── src/host.rs                            (PluginHost)
│   │   ├── src/loader.rs                          (Dynamic loading)
│   │   ├── src/service_registry.rs                (Service registry)
│   │   └── src/callbacks.rs                       (HostVTable impl)
│   │
│   ├── /crates/lib/lib-plugin-manifest/           (Manifest parsing)
│   │   ├── src/plugin.rs                          (plugin.toml)
│   │   └── src/package.rs                         (package.toml)
│   │
│   └── /crates/lib/lib-i18n-core/                 (Translation system)
│       ├── src/core.rs                            (I18n implementation)
│       ├── src/discovery.rs                       (Service discovery)
│       └── src/macro.rs                           (t!() macro)
│
├── CLI Integration
│   └── /crates/adi-cli/src/
│       ├── main.rs                                (CLI entry point)
│       ├── plugin_runtime.rs                      (Plugin loading)
│       └── plugin_registry.rs                     (Registry client)
│
├── Plugin Registry
│   └── /crates/adi-plugin-registry-http/src/
│       ├── main.rs                                (HTTP API server)
│       └── storage.rs                             (Plugin storage)
│
└── Plugin Implementations
    ├── /crates/adi-tasks/plugin/                  (Tasks plugin)
    ├── /crates/adi-indexer/plugin/                (Indexer plugin)
    ├── /crates/adi-agent-loop/plugin/             (Agent loop plugin)
    ├── /crates/hive/plugins/                      (Orchestration plugins)
    │   ├── hive-runner-docker/
    │   ├── hive-health-http/
    │   ├── hive-proxy-cors/
    │   └── ... (32 total)
    └── /crates/adi-cli/plugins/                   (Translation plugins)
        ├── adi-cli-lang-en/
        ├── adi-cli-lang-zh-CN/
        └── ... (9 languages)
```

---

## Appendix B: Quick Reference

### Plugin Type Decision Tree

```
What are you building?
│
├── CLI Command?
│   └─> Use lib-plugin-abi + adi.cli.commands service
│
├── HTTP API Endpoint?
│   └─> Use lib-plugin-abi + adi.http.routes service
│
├── MCP Tool/Resource?
│   └─> Use lib-plugin-abi + adi.mcp.tools service
│
├── Container Runner?
│   └─> Use lib-plugin-abi-orchestration + RunnerPlugin trait
│
├── Health Check?
│   └─> Use lib-plugin-abi-orchestration + HealthPlugin trait
│
├── Proxy Middleware?
│   └─> Use lib-plugin-abi-orchestration + ProxyPlugin trait
│
├── Observability Sink?
│   └─> Use lib-plugin-abi-orchestration + ObsPlugin trait
│
├── Translation?
│   └─> Use lib-i18n-core + register adi.i18n.<namespace>.<lang> service
│
└── Custom Service?
    └─> Use lib-plugin-abi + register custom service ID
```

---

### Common Plugin Operations

```bash
# Search for plugins
adi plugin search <query>

# Install plugin
adi plugin install <plugin-id>

# List installed plugins
adi plugin list

# Update plugin
adi plugin update <plugin-id>

# Uninstall plugin
adi plugin uninstall <plugin-id>

# Show plugin info
adi plugin info <plugin-id>

# Build plugin locally
cargo build --release

# Install local plugin
adi plugin install --local ./target/release/libplugin.dylib
```

---

**Document Version:** 1.0
**Last Updated:** 2026-01-31
**Maintainer:** ADI Development Team
