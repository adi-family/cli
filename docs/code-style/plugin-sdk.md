# Plugin SDK

Use `lib-plugin-prelude` for all ADI plugins. It re-exports everything from `lib-plugin-abi-v3` plus SDK utilities.

## 1. Plugin Definition

### Cargo.toml

```toml
[package]
name = "tasks-plugin"
version = "1.0.0"
edition = "2021"
license = "BSL-1.0"
description = "Task management with dependencies"
authors = ["ADI Team"]

[lib]
crate-type = ["cdylib"]

[dependencies]
lib-plugin-prelude = { path = "../../lib/lib-plugin-prelude" }

[package.metadata.plugin]
id = "adi.tasks"
name = "ADI Tasks"
type = "core"  # or "extension"

[package.metadata.plugin.compatibility]
min_host_version = "0.9.0"

[package.metadata.plugin.cli]
command = "tasks"
description = "Task management"
aliases = ["t"]

[[package.metadata.plugin.provides]]
id = "adi.tasks.cli"
version = "1.0.0"
description = "CLI commands for task management"

[package.metadata.plugin.tags]
categories = ["tasks", "project-management"]
```

### Plugin Code

```rust
use lib_plugin_prelude::*;

pub struct TasksPlugin {
    db: Database,
}

#[async_trait]
impl Plugin for TasksPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata::new("adi.tasks", t!("plugin-name"), "1.0.0")
            .with_type(PluginType::Core)
            .with_author("ADI Team")
            .with_description(t!("plugin-description"))
    }

    async fn init(&mut self, ctx: &PluginContext) -> Result<()> {
        self.db = Database::open(ctx.data_dir.clone())?;
        Ok(())
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    fn provides(&self) -> Vec<&'static str> {
        vec![SERVICE_CLI_COMMANDS]
    }
}

// Entry point (required)
#[no_mangle]
pub fn plugin_create() -> Box<dyn Plugin> {
    Box::new(TasksPlugin::new())
}
```

---

## 2. Translations

Files in `locales/<locale>/messages.ftl`:

```
locales/
├── en-US/messages.ftl   # fallback (required)
├── zh-CN/messages.ftl
├── uk-UA/messages.ftl
└── de-DE/messages.ftl
```

### English (en-US)

```ftl
plugin-name = Tasks
plugin-description = Task management with dependencies

cmd-list-help = List all tasks
cmd-add-help = Add a new task
error-not-found = Task { $id } not found
```

### Chinese (zh-CN)

```ftl
plugin-name = 任务
plugin-description = 带依赖关系的任务管理

cmd-list-help = 列出所有任务
cmd-add-help = 添加新任务
error-not-found = 找不到任务 { $id }
```

### Usage

```rust
use lib_plugin_prelude::*;

// Simple
let msg = t!("plugin-name");

// With arguments
let msg = t!("error-not-found", "id" => task_id.to_string());

// Multiple arguments
let msg = t!("greeting", "name" => name, "count" => count.to_string());
```

---

## 3. CLI Commands with Typed Arguments

CLI commands use typed argument structs for:
- **Schema generation** - AI agents can discover commands via `adi --schema`
- **Type-safe parsing** - Arguments are parsed into Rust types automatically
- **Validation** - Required fields and type errors are handled automatically

### Defining Argument Structs

```rust
use lib_plugin_prelude::*;

/// Arguments for the `list` command
#[derive(CliArgs)]
pub struct ListArgs {
    /// Filter by status (optional flag)
    #[arg(long)]
    pub status: Option<String>,

    /// Show only ready tasks (boolean flag)
    #[arg(long)]
    pub ready: bool,

    /// Output format with default value
    #[arg(long, default = "text".to_string())]
    pub format: String,

    /// Limit results (optional with type conversion)
    #[arg(long)]
    pub limit: Option<i64>,
}

/// Arguments for the `add` command  
#[derive(CliArgs)]
pub struct AddArgs {
    /// Task title (required positional argument)
    #[arg(position = 0)]
    pub title: String,

    /// Task description (optional flag)
    #[arg(long)]
    pub description: Option<String>,

    /// Dependencies (custom long name)
    #[arg(long = "depends-on")]
    pub depends_on: Option<String>,
}
```

### Argument Attributes

| Attribute | Description | Example |
|-----------|-------------|---------|
| `#[arg(long)]` | Long flag `--name` | `--status todo` |
| `#[arg(long = "custom")]` | Custom flag name | `--depends-on 1,2` |
| `#[arg(short = 'n')]` | Short flag `-n` | `-s todo` |
| `#[arg(position = 0)]` | Positional argument | `add "my task"` |
| `#[arg(default = expr)]` | Default value | `default = 10` |

