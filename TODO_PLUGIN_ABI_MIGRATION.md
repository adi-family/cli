# Plugin ABI Migration: v2/Orchestration → v3

## Agent Instructions

This file tracks the migration of all plugins from legacy ABIs to the unified v3 ABI.

**When working on this migration:**
1. Pick an uncompleted task from the list below
2. Follow the migration patterns established in completed plugins
3. Mark the task as `[x]` when done and the plugin compiles
4. Run `cargo check -p <package-name>` to verify compilation

**Migration patterns (see completed examples):**
- CLI plugins: `adi-tasks-plugin`, `adi-hive-plugin`
- Orchestration plugins: TBD (runner-docker will be the reference)

**Key files:**
- `crates/lib/lib-plugin-abi-v3/` - The unified v3 ABI
- `crates/lib/lib-plugin-abi/` - Legacy v2 ABI (to be removed)

---

## Migration Status

### Phase 1: Core CLI Plugins (HIGH PRIORITY)
These are the most used plugins - migrate first.

- [x] `adi-tasks-plugin` - Task management CLI
- [x] `adi-hive-plugin` - Service orchestration CLI
- [x] `adi-indexer-plugin` - Code indexer with MCP tools
- [x] `adi-knowledgebase-plugin` - Knowledge graph plugin
- [x] `adi-workflow-plugin` - Workflow execution
- [x] `adi-coolify-plugin` - Coolify deployment CLI
- [x] `adi-linter-plugin` - Code linting
- [x] `adi-browser-debug-plugin` - Browser debugging
- [x] `adi-audio-plugin` - Audio processing
- [x] `adi-agent-loop-plugin` - Autonomous LLM agents
- [x] `adi-api-proxy-plugin` - LLM API proxy CLI
- [x] `adi-embed-plugin` - Embedding service
- [x] `adi-llm-extract-plugin` - LLM extraction
- [x] `adi-llm-uzu-plugin` - Local LLM inference (Apple Silicon)
- [x] `cocoon` - Containerized worker

### Phase 2: i18n/Language Plugins (COMPLETED ✅)
All translation plugins migrated to v3.

**adi-cli language plugins (9) - COMPLETED ✅:**
- [x] `adi-cli/plugins/en-US`
- [x] `adi-cli/plugins/de-DE`
- [x] `adi-cli/plugins/es-ES`
- [x] `adi-cli/plugins/fr-FR`
- [x] `adi-cli/plugins/ja-JP`
- [x] `adi-cli/plugins/ko-KR`
- [x] `adi-cli/plugins/ru-RU`
- [x] `adi-cli/plugins/uk-UA`
- [x] `adi-cli/plugins/zh-CN`

**adi-workflow language plugins (9) - COMPLETED ✅:**
- [x] `adi-workflow/langs/en`
- [x] `adi-workflow/langs/de`
- [x] `adi-workflow/langs/es`
- [x] `adi-workflow/langs/fr`
- [x] `adi-workflow/langs/ja`
- [x] `adi-workflow/langs/ko`
- [x] `adi-workflow/langs/ru`
- [x] `adi-workflow/langs/uk`
- [x] `adi-workflow/langs/zh`

**adi-coolify - REMOVED:**
- adi-coolify component and all 9 language plugins deleted from workspace

### Phase 3: Language Analyzer Plugins (COMPLETED)
Code analysis plugins - use new `LanguageAnalyzer` trait in v3. See migration pattern below.

- [x] `adi-lang/cpp/plugin`
- [x] `adi-lang/csharp/plugin`
- [x] `adi-lang/go/plugin`
- [x] `adi-lang/java/plugin`
- [x] `adi-lang/lua/plugin`
- [x] `adi-lang/php/plugin`
- [x] `adi-lang/python/plugin`
- [x] `adi-lang/ruby/plugin`
- [x] `adi-lang/rust/plugin`
- [x] `adi-lang/swift/plugin`
- [x] `adi-lang/typescript/plugin`

### Phase 4: Hive Orchestration Plugins (COMPLETED ✅)
All orchestration plugins migrated to lib-plugin-abi-v3.
Also: `hive-core` now uses lib-plugin-abi-v3 instead of lib-plugin-abi-orchestration.

**Runner plugins (3):**
- [x] `hive/plugins/runner-docker` - Docker container runner
- [x] `hive/plugins/runner-compose` - Docker Compose runner
- [x] `hive/plugins/runner-podman` - Podman runner

**Environment plugins (4):**
- [x] `hive/plugins/env-dotenv` - .env file loader
- [x] `hive/plugins/env-vault` - HashiCorp Vault
- [x] `hive/plugins/env-1password` - 1Password secrets
- [x] `hive/plugins/env-aws-secrets` - AWS Secrets Manager

