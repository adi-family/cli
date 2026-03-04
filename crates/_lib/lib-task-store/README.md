# lib-task-store

Device-owned task storage abstraction for hybrid cloud deployment.

## Overview

Provides a backend-agnostic interface for task storage, allowing each device to choose its own backend (SQLite, PostgreSQL, etc.) while maintaining a consistent protocol for querying and aggregation across devices.

## Features

- **Backend Abstraction**: Common `TaskStore` trait for all backends
- **SQLite Support**: Lightweight local storage (perfect for laptops, edge devices)
- **PostgreSQL Support**: Scalable cloud storage (perfect for servers)
- **Device Autonomy**: Each device chooses its own storage backend
- **Protocol Consistency**: All backends respond with the same data format

## Usage

```rust
use lib_task_store::{TaskStoreBackend, create_task_store, CreateTask};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Device A: Uses SQLite (laptop)
    let store_a = create_task_store(TaskStoreBackend::Sqlite {
        path: PathBuf::from("~/.local/share/adi/tasks.db"),
    }).await?;

    // Device B: Uses PostgreSQL (server)
    let store_b = create_task_store(TaskStoreBackend::Postgres {
        url: "postgres://user:pass@localhost/adi_tasks".to_string(),
    }).await?;

    // Both use the same interface
    let task = CreateTask {
        title: "Build project".to_string(),
        description: Some("Run cargo build".to_string()),
        command: Some("cargo build --release".to_string()),
        input: serde_json::json!({}),
    };

    let created = store_a.create_task(task).await?;
    println!("Created task: {}", created.id);

    Ok(())
}
```

## Environment Configuration

```bash
# SQLite (local)
TASK_STORE_BACKEND=sqlite
TASK_STORE_PATH=~/.local/share/adi/tasks.db

# PostgreSQL (cloud)
TASK_STORE_BACKEND=postgres
TASK_STORE_URL=postgres://user:pass@localhost/adi_tasks
```

## Hybrid Cloud Architecture

In the hybrid cloud model:

1. **Laptop** (SQLite): Fast local storage, works offline
2. **Cloud Server** (PostgreSQL): Persistent, always-on storage
3. **GPU Box** (SQLite): Isolated task execution

All devices respond to the same query protocol, but choose their own backend. The signaling server sees only the protocol responses, not the implementation details.

## License

BSL-1.0