### Type Mapping

| Rust Type | CLI Type | Required | Example |
|-----------|----------|----------|---------|
| `String` | String | yes | `--name foo` |
| `Option<String>` | String | no | `--name foo` or omit |
| `i64` | Int | yes | `--limit 10` |
| `Option<i64>` | Int | no | `--limit 10` or omit |
| `bool` | Bool | no (flag) | `--verbose` |
| `f64` | Float | yes | `--rate 0.5` |

### Implementing Commands

```rust
impl TasksPlugin {
    /// List all tasks
    #[command(name = "list", description = "cmd-list-help")]
    async fn list(&self, args: ListArgs) -> CmdResult {
        // args.status, args.ready, args.format, args.limit - all typed!
        let tasks = if args.ready {
            self.db.get_ready()?
        } else if let Some(status) = &args.status {
            self.db.get_by_status(status)?
        } else {
            self.db.list()?
        };

        if args.format == "json" {
            Ok(serde_json::to_string_pretty(&tasks)?)
        } else {
            Ok(format_tasks_text(&tasks))
        }
    }

    /// Add a new task
    #[command(name = "add", description = "cmd-add-help")]
    async fn add(&self, args: AddArgs) -> CmdResult {
        let id = self.db.create(&args.title, args.description)?;
        Ok(t!("task-created", "id" => id.to_string()))
    }

    /// Command with no arguments
    #[command(name = "stats", description = "cmd-stats-help")]
    async fn stats(&self) -> CmdResult {
        let stats = self.db.get_stats()?;
        Ok(format!("Total: {}", stats.total))
    }
}
```

### Registering Commands

The `#[command]` macro generates:
- `__sdk_cmd_meta_<fn>()` - Returns `CliCommand` with schema
- `__sdk_cmd_handler_<fn>(ctx)` - Parses args and calls function

```rust
#[async_trait]
impl CliCommands for TasksPlugin {
    async fn list_commands(&self) -> Vec<CliCommand> {
        vec![
            Self::__sdk_cmd_meta_list(),
            Self::__sdk_cmd_meta_add(),
            Self::__sdk_cmd_meta_stats(),
        ]
    }

    async fn run_command(&self, ctx: &CliContext) -> Result<CliResult> {
        match ctx.subcommand.as_deref() {
            Some("list") => self.__sdk_cmd_handler_list(ctx).await,
            Some("add") => self.__sdk_cmd_handler_add(ctx).await,
            Some("stats") => self.__sdk_cmd_handler_stats(ctx).await,
            _ => Ok(CliResult::error("Unknown command")),
        }
    }
}
```

### Generated Schema

The schema is available for AI agents via `adi --schema`:

```json
{
  "commands": [{
    "name": "tasks",
    "commands": [{
      "name": "list",
      "description": "List all tasks",
      "args": [
        {"name": "--status", "type": "string", "required": false},
        {"name": "--ready", "type": "bool", "required": false},
        {"name": "--format", "type": "string", "required": false},
        {"name": "--limit", "type": "int", "required": false}
      ]
    }, {
      "name": "add",
      "description": "Add a new task",
      "args": [
        {"name": "title", "type": "string", "required": true, "position": 0},
        {"name": "--description", "type": "string", "required": false},
        {"name": "--depends-on", "type": "string", "required": false}
      ]
    }]
  }]
}
```

### Error Handling

Parse errors provide clear messages:

```
$ adi tasks add
error: Missing required argument: title

$ adi tasks list --limit abc
error: Invalid value for --limit
```
```

### Global Commands (`adi <cmd>`)

Register commands directly on CLI root (e.g., `adi up` instead of `adi hive up`):

```rust
#[async_trait]
impl GlobalCommands for HivePlugin {
    async fn list_global_commands(&self) -> Vec<CliCommand> {
        vec![
            CliCommand {
                name: "up".to_string(),
                description: "Start services".to_string(),
                usage: "up [service]".to_string(),
                has_subcommands: false,
            },
            CliCommand {
                name: "down".to_string(),
                description: "Stop services".to_string(),
                usage: "down [service]".to_string(),
                has_subcommands: false,
            },
        ]
    }

