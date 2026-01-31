# lib-plugin-abi-orchestration

Shared ABI definitions for orchestration plugins across different orchestrators.

## Overview

This library provides stable trait definitions for common orchestration concerns:

- **Runners**: Execute services (Docker, scripts, compose, etc.)
- **Environment**: Load and provide environment variables
- **Health**: Check service readiness (HTTP, TCP, database, etc.)
- **Proxy**: HTTP middleware (CORS, rate limiting, auth, etc.)
- **Observability**: Logging, metrics, and monitoring
- **Rollout**: Deployment strategies (recreate, blue-green, etc.)

## Plugin Categories

| Category | Trait | Description |
|----------|-------|-------------|
| Runner | `RunnerPlugin` | Execute services (script, docker, etc.) |
| Env | `EnvPlugin` | Provide environment variables |
| Health | `HealthPlugin` | Check service readiness |
| Proxy | `ProxyPlugin` | Middleware for HTTP proxy |
| Obs | `ObsPlugin` | Observability (logging, metrics) |
| Rollout | `RolloutPlugin` | Deployment strategies |

## Usage

### For Orchestrators (like Hive)

Depend on this crate to define your plugin contracts:

```toml
[dependencies]
lib-plugin-abi-orchestration = { path = "../lib/lib-plugin-abi-orchestration" }
```

### For Plugin Authors

Implement the relevant trait for your plugin:

```rust
use lib_plugin_abi_orchestration::{RunnerPlugin, PluginMetadata, PluginCategory};
use async_trait::async_trait;

pub struct MyDockerRunner;

#[async_trait]
impl RunnerPlugin for MyDockerRunner {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            id: "hive.runner.docker".to_string(),
            name: "docker".to_string(),
            version: "0.1.0".to_string(),
            description: "Docker container runner".to_string(),
            category: PluginCategory::Runner,
        }
    }

    async fn start(&self, config: serde_json::Value) -> anyhow::Result<ProcessHandle> {
        // Implementation
    }

    // ... other trait methods
}
```

## Plugin IDs

Plugins are identified by their plugin ID following the pattern:
`<orchestrator>.<category>.<name>`

Examples:
- `hive.runner.docker`
- `hive.obs.stdout`
- `hive.health.http`

## Lifecycle Hooks

Runner plugins can also execute one-shot tasks for lifecycle hooks (pre-up, post-up, pre-down, post-down).
See the `hooks` module for hook types and the `HookExecutor`.

## Compatibility

This library is used by:
- **Hive**: Cocoon orchestration system (via `hive-plugin-abi` wrapper)
- Future orchestrators can adopt the same ABI for plugin compatibility

## Design Principles

1. **Stability**: ABI changes are breaking changes
2. **Composability**: Plugins can be mixed and matched
3. **Async-first**: All operations use async/await
4. **Error handling**: Uses `anyhow::Result` for flexibility
5. **Serialization**: Config uses `serde_json::Value` for flexibility
