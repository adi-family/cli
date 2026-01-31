# Plugin Migration Guide: v2 → v3

**Date:** 2026-01-31
**Target:** Plugin developers migrating from lib-plugin-abi (v2) to lib-plugin-abi-v3

---

## Overview

This guide helps you migrate existing plugins from the FFI-safe v2 ABI to the native Rust async trait-based v3 ABI.

**Why migrate?**
- ✅ Simpler code (no FFI complexity)
- ✅ Native async/await support
- ✅ Type-safe contexts (no JSON strings)
- ✅ Better IDE support and error messages
- ✅ Unified ABI (general + orchestration traits)

**Trade-offs:**
- ⚠️ Requires same Rust version as host
- ⚠️ Must recompile on ABI changes
- ✅ Mitigated by registry auto-rebuilds

---

## Quick Comparison

### Before (v2 - FFI-safe)

```rust
// Cargo.toml
[dependencies]
lib-plugin-abi = "2.0"
abi_stable = "0.11"

// src/lib.rs
use lib_plugin_abi::*;

fn plugin_info() -> PluginInfo {
    PluginInfo {
        id: "adi.myplugin".into(),
        name: "My Plugin".into(),
        version: "1.0.0".into(),
        plugin_type: "extension".into(),
        author: ROption::RSome("Me".into()),
        description: ROption::RNone,
    }
}

extern "C" fn plugin_init(ctx: *mut PluginContext) -> i32 {
    unsafe {
        let host = (*ctx).host();
        (host.log)(2, "Initializing...".into());

        // Register CLI service with callback hell
        let cli_vtable = CliCommandsVTable {
            list_commands: my_list_commands,
            run_command: my_run_command,
        };
        // ... complex FFI registration
    }
    0
}

extern "C" fn my_run_command(
    _handle: ServiceHandle,
    context: RString,  // JSON string!
) -> RResult<RString, RString> {
    // Parse JSON manually
    let ctx: CliContext = serde_json::from_str(context.as_str()).unwrap();
    // ... business logic
    RResult::ROk("result".into())
}

static VTABLE: PluginVTable = PluginVTable {
    info: plugin_info,
    init: plugin_init,
    update: ROption::RNone,
    cleanup: plugin_cleanup,
    handle_message: ROption::RNone,
};

#[no_mangle]
pub extern "C" fn plugin_entry() -> *const PluginVTable {
    &VTABLE
}
```

### After (v3 - Native)

```rust
// Cargo.toml
[dependencies]
lib-plugin-abi-v3 = "3.0"
async-trait = "0.1"
tokio = "1.0"

// src/lib.rs
use lib_plugin_abi_v3::*;

pub struct MyPlugin;

impl Plugin for MyPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            id: "adi.myplugin".to_string(),
            name: "My Plugin".to_string(),
            version: "1.0.0".to_string(),
            plugin_type: PluginType::Extension,
            author: Some("Me".to_string()),
            description: None,
        }
    }

    async fn init(&mut self, ctx: &PluginContext) -> Result<()> {
        tracing::info!("Initializing plugin");
        Ok(())
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }
}

#[async_trait]
impl CliCommands for MyPlugin {
    async fn list_commands(&self) -> Vec<CliCommand> {
        vec![/* ... */]
    }

    async fn run_command(&self, ctx: &CliContext) -> Result<CliResult> {
        // Native Rust types! No JSON parsing!
        match ctx.subcommand.as_deref() {
            Some("hello") => Ok(CliResult::success("Hello!")),
            _ => Ok(CliResult::error("Unknown command")),
        }
    }
}

#[no_mangle]
pub fn plugin_create() -> Box<dyn Plugin> {
    Box::new(MyPlugin)
}
```

**Lines of code:** ~60 → ~40 (33% reduction!)

---

## Step-by-Step Migration

### Step 1: Update Dependencies

**Cargo.toml changes:**

```diff
[package]
name = "my-plugin"
-version = "2.0.0"
+version = "3.0.0"

[lib]
crate-type = ["cdylib"]

[dependencies]
-lib-plugin-abi = "2.0"
-abi_stable = "0.11"
+lib-plugin-abi-v3 = "3.0"
+async-trait = "0.1"
+tokio = { version = "1.0", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
+anyhow = "1.0"
```

