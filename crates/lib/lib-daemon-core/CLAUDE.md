daemon-management, cross-platform, service-manager, ipc, lifecycle, systemd, launchd

## Overview
- Generic daemon management library for Rust applications
- Cross-platform support: Linux (systemd), macOS (launchd), Windows (basic)
- Provides reusable infrastructure for building daemon processes
- Domain-agnostic, can be used by any application needing daemon functionality

## Key Features
- **DaemonBuilder**: High-level fluent API for daemon configuration and management
- **Service Integration**: systemd (Linux), launchd (macOS), with autostart support
- **Cross-platform IPC**: Unix sockets (Linux/macOS), Named pipes (Windows), TCP fallback
- **Background Spawning**: fork()+setsid() on Unix, CREATE_NEW_PROCESS_GROUP on Windows
- **PID File Management**: Check if running, write/remove PID files, auto-cleanup
- **Graceful Shutdown**: Shutdown coordinator with signal handling (SIGTERM, SIGINT)
- **Process Control**: Check process existence, kill processes, wait for exit

## Modules

| Module | Description |
|--------|-------------|
| `builder` | DaemonBuilder API - high-level daemon configuration |
| `service` | ServiceManager trait + systemd/launchd implementations |
| `platform` | Platform abstraction (spawn, process checks, kill) |
| `ipc_transport` | Cross-platform IPC (Unix sockets, Named pipes, TCP) |
| `config` | Daemon configuration (base dir, socket path, PID path) |
| `error` | Error types for daemon operations |
| `ipc` | IPC protocol traits and message codec |
| `lifecycle` | Shutdown coordinator and signal handlers |
| `pid` | PID file management |
| `socket` | Unix socket server/client utilities (legacy) |

## Quick Start (DaemonBuilder)

```rust
use lib_daemon_core::{DaemonBuilder, Result};

#[tokio::main]
async fn main() -> Result<()> {
    let daemon = DaemonBuilder::new("my-service")
        .executable("/usr/bin/my-service")
        .description("My background service")
        .working_dir("/var/lib/my-service")
        .log_file("/var/log/my-service.log")
        .env("RUST_LOG", "info")
        .autostart(true)
        .build()?;

    // Install as system service
    daemon.install().await?;
    daemon.start().await?;

    // Check status
    let status = daemon.status().await?;
    println!("Running: {}", status.is_running());

    // Or spawn directly (without service manager)
    // daemon.spawn().await?;

    Ok(())
}
```

## Service Manager API

```rust
use lib_daemon_core::{get_service_manager, ServiceConfig, RestartPolicy};

#[tokio::main]
async fn main() -> Result<()> {
    let manager = get_service_manager(); // Auto-detects platform

    let config = ServiceConfig::new("my-daemon", "/usr/bin/my-daemon")
        .description("My daemon service")
        .args(["--config", "/etc/my-daemon.conf"])
        .env("RUST_LOG", "info")
        .restart_policy(RestartPolicy::OnFailure)
        .autostart(true);

    // Install, start, manage
    manager.install(&config).await?;
    manager.start("my-daemon").await?;
    manager.enable_autostart("my-daemon").await?;

    // Query status
    let status = manager.status("my-daemon").await?;
    let logs = manager.logs("my-daemon", 100).await?;

    Ok(())
}
```

## Cross-Platform IPC

```rust
use lib_daemon_core::{IpcServer, IpcClient, IpcEndpoint};

// Server side
let endpoint = IpcEndpoint::for_path("/var/run/my.sock"); // Auto-selects transport
let server = IpcServer::bind(endpoint).await?;
let stream = server.accept().await?;

// Client side  
let client = IpcClient::for_path("/var/run/my.sock");
let response: MyResponse = client.request(&MyRequest::Status).await?;
```

## Background Process Spawning

```rust
use lib_daemon_core::{spawn_background, SpawnConfig, is_process_running, kill_process};

let config = SpawnConfig::new("/usr/bin/my-daemon")
    .args(["--flag"])
    .working_dir("/var/lib")
    .env("RUST_LOG", "info")
    .stdout("/var/log/out.log")
    .stderr("/var/log/err.log")
    .pid_file("/var/run/my.pid");

let pid = spawn_background(&config)?;

// Later...
if is_process_running(pid) {
    kill_process(pid)?;
}
```

## Platform Support Matrix

| Feature | Linux | macOS | Windows |
|---------|-------|-------|---------|
| DaemonBuilder | ✅ | ✅ | ✅ |
| Background spawn | ✅ fork+setsid | ✅ fork+setsid | ✅ CREATE_NEW_PROCESS_GROUP |
| IPC | ✅ Unix socket | ✅ Unix socket | ✅ Named pipes |
| Service install | ✅ systemd | ✅ launchd | 🔶 stub |
| Autostart | ✅ systemctl enable | ✅ RunAtLoad | 🔶 stub |
| PID checking | ✅ kill(0) | ✅ kill(0) | ✅ OpenProcess |
| Graceful shutdown | ✅ SIGTERM | ✅ SIGTERM | ✅ TerminateProcess |

## Key Types

| Type | Description |
|------|-------------|
| `DaemonBuilder` | Fluent builder for daemon configuration |
| `Daemon` | Handle for controlling a configured daemon |
| `DaemonStatus` | Status info (state, PID, paths, autostart) |
| `ServiceManager` | Trait for platform service managers |
| `SystemdManager` | systemd implementation (Linux) |
| `LaunchdManager` | launchd implementation (macOS) |
| `ServiceConfig` | Service installation configuration |
| `ServiceStatus` | Service state enum (Running, Stopped, etc.) |
| `RestartPolicy` | Restart behavior (Never, OnFailure, Always) |
| `IpcServer` / `IpcClient` | Cross-platform IPC |
| `IpcEndpoint` | IPC endpoint (UnixSocket, NamedPipe, Tcp) |
| `SpawnConfig` | Configuration for background process spawning |
| `Platform` | Platform detection enum |

## Used By
- `adi-hive` - Hive orchestrator daemon
- `adi-cocoon` - Cocoon container service
- Future: Any ADI component needing daemon functionality

## Design Principles
- **Cross-platform**: Unified API works on Linux, macOS, Windows
- **Layered**: High-level DaemonBuilder or low-level primitives
- **Type-safe**: Rust's type system for protocol definitions
- **Async-first**: Built on tokio for async I/O
- **Auto-cleanup**: Resources cleaned up automatically (RAII)
