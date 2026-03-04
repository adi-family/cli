# lib-daemon-core

Generic daemon management library for Rust applications. Provides reusable infrastructure for building daemon processes with PID files, Unix socket IPC, and graceful shutdown coordination.

## Features

- **PID File Management**: Check if daemon is running, write/remove PID files, auto-cleanup of stale files
- **Unix Socket IPC**: Server and client utilities for request/response communication
- **Protocol Framework**: Generic traits for defining daemon request/response protocols
- **Graceful Shutdown**: Shutdown coordinator with signal handling (SIGTERM, SIGINT, Ctrl+C)
- **Auto-cleanup**: Stale PID file removal, socket cleanup on drop (RAII)
- **Type-safe**: Use Rust's type system for protocol definitions
- **Async-first**: Built on tokio for async I/O

## Quick Start

```rust
use lib_daemon_core::{
    DaemonConfig, PidFile, UnixSocketServer, DaemonProtocol,
    ShutdownCoordinator,
};
use serde::{Serialize, Deserialize};
use anyhow::Result;

// Define your protocol
#[derive(Debug, Clone, Serialize, Deserialize)]
enum MyRequest {
    Status,
    Shutdown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum MyResponse {
    Ok { message: String },
    Error { message: String },
}

impl DaemonProtocol for MyRequest {
    type Response = MyResponse;
}

#[tokio::main]
async fn main() -> Result<()> {
    let config = DaemonConfig::new("/var/run/mydaemon")
        .with_socket_name("my.sock")
        .with_pid_name("my.pid");

    // Check if already running
    let pid_file = PidFile::new(config.pid_path());
    pid_file.ensure_not_running()?;

    // Write PID file
    pid_file.write()?;

    // Start Unix socket server
    let server = UnixSocketServer::bind(config.socket_path()).await?;

    // Setup graceful shutdown
    let mut coordinator = ShutdownCoordinator::new();

    // Accept connections...
    loop {
        tokio::select! {
            result = server.accept() => {
                // Handle connection
            }
            _ = coordinator.wait() => {
                break;
            }
        }
    }

    Ok(())
}
```

## Client Example

```rust
use lib_daemon_core::UnixSocketClient;

#[tokio::main]
async fn main() -> Result<()> {
    let client = UnixSocketClient::new("/var/run/mydaemon/my.sock");

    if !client.is_available() {
        println!("Daemon is not running");
        return Ok(());
    }

    let request = MyRequest::Status;
    let response: MyResponse = client.send(&request).await?;

    println!("Response: {:?}", response);
    Ok(())
}
```

## Modules

| Module | Description |
|--------|-------------|
| `config` | Daemon configuration (base dir, socket path, PID path) |
| `error` | Error types for daemon operations |
| `ipc` | IPC protocol traits and message codec |
| `lifecycle` | Shutdown coordinator and signal handlers |
| `pid` | PID file management |
| `socket` | Unix socket server/client utilities |

## Platform Support

- **Linux**: Full support (PID checks, signal handling)
- **macOS**: Full support (PID checks, signal handling)
- **Windows**: Limited (no Unix sockets, Ctrl+C only)

## Used By

- `adi-hive-core` - Hive orchestrator daemon
- Future: Any ADI component needing daemon functionality

## License

BSL-1.0