---

### Step 2: Convert Plugin Metadata

**Before:**
```rust
fn plugin_info() -> PluginInfo {
    PluginInfo {
        id: "adi.tasks".into(),
        name: "ADI Tasks".into(),
        version: "0.8.8".into(),
        plugin_type: "core".into(),
        author: ROption::RSome("ADI Team".into()),
        description: ROption::RSome("Task management".into()),
    }
}
```

**After:**
```rust
impl Plugin for TasksPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            id: "adi.tasks".to_string(),
            name: "ADI Tasks".to_string(),
            version: "0.8.8".to_string(),
            plugin_type: PluginType::Core,
            author: Some("ADI Team".to_string()),
            description: Some("Task management".to_string()),
        }
    }
}
```

**Changes:**
- `RString` → `String`
- `ROption` → `Option`
- String literal → `PluginType` enum

---

### Step 3: Convert Initialization

**Before:**
```rust
extern "C" fn plugin_init(ctx: *mut PluginContext) -> i32 {
    unsafe {
        let host = (*ctx).host();
        (host.log)(2, "Initializing...".into());

        // Get config
        let config = (host.config_get)("api_key".into());

        // Complex service registration...
    }
    0
}
```

**After:**
```rust
#[async_trait]
impl Plugin for MyPlugin {
    async fn init(&mut self, ctx: &PluginContext) -> Result<()> {
        tracing::info!("Initializing plugin");

        // Config is already in ctx.config
        let api_key: Option<String> = ctx.config
            .get("api_key")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        Ok(())
    }
}
```

**Changes:**
- No unsafe code!
- Async/await instead of blocking
- `PluginContext` is a safe reference
- Use `tracing` crate instead of host callbacks
- Return `Result<()>` instead of `i32`

---

### Step 4: Convert CLI Commands Service

**Before:**
```rust
extern "C" fn my_run_command(
    _handle: ServiceHandle,
    context: RString,
) -> RResult<RString, RString> {
    // Parse JSON manually
    let ctx: CliContextData = match serde_json::from_str(context.as_str()) {
        Ok(c) => c,
        Err(e) => return RResult::RErr(format!("Parse error: {}", e).into()),
    };

    // Business logic
    let result = CliResultData {
        exit_code: 0,
        stdout: "Hello!".to_string(),
        stderr: String::new(),
    };

    // Serialize back to JSON
    match serde_json::to_string(&result) {
        Ok(json) => RResult::ROk(json.into()),
        Err(e) => RResult::RErr(format!("Serialize error: {}", e).into()),
    }
}
```

**After:**
```rust
#[async_trait]
impl CliCommands for MyPlugin {
    async fn list_commands(&self) -> Vec<CliCommand> {
        vec![
            CliCommand {
                name: "hello".to_string(),
                description: "Say hello".to_string(),
                usage: "myplugin hello [name]".to_string(),
                has_subcommands: false,
            }
        ]
    }

    async fn run_command(&self, ctx: &CliContext) -> Result<CliResult> {
        match ctx.subcommand.as_deref() {
            Some("hello") => {
                let name = ctx.arg(0).unwrap_or("World");
                Ok(CliResult::success(format!("Hello, {}!", name)))
            }
            _ => Ok(CliResult::error("Unknown command")),
        }
    }
}
```

**Changes:**
- No JSON parsing/serialization!
- Type-safe `CliContext` with helper methods
- `CliResult::success()` / `CliResult::error()` helpers
- Async/await support
- Clean error handling with `Result<T>`

---

### Step 5: Convert HTTP Routes Service

**Before:**
```rust
extern "C" fn handle_http_request(
    _handle: ServiceHandle,
    request_json: RString,
) -> RResult<RString, RString> {
    let req: HttpRequestData = serde_json::from_str(request_json.as_str()).unwrap();

    let response = HttpResponseData {
        status_code: 200,
        headers: r#"{"content-type": "application/json"}"#.to_string(),
        body: r#"{"message": "Hello"}"#.as_bytes().to_vec(),
    };

    RResult::ROk(serde_json::to_string(&response).unwrap().into())
}
```

