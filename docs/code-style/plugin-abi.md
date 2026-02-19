# Plugin ABI v3

**Use `lib-plugin-abi-v3` for all ADI plugins.**

## Overview

Unified plugin ABI using native Rust async traits (no FFI complexity).

## Traits

| Trait | Purpose |
|-------|---------|
| `Plugin` | Base trait - metadata, init, shutdown |
| `CliCommands` | CLI plugin commands |
| `HttpService` | HTTP server plugin |
| `McpServer` | MCP (Model Context Protocol) server |
| `LangAnalyzer` | Language analyzer for indexer |
| `EmbedProvider` | Text embedding provider |
| `LogsProvider` | Logging provider |
| `ProxyProvider` | LLM proxy provider |

## Basic Plugin

```rust
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
            description: Some("Does things".to_string()),
        }
    }

    async fn init(&mut self, ctx: &PluginContext) -> Result<()> {
        Ok(())
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }
}

// Export plugin
#[no_mangle]
pub fn plugin_create() -> Box<dyn Plugin> {
    Box::new(MyPlugin)
}
```

## CLI Plugin

```rust
#[async_trait]
impl CliCommands for MyPlugin {
    async fn list_commands(&self) -> Vec<CliCommand> {
        vec![CliCommand {
            name: "hello".to_string(),
            description: "Say hello".to_string(),
            usage: "myplugin hello".to_string(),
            has_subcommands: false,
        }]
    }

    async fn run_command(&self, ctx: &CliContext) -> Result<CliResult> {
        Ok(CliResult::success("Hello!"))
    }
}
```

## Plugin Cargo.toml

Plugin metadata is defined in `Cargo.toml` under `[package.metadata.plugin]`:

```toml
[package]
name = "myplugin-plugin"
version = "1.0.0"
edition = "2021"
license = "MIT"
description = "My Plugin - does things"
authors = ["Me"]

[lib]
crate-type = ["cdylib"]

[dependencies]
lib-plugin-abi-v3 = { path = "../../lib/lib-plugin-abi-v3" }
lib-console-output = { path = "../../lib/lib-console-output" }
async-trait = "0.1"
tokio = { version = "1.0", features = ["full"] }

[package.metadata.plugin]
id = "adi.myplugin"
name = "My Plugin"
type = "core"  # or "extension"

[package.metadata.plugin.compatibility]
api_version = 3
min_host_version = "0.9.0"

[package.metadata.plugin.cli]
command = "myplugin"
description = "Does things"
aliases = ["mp"]

[[package.metadata.plugin.provides]]
id = "adi.myplugin.cli"
version = "1.0.0"
description = "CLI commands"

[package.metadata.plugin.tags]
categories = ["utility"]
```

## Building

Use `adi wf build-plugin` to build and install locally.

Plugins compile to `cdylib` and are loaded by `lib-plugin-host`.