**Health check plugins (7):**
- [x] `hive/plugins/health-http` - HTTP health checks
- [x] `hive/plugins/health-tcp` - TCP port checks
- [x] `hive/plugins/health-cmd` - Command execution checks
- [x] `hive/plugins/health-grpc` - gRPC health checks
- [x] `hive/plugins/health-mysql` - MySQL connectivity
- [x] `hive/plugins/health-postgres` - PostgreSQL connectivity
- [x] `hive/plugins/health-redis` - Redis connectivity

**Proxy middleware plugins (11):**
- [x] `hive/plugins/proxy-cors` - CORS middleware
- [x] `hive/plugins/proxy-rate-limit` - Rate limiting
- [x] `hive/plugins/proxy-auth-basic` - Basic auth
- [x] `hive/plugins/proxy-auth-jwt` - JWT auth
- [x] `hive/plugins/proxy-auth-api-key` - API key auth
- [x] `hive/plugins/proxy-auth-oidc` - OIDC auth
- [x] `hive/plugins/proxy-cache` - Response caching
- [x] `hive/plugins/proxy-compress` - Compression
- [x] `hive/plugins/proxy-headers` - Header manipulation
- [x] `hive/plugins/proxy-ip-filter` - IP filtering
- [x] `hive/plugins/proxy-rewrite` - URL rewriting

**Observability plugins (4):**
- [x] `hive/plugins/obs-stdout` - Console logging
- [x] `hive/plugins/obs-file` - File logging
- [x] `hive/plugins/obs-loki` - Grafana Loki
- [x] `hive/plugins/obs-prometheus` - Prometheus metrics

**Rollout strategy plugins (2):**
- [x] `hive/plugins/rollout-recreate` - Recreate deployment
- [x] `hive/plugins/rollout-blue-green` - Blue-green deployment

**Core (hive-core):**
- [x] `hive/core` - Uses v3 traits (Runner, HealthCheck, EnvProvider, etc.)

### Phase 5: Cleanup (COMPLETED ✅)

**Completed:**
- [x] Update `hive/core` to import v3 traits (Runner, HealthCheck, EnvProvider, ProxyMiddleware, ObservabilitySink, RolloutStrategy) instead of orchestration traits
- [x] Add HookExecutor to lib-plugin-abi-v3 (was previously only in orchestration ABI)
- [x] Fix PluginMetadata to include `category` field with Default support
- [x] Add `Internal` variant to PluginError for compatibility
- [x] Remove `lib-plugin-abi-orchestration` from workspace Cargo.toml
- [x] Delete `crates/lib/lib-plugin-abi-orchestration/` directory
- [x] Migrate `adi-workflow/langs/*` (9 plugins) - COMPLETED ✅
- [x] `adi-coolify` - REMOVED (entire component deleted from workspace)  
- [x] Update `adi-cli` to only use v3 plugin loading (routes v2 through lib-plugin-host)
- [x] Update `adi-indexer/core` to use v3 instead of v2 ServiceHandle (routes v2 through lib-plugin-host)
- [x] Update `tsp-gen-plugin` to route v2 through lib-plugin-host
- [x] Isolate `lib-plugin-abi` (v2) - now only used internally by lib-plugin-host
- [x] Migrate `tsp-gen-plugin` from v2 to v3 ABI
- [x] Update CLAUDE.md documentation
- [x] Remove v2 fallback paths in `adi-cli/plugin_runtime.rs` - Removed PluginHost, now uses only PluginManagerV3
- [x] Remove ServiceRegistryAdapter from `adi-cli/main.rs` - i18n now loads FTL files directly
- [x] Remove v2 error conversion in `adi-cli/error.rs` - No longer needed
- [x] Refactor i18n to load translations directly from FTL files - Added `I18n::new_standalone()` method

### Phase 6: lib-plugin-host Migration (COMPLETED ✅)

**Completed:**
- [x] Remove v2 files from lib-plugin-host (host.rs, loader.rs, callbacks.rs, service_registry.rs)
- [x] Update lib.rs exports to remove v2 modules and re-exports
- [x] Remove lib-plugin-abi and abi_stable dependencies from Cargo.toml
- [x] Add LanguageAnalyzer support to PluginManagerV3
- [x] Add Embedder support to PluginManagerV3 (new trait in lib-plugin-abi-v3/src/embed.rs)
- [x] Add thread-local `current_plugin_manager()` for plugin-to-plugin access
- [x] Migrate adi-indexer/core from ServiceRegistry to PluginManagerV3
- [x] Migrate lib-embed PluginEmbedder to use PluginManagerV3
- [x] Migrate adi-knowledgebase/core to use PluginManagerV3
- [x] Update adi-indexer/plugin and adi-knowledgebase/plugin to use current_plugin_manager()
- [x] Update error.rs to simplify error types
- [x] Full workspace compilation verified