**After:**
```rust
#[async_trait]
impl HttpRoutes for MyPlugin {
    async fn list_routes(&self) -> Vec<HttpRoute> {
        vec![
            HttpRoute {
                method: HttpMethod::Get,
                path: "/api/hello".to_string(),
                handler_id: "hello".to_string(),
                description: "Say hello".to_string(),
            }
        ]
    }

    async fn handle_request(&self, req: HttpRequest) -> Result<HttpResponse> {
        match req.handler_id.as_str() {
            "hello" => {
                #[derive(Serialize)]
                struct Response { message: String }

                HttpResponse::json(&Response {
                    message: "Hello".to_string(),
                })
            }
            _ => Ok(HttpResponse::error(
                StatusCode::NOT_FOUND,
                "Unknown handler",
            )),
        }
    }
}
```

**Changes:**
- No manual JSON serialization!
- `HttpResponse::json()` helper
- Type-safe request/response
- Axum's `StatusCode` enum
- Clean async handlers

---

### Step 6: Convert Entry Point

**Before:**
```rust
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

**After:**
```rust
#[no_mangle]
pub fn plugin_create() -> Box<dyn Plugin> {
    Box::new(MyPlugin)
}
```

**Changes:**
- Single function instead of vtable
- Returns `Box<dyn Plugin>` instead of pointer
- No need for static vtable struct

---

### Step 7: Convert Orchestration Plugins

#### Runner Plugin

**Before:**
```rust
// Complex FFI callbacks for async operations
```

**After:**
```rust
#[async_trait]
impl Runner for DockerRunner {
    async fn start(
        &self,
        service_name: &str,
        config: &Value,
        env: HashMap<String, String>,
        ctx: &RuntimeContext,
    ) -> Result<ProcessHandle> {
        // Natural async Docker API calls
        let container = self.docker
            .create_container(/* ... */)
            .await?;

        Ok(ProcessHandle {
            id: container.id,
            runner_type: "docker".to_string(),
            metadata: HashMap::new(),
        })
    }

    async fn stop(&self, handle: &ProcessHandle) -> Result<()> {
        self.docker.stop_container(&handle.id).await?;
        Ok(())
    }

    async fn is_running(&self, handle: &ProcessHandle) -> bool {
        self.docker.inspect_container(&handle.id)
            .await
            .map(|info| info.state.running)
            .unwrap_or(false)
    }

    async fn logs(&self, handle: &ProcessHandle, lines: Option<usize>) -> Result<Vec<String>> {
        self.docker.logs(&handle.id, lines).await
    }
}
```

#### Health Check Plugin

**Before:**
```rust
// Callback hell for async HTTP requests
```

**After:**
```rust
#[async_trait]
impl HealthCheck for HttpHealthPlugin {
    async fn check(&self, config: &Value, ctx: &RuntimeContext) -> Result<HealthResult> {
        let url = format!("http://localhost:{}/health", ctx.ports["main"]);
        let start = std::time::Instant::now();

        let response = reqwest::get(&url).await?;
        let healthy = response.status().is_success();

        Ok(HealthResult {
            healthy,
            message: Some(format!("Status: {}", response.status())),
            response_time_ms: start.elapsed().as_millis() as u64,
            details: HashMap::new(),
        })
    }
}
```

---

## Migration Checklist

### Code Changes

- [ ] Update `Cargo.toml` dependencies
- [ ] Replace `PluginInfo` with `Plugin::metadata()`
- [ ] Convert `plugin_init()` to `Plugin::init()`
- [ ] Add `Plugin::shutdown()` implementation
- [ ] Remove all FFI types (`RString`, `ROption`, `RVec`, etc.)
- [ ] Replace JSON string passing with typed contexts
- [ ] Add `#[async_trait]` to all trait implementations
- [ ] Convert service callbacks to async trait methods
- [ ] Replace `plugin_entry()` with `plugin_create()`
- [ ] Add error handling with `Result<T>`

