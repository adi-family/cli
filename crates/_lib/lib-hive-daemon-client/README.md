# lib-hive-daemon-client

Rust client library for communicating with the Hive daemon via Unix socket.

## Overview

This library provides a high-level API for interacting with the Hive orchestrator daemon. It's used by:
- Core plugins to extend daemon functionality
- External tools to manage services
- CLI commands for user interaction

## Features

- ✅ **Async/await API** - Built on Tokio for efficient I/O
- ✅ **Type-safe requests** - Strongly-typed request/response enums
- ✅ **Connection management** - Automatic reconnection and lazy connection
- ✅ **Service lifecycle** - Create, start, stop, restart, delete services
- ✅ **Source management** - Add, remove, reload configuration sources
- ✅ **Log streaming** - Real-time log collection from services
- ✅ **Status queries** - Daemon and service health information

## Installation

```toml
[dependencies]
lib-hive-daemon-client = { path = "path/to/lib-hive-daemon-client" }
```

## Usage

### Basic Example

```rust
use lib_hive_daemon_client::DaemonClient;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Create client (default socket: ~/.adi/hive/hive.sock)
    let client = DaemonClient::new_default()?;

    // Check if daemon is running
    if !client.ping().await? {
        eprintln!("Daemon is not running");
        return Ok(());
    }

    // Get daemon status
    let status = client.status().await?;
    println!("Daemon version: {}", status.version);
    println!("Running services: {}/{}",
        status.running_services,
        status.total_services
    );

    // List all services
    let services = client.list_services(None).await?;
    for service in services {
        println!("{}: {:?}", service.fqn, service.state);
    }

    Ok(())
}
```

### Service Management

```rust
use lib_hive_daemon_client::DaemonClient;
use serde_json::json;

async fn manage_services() -> anyhow::Result<()> {
    let client = DaemonClient::new_default()?;

    // Create a new service dynamically (SQLite sources only)
    client.create_service(
        "cocoons".to_string(),
        "my-worker".to_string(),
        json!({
            "runner": {
                "type": "docker",
                "config": {
                    "image": "nginx:alpine"
                }
            },
            "rollout": {
                "type": "recreate",
                "config": {
                    "ports": { "http": 8080 }
                }
            }
        })
    ).await?;

    // Start the service
    client.start_service("cocoons:my-worker".to_string()).await?;

    // Check status
    let status = client.get_service_status(
        "cocoons:my-worker".to_string()
    ).await?;
    println!("Service state: {:?}", status.state);

    // Stop the service
    client.stop_service("cocoons:my-worker".to_string()).await?;

    // Delete the service
    client.delete_service("cocoons:my-worker".to_string()).await?;

    Ok(())
}
```

### Log Streaming

```rust
use lib_hive_daemon_client::DaemonClient;

async fn stream_logs() -> anyhow::Result<()> {
    let client = DaemonClient::new_default()?;

    // Get recent logs
    let logs = client.get_logs(
        Some("default:web".to_string()),
        Some(100),  // Last 100 lines
        None,       // No time filter
        Some("info".to_string()),  // Info level and above
    ).await?;

    for log in logs {
        println!("[{}] {}: {}",
            log.timestamp,
            log.level,
            log.message
        );
    }

    // Start streaming logs
    let stream = client.stream_logs(
        Some("default:*".to_string()),  // All services in 'default' source
        Some("warn".to_string()),        // Only warnings and errors
    ).await?;

    // ... handle log stream ...

    // Stop streaming
    stream.stop().await?;

    Ok(())
}
```

## Core Plugin Integration

Use this client in core plugins to extend daemon functionality:

```rust
use async_trait::async_trait;
use lib_hive_daemon_client::DaemonClient;
use std::sync::Arc;

pub struct MyPlugin {
    client: Option<Arc<DaemonClient>>,
}

#[async_trait]
impl CorePlugin for MyPlugin {
    fn name(&self) -> &str {
        "my-plugin"
    }

    async fn init(&mut self, client: Arc<DaemonClient>) -> anyhow::Result<()> {
        self.client = Some(client.clone());

        // Use client to interact with daemon
        let services = client.list_services(None).await?;
        println!("Found {} services", services.len());

        Ok(())
    }

    async fn on_event(&self, event: DaemonEvent) -> anyhow::Result<()> {
        match event {
            DaemonEvent::ServiceStarted { fqn } => {
                // React to service start
                if let Some(client) = &self.client {
                    let status = client.get_service_status(fqn).await?;
                    // ... do something with status ...
                }
            }
            _ => {}
        }
        Ok(())
    }
}
```

## API Reference

### DaemonClient Methods

| Method | Description |
|--------|-------------|
| `new(socket_path)` | Create client with custom socket path |
| `new_default()` | Create client with default socket path |
| `ping()` | Check if daemon is running |
| `status()` | Get daemon status information |
| `list_sources()` | List all configuration sources |
| `list_services(source)` | List services (optionally filtered by source) |
| `get_service_status(fqn)` | Get detailed service status |
| `create_service(source, name, config)` | Create new service (SQLite only) |
| `update_service(fqn, patch)` | Update service configuration |
| `delete_service(fqn)` | Delete a service |
| `start_service(fqn)` | Start a service |
| `stop_service(fqn)` | Stop a service |
| `restart_service(fqn)` | Restart a service |
| `get_logs(fqn, lines, since, level)` | Get historical logs |
| `stream_logs(fqn, level)` | Start streaming logs |
| `disconnect()` | Close connection to daemon |

### Types

See [src/lib.rs](src/lib.rs) for complete type definitions:
- `DaemonRequest` - All request types
- `DaemonResponse` - All response types
- `DaemonStatus` - Daemon health and statistics
- `ServiceStatus` - Service state and metadata
- `SourceInfo` - Configuration source details
- `LogLine` - Log entry structure

## Error Handling

All methods return `anyhow::Result<T>` and can fail if:
- Daemon is not running
- Unix socket connection fails
- Invalid request/response format
- Daemon returns error response

Example error handling:

```rust
match client.start_service("default:web".to_string()).await {
    Ok(_) => println!("Service started"),
    Err(e) if e.to_string().contains("not running") => {
        eprintln!("Daemon is not running. Start it with: adi hive start");
    }
    Err(e) => {
        eprintln!("Failed to start service: {}", e);
    }
}
```

## Testing

Run tests with:

```bash
cargo test
```

Integration tests require a running daemon:

```bash
# Start daemon in background
adi hive start

# Run integration tests
cargo test --test integration

# Stop daemon
adi hive stop
```

## License

BSL-1.0