**Status:** lib-plugin-host is now pure v3. No v2 code remains.

**New lib-plugin-host Architecture:**
- `config.rs` - PluginConfig configuration
- `error.rs` - HostError error types
- `installed.rs` - InstalledPackage, InstalledPlugin types
- `loader_v3.rs` - LoadedPluginV3 for dynamic loading
- `manager_v3.rs` - PluginManagerV3 with all service registries

**New v3 Service Types:**
- `LanguageAnalyzer` - Code analysis plugins (keyed by language name)
- `Embedder` - Text embedding plugins (keyed by provider name)
- Thread-local `current_plugin_manager()` for plugin-to-plugin communication

**i18n Changes:**
- `lib-i18n-core` now supports standalone mode via `I18n::new_standalone()`
- `adi-cli` loads translation FTL files directly from installed plugins (no ServiceRegistry needed)
- Translation plugins still provide FTL files, but discovery is file-based instead of service-based

---

## Migration Pattern Reference

### CLI Plugin Migration (v2 → v3)

**Before (v2):**
```rust
use abi_stable::std_types::{ROption, RResult, RStr, RString};
use lib_plugin_abi::{PluginContext, PluginInfo, PluginVTable, ...};

static PLUGIN_VTABLE: PluginVTable = PluginVTable {
    info: plugin_info,
    init: plugin_init,
    cleanup: plugin_cleanup,
    ...
};

#[no_mangle]
pub extern "C" fn plugin_entry() -> *const PluginVTable {
    &PLUGIN_VTABLE
}
```

**After (v3):**
```rust
use lib_plugin_abi_v3::{
    async_trait, Plugin, PluginContext, PluginMetadata, PluginType,
    cli::{CliCommand, CliCommands, CliContext, CliResult},
    Result as PluginResult, SERVICE_CLI_COMMANDS,
};

pub struct MyPlugin;

#[async_trait]
impl Plugin for MyPlugin {
    fn metadata(&self) -> PluginMetadata { ... }
    async fn init(&mut self, ctx: &PluginContext) -> PluginResult<()> { ... }
    async fn shutdown(&self) -> PluginResult<()> { ... }
    fn provides(&self) -> Vec<&'static str> { vec![SERVICE_CLI_COMMANDS] }
}

#[async_trait]
impl CliCommands for MyPlugin {
    async fn list_commands(&self) -> Vec<CliCommand> { ... }
    async fn run_command(&self, ctx: &CliContext) -> PluginResult<CliResult> { ... }
}

#[no_mangle]
pub fn plugin_create() -> Box<dyn Plugin> {
    Box::new(MyPlugin::new())
}
```

### Orchestration Plugin Migration (orchestration ABI → v3)

**Before (orchestration ABI):**
```rust
use lib_plugin_abi_orchestration::{
    runner::{RunnerPlugin, ProcessHandle, HookExitStatus},
    PluginCategory, PluginMetadata, RuntimeContext,
};

#[async_trait]
impl RunnerPlugin for DockerRunner {
    fn metadata(&self) -> PluginMetadata { ... }
    async fn init(&mut self, defaults: &Value) -> Result<()> { ... }
    async fn start(&self, ...) -> Result<ProcessHandle> { ... }
    async fn stop(&self, handle: &ProcessHandle) -> Result<()> { ... }
    ...
}
```

**After (v3):**
```rust
use lib_plugin_abi_v3::{
    async_trait, Plugin, PluginContext, PluginMetadata, PluginType,
    runner::{Runner, ProcessHandle, RuntimeContext},
    Result as PluginResult, SERVICE_RUNNER,
};

pub struct DockerRunner;

#[async_trait]
impl Plugin for DockerRunner {
    fn metadata(&self) -> PluginMetadata { ... }
    async fn init(&mut self, ctx: &PluginContext) -> PluginResult<()> { ... }
    async fn shutdown(&self) -> PluginResult<()> { ... }
    fn provides(&self) -> Vec<&'static str> { vec![SERVICE_RUNNER] }
}

#[async_trait]
impl Runner for DockerRunner {
    async fn start(&self, ...) -> PluginResult<ProcessHandle> { ... }
    async fn stop(&self, handle: &ProcessHandle) -> PluginResult<()> { ... }
    ...
}

#[no_mangle]
pub fn plugin_create() -> Box<dyn Plugin> {
    Box::new(DockerRunner::new())
}
```

---

## Cargo.toml Changes

**Before:**
```toml
[dependencies]
lib-plugin-abi = { path = "../../lib/lib-plugin-abi" }
abi_stable = "0.11"
```

**After:**
```toml
[dependencies]
lib-plugin-abi-v3 = { path = "../../lib/lib-plugin-abi-v3" }
```

---