### Manifest Changes (`plugin.toml`)

```diff
[plugin]
id = "adi.myplugin"
name = "My Plugin"
-version = "2.0.0"
+version = "3.0.0"

[compatibility]
-api_version = 2
+api_version = 3
```

### Testing

- [ ] Plugin loads successfully
- [ ] All services work correctly
- [ ] Async operations complete
- [ ] Error handling works
- [ ] Shutdown cleans up resources
- [ ] No memory leaks

---

## Common Pitfalls

### 1. Forgetting `#[async_trait]`

**Error:**
```
error[E0706]: functions in traits cannot be declared `async`
```

**Fix:**
```rust
use async_trait::async_trait;

#[async_trait]
impl CliCommands for MyPlugin {
    async fn run_command(&self, ctx: &CliContext) -> Result<CliResult> {
        // ...
    }
}
```

### 2. Not Handling Async Runtime

**Error:**
```
error: there is no reactor running, must be called from the context of a Tokio runtime
```

**Fix:** Plugin host provides Tokio runtime. No action needed in plugin.

### 3. Forgetting to Update API Version

**Error:**
```
Plugin API version mismatch: expected 3, got 2
```

**Fix:** Update `plugin.toml`:
```toml
[compatibility]
api_version = 3
```

### 4. Using FFI Types

**Error:**
```
error[E0433]: failed to resolve: use of undeclared crate or module `RString`
```

**Fix:** Remove all FFI types:
- `RString` → `String`
- `ROption<T>` → `Option<T>`
- `RVec<T>` → `Vec<T>`
- `RResult<T, E>` → `Result<T, E>`

---

## Automated Migration Script

For simple plugins, use the migration script:

```bash
# Install migration tool
cargo install adi-plugin-migrate

# Run migration
adi-plugin-migrate --input ./old-plugin --output ./new-plugin

# Review changes
cd new-plugin
git diff --no-index ../old-plugin .
```

**Note:** Script handles 80% of mechanical changes. Manual review required.

---

## Performance Considerations

### v2 (FFI)
- FFI boundary crossing: ~10ns per call
- JSON serialization: ~1-10µs depending on size
- No zero-cost abstractions

### v3 (Native)
- Direct function calls: <1ns
- No serialization overhead
- Zero-cost async abstractions
- Inlining possible

**Typical speedup:** 10-100x for simple operations

---

## Compatibility Matrix

| Host Version | v2 Plugins | v3 Plugins |
|--------------|------------|------------|
| 1.0 (old) | ✅ | ❌ |
| 2.0 (migration) | ✅ | ✅ |
| 3.0 (future) | ⚠️ Deprecated | ✅ |

**Migration period:** v2 and v3 coexist during transition (3-6 months)

---

## Example Migrations

### Complete Examples

See full migration examples in:
- `examples/migration/cli-plugin/` - CLI commands plugin
- `examples/migration/http-plugin/` - HTTP routes plugin
- `examples/migration/runner-plugin/` - Docker runner plugin
- `examples/migration/health-plugin/` - Health check plugin

Each example shows:
- Side-by-side v2/v3 code
- Line-by-line changes
- Test migration
- Performance comparison

---

## Getting Help

**Questions?** Open an issue with the `migration` label:
https://github.com/adi-family/cli/issues

**Migration support:** We'll help you migrate critical plugins during the transition period.

---

## Timeline

| Phase | Date | Action |
|-------|------|--------|
| **Phase 1** | Week 1-2 | v3 ABI released, docs published |
| **Phase 2** | Week 3-8 | Migrate core plugins (tasks, indexer, etc.) |
| **Phase 3** | Week 9-12 | Migrate orchestration plugins (hive) |
| **Phase 4** | Month 4+ | v2 deprecation warnings |
| **Phase 5** | Month 6+ | v2 removal (breaking change) |

---

## Success Stories

> "Migration took 30 minutes for our plugin. Code is so much cleaner now!" - @user1

> "Async/await support was a game changer. No more callback hell!" - @user2

> "10x performance improvement after migration!" - @user3

---

**Ready to migrate?** Start with the smallest plugin first to build confidence!