    async fn run_global_command(&self, ctx: &CliContext) -> Result<CliResult> {
        match ctx.command.as_str() {
            "up" => self.cmd_up(ctx).await,
            "down" => self.cmd_down(ctx).await,
            _ => Ok(CliResult::error("Unknown command")),
        }
    }
}
```

---

## 4. HTTP Server

```rust
#[async_trait]
impl HttpRoutes for TasksPlugin {
    async fn list_routes(&self) -> Vec<HttpRoute> {
        vec![
            HttpRoute {
                method: HttpMethod::Get,
                path: "/tasks".to_string(),
                handler_id: "list_tasks".to_string(),
                description: "List all tasks".to_string(),
            },
            HttpRoute {
                method: HttpMethod::Post,
                path: "/tasks".to_string(),
                handler_id: "create_task".to_string(),
                description: "Create a task".to_string(),
            },
        ]
    }

    async fn handle_request(&self, req: HttpRequest) -> Result<HttpResponse> {
        match req.handler_id.as_str() {
            "list_tasks" => {
                let tasks = self.db.list(None)?;
                HttpResponse::json(&tasks)
            }
            "create_task" => {
                let input: CreateTask = req.json()?;
                let task = self.db.create(input)?;
                HttpResponse::json(&task)
            }
            _ => Ok(HttpResponse::error(StatusCode::NOT_FOUND, "Not found")),
        }
    }
}
```

- Port assigned by host at runtime via `ctx.http_port()`

---

## 5. WebRTC Connection

Signaling handled by CLI core. Plugin only defines message handlers.

```rust
#[async_trait]
impl WebRtcHandlers for TasksPlugin {
    async fn on_connect(&self, peer: Peer) -> Result<()> {
        println!("Peer connected: {}", peer.id);
        Ok(())
    }
    
    async fn on_message(&self, peer: Peer, msg: Message) -> Result<()> {
        let data: TaskUpdate = msg.parse_json()?;
        self.handle_update(data).await?;
        Ok(())
    }
    
    async fn on_disconnect(&self, peer: Peer) -> Result<()> {
        println!("Peer disconnected: {}", peer.id);
        Ok(())
    }
}
```

---

## 6. Daemon Commands

Execute privileged commands via daemon:

```rust
impl TasksPlugin {
    // Regular (user-level)
    async fn install_deps(&self, ctx: &PluginContext) -> Result<()> {
        ctx.daemon().exec(DaemonCommand::regular("brew install ffmpeg")).await?;
        Ok(())
    }

    // Sudo (root-level)
    async fn restart_service(&self, ctx: &PluginContext) -> Result<()> {
        ctx.daemon().exec(DaemonCommand::sudo("systemctl restart nginx")).await?;
        Ok(())
    }
}
```

User sees permission prompt on install:

```
Plugin "adi.tasks" requires these permissions:

  Regular commands:
    - brew install ffmpeg

  Sudo commands (root):
    - systemctl restart nginx

  [Install] [Cancel]
```

---

## 7. Daemon Service

Long-running background service:

```rust
#[async_trait]
impl DaemonService for TasksPlugin {
    async fn start(&self, ctx: DaemonContext) -> Result<()> {
        loop {
            self.process_queue().await?;
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    }
    
    async fn stop(&self) -> Result<()> {
        self.cleanup().await
    }
    
    async fn status(&self) -> ServiceStatus {
        ServiceStatus::Running
    }
}
```

Managed by `adi daemon start/stop/status`.

---

## Available Traits

| Trait | Purpose | Service ID |
|-------|---------|------------|
| `Plugin` | Base trait (required) | - |
| `CliCommands` | Plugin CLI commands | `cli.commands` |
| `GlobalCommands` | Root CLI commands | `cli.global` |
| `HttpRoutes` | HTTP server | `http.routes` |
| `WebRtcHandlers` | WebRTC peer handling | `webrtc.handlers` |
| `DaemonService` | Background service | `daemon.service` |
| `LanguageAnalyzer` | Indexer language plugin | `indexer.lang` |
| `Embedder` | Text embedding | `indexer.embed` |
| `LogProvider` | Log streaming | `logs.provider` |

---

## Crates

| Crate | Purpose |
|-------|---------|
| `lib-plugin-prelude` | One-stop import for plugins |
| `lib-plugin-abi-v3` | Runtime types and traits |
| `lib-plugin-sdk` | Procedural macros (internal) |
| `lib-plugin-host` | Plugin loading (host side) |
| `lib-plugin-manifest` | Cargo.toml metadata parsing |

---

## Building

```bash
# Build and install locally
adi wf build-plugin

# Build for release
adi wf release-plugin
```

Plugins compile to `cdylib` and are loaded by `lib-plugin-host`.