## Testing

After migrating a plugin:
1. `cargo check -p <package-name>` - Must compile without errors
2. `cargo build -p <package-name>` - Must build successfully
3. Test the plugin manually if possible

### Language Analyzer Plugin Migration (lib-indexer-lang-abi → v3)

**Before (lib-indexer-lang-abi):**
```rust
use abi_stable::std_types::{RString, RVec};
use lib_plugin_abi::{PluginVTable, ServiceHandle, ServiceVTable};
use lib_indexer_lang_abi::{ParsedSymbolAbi, ParsedReferenceAbi, LanguageInfoAbi};

extern "C" fn service_invoke(method: *const i8, args: *const i8) -> RString {
    match method {
        "get_grammar" => ...,
        "extract_symbols" => ...,
        "extract_references" => ...,
        "get_info" => ...,
    }
}

#[no_mangle]
pub extern "C" fn plugin_entry() -> *const PluginVTable { ... }
```

**After (v3):**
```rust
use lib_plugin_abi_v3::{
    async_trait, Plugin, PluginContext, PluginMetadata, PluginType,
    lang::{LanguageAnalyzer, LanguageInfo, ParsedSymbol, ParsedReference, SymbolKind, Location},
    Result as PluginResult, SERVICE_LANGUAGE_ANALYZER,
};
use tree_sitter::Language;

pub struct RustAnalyzer;

impl Plugin for RustAnalyzer {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            id: "adi.lang.rust".to_string(),
            name: "Rust Language Support".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            plugin_type: PluginType::Extension,
            author: Some("ADI Team".to_string()),
            description: Some("Rust code analysis plugin".to_string()),
            category: None,
        }
    }

    async fn init(&mut self, _ctx: &PluginContext) -> PluginResult<()> { Ok(()) }
    async fn shutdown(&self) -> PluginResult<()> { Ok(()) }
    fn provides(&self) -> Vec<&'static str> { vec![SERVICE_LANGUAGE_ANALYZER] }
}

#[async_trait]
impl LanguageAnalyzer for RustAnalyzer {
    fn language_info(&self) -> LanguageInfo {
        LanguageInfo::new("rust", "Rust")
            .with_extensions(["rs"])
            .with_version(env!("CARGO_PKG_VERSION"))
    }

    async fn extract_symbols(&self, source: &str) -> PluginResult<Vec<ParsedSymbol>> {
        // Parse with tree-sitter and extract symbols
        Ok(vec![])
    }

    async fn extract_references(&self, source: &str) -> PluginResult<Vec<ParsedReference>> {
        // Parse with tree-sitter and extract references
        Ok(vec![])
    }

    fn tree_sitter_language(&self) -> *const () {
        extern "C" { fn tree_sitter_rust() -> Language; }
        unsafe { &tree_sitter_rust() as *const Language as *const () }
    }
}

#[no_mangle]
pub fn plugin_create() -> Box<dyn Plugin> {
    Box::new(RustAnalyzer)
}
```

---

## Notes

- Total plugins migrated: ~79 (including 9 adi-workflow language plugins)
- Phase 2 (i18n plugins) - COMPLETE, all 18 translation plugins use v3
- Phase 4 (Hive orchestration plugins) - COMPLETE, all 32 plugins use v3
- Phase 5 (Cleanup) - COMPLETE, v2 fallback paths removed from adi-cli
- Phase 6 (lib-plugin-host) - COMPLETE, v2 code entirely removed
- `hive-core` - Migrated to use v3 traits instead of orchestration ABI
- `lib-plugin-abi-orchestration` - DELETED (no longer used)
- `lib-plugin-abi` (v2) - **NO LONGER USED** - can be safely removed from workspace
- `lib-plugin-host` - Now pure v3, uses only PluginManagerV3
- `adi-cli` - Now uses only PluginManagerV3, no v2 PluginHost
- `adi-indexer/core` - Migrated to use PluginManagerV3 for language analyzers
- `lib-embed` - Migrated to use PluginManagerV3 for embedder plugins
- `adi-knowledgebase/core` - Migrated to use PluginManagerV3
- `lib-i18n-core` - Supports standalone mode, no longer requires ServiceRegistry
- Complex plugins (like hive-plugin): Already migrated as reference
- Language analyzer plugins: Use new `LanguageAnalyzer` trait (SERVICE_LANGUAGE_ANALYZER)
- Embedder plugins: Use new `Embedder` trait (SERVICE_EMBEDDER)
- Orchestration plugins: Use new v3 traits (Runner, HealthCheck, ProxyMiddleware, ObservabilitySink, RolloutStrategy)

## Migration Complete

**All v2 code has been removed from the codebase.**

The legacy `lib-plugin-abi` crate is no longer used by any crate in the workspace and can be safely deleted.
