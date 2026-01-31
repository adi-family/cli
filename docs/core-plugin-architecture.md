# Core Plugin Architecture

## Overview

The Hive daemon now supports **core plugins** that extend the orchestrator itself. Core plugins receive a `DaemonClient` (Unix socket client) to interact with the daemon programmatically.

```
┌─────────────────────────────────────────────────────────────┐
│                      Hive Daemon                            │
│                                                              │
│  ┌────────────────────────────────────────────────┐         │
│  │         Unix Socket Server                     │         │
│  │         ~/.adi/hive/hive.sock                  │         │
│  └───────────────────┬────────────────────────────┘         │
│                      │                                      │
│  ┌───────────────────▼────────────────────────────┐         │
│  │    Source Manager + Service Manager            │         │
│  │    (Core orchestration)                        │         │
│  └────────────────────────────────────────────────┘         │
└─────────────────────────────────────────────────────────────┘
                      ▲
                      │ DaemonClient (Unix socket IPC)
                      │
┌─────────────────────┴───────────────────────────────────────┐
│                Core Plugin System                           │
│                                                              │
│  ┌────────────────────────────────────────────────┐         │
│  │  SignalingControlPlugin                        │         │
│  │  - WebSocket → Daemon translator               │         │
│  │  - Cocoon spawning via service manager         │         │
│  └────────────────────────────────────────────────┘         │
│                                                              │
│  ┌────────────────────────────────────────────────┐         │
│  │  AutoscalerPlugin                              │         │
│  │  - CPU/memory monitoring                       │         │
│  │  - Auto scale up/down                          │         │
│  └────────────────────────────────────────────────┘         │
│                                                              │
│  ┌────────────────────────────────────────────────┐         │
│  │  BackupPlugin                                  │         │
│  │  - Periodic state snapshots                    │         │
│  │  - Service config backups                      │         │
│  └────────────────────────────────────────────────┘         │
└─────────────────────────────────────────────────────────────┘
```

## Components

### 1. lib-hive-daemon-client

**Location**: `crates/lib/lib-hive-daemon-client/`

**Purpose**: Rust client library for communicating with the Hive daemon via Unix socket.

**Key Features**:
- Type-safe request/response API
- Async/await with Tokio
- Service lifecycle management (create, start, stop, delete)
- Source management (add, remove, reload)
- Log streaming
- Status queries

**Example**:
```rust
use lib_hive_daemon_client::DaemonClient;

let client = DaemonClient::new_default()?;

// Create service
client.create_service(
    "cocoons".to_string(),
    "worker-123".to_string(),
    service_config_json,
).await?;

// Start service
client.start_service("cocoons:worker-123".to_string()).await?;
```

### 2. CorePlugin Trait

**Location**: `crates/hive/core/src/core_plugins.rs`

**Purpose**: Trait for plugins that extend the daemon core.

**Methods**:
```rust
#[async_trait]
pub trait CorePlugin: Send + Sync {
    fn name(&self) -> &str;
    fn version(&self) -> &str;

    async fn init(&mut self, client: Arc<DaemonClient>) -> Result<()>;
    async fn shutdown(&mut self) -> Result<()>;
    async fn on_event(&self, event: DaemonEvent) -> Result<()>;
}
```

**Events**:
- `DaemonStarted` - Daemon initialized
- `DaemonShutdown` - Daemon shutting down
- `SourceAdded` - Configuration source added
- `ServiceStarted` - Service started
- `ServiceStopped` - Service stopped
- `ServiceCrashed` - Service crashed
- `ServiceHealthChanged` - Health status changed

### 3. CorePluginRegistry

**Location**: `crates/hive/core/src/core_plugins.rs`

**Purpose**: Manages core plugin lifecycle.

**Usage**:
```rust
let mut registry = CorePluginRegistry::new();

// Register plugins
registry.register(SignalingControlPlugin::new(config));
registry.register(AutoscalerPlugin::new(autoscaler_config));

// Initialize all plugins
registry.init_all(daemon_client).await?;

// Broadcast events
registry.broadcast_event(DaemonEvent::ServiceStarted {
    fqn: "default:web".to_string(),
}).await;

// Shutdown
registry.shutdown_all().await?;
```

## Built-in Core Plugins

### SignalingControlPlugin

**Purpose**: Translates WebSocket/WebRTC cocoon spawn requests into daemon service operations.

**Flow**:
1. Connects to signaling server as WebSocket client
2. Receives `SpawnCocoon` message
3. Translates to `DaemonRequest::CreateService`
4. Sends via Unix socket to daemon
5. Daemon creates service in "cocoons" SQLite source
6. Service manager starts service (docker/script/podman runner)
7. Plugin sends `SpawnCocoonResult` back via WebSocket

**Benefits**:
- Cocoons use unified orchestration (same as local services)
- Cocoons get HTTP proxy, health checks, observability
- Supports ANY runner (not just Docker!)
- No code duplication

**Configuration**:
```bash
# Use script runner for native cocoons
COCOON_RUNNER_TYPE=script
COCOON_SCRIPT_PATH=/usr/local/bin/cocoon-worker

# Use docker runner (default)
COCOON_RUNNER_TYPE=docker
COCOON_REGISTRY=git.the-ihor.com/adi
```

### AutoscalerPlugin

**Purpose**: Auto-scales services based on CPU/memory metrics.

**Features**:
- Monitors service resource usage
- Scales up at high utilization
- Scales down at low utilization
- Cooldown period to prevent flapping
- Min/max instance limits

**Configuration**:
```rust
AutoscalerConfig {
    check_interval_secs: 30,
    cpu_scale_up_threshold: 0.80,    // 80% CPU
    cpu_scale_down_threshold: 0.20,  // 20% CPU
    min_instances: 1,
    max_instances: 10,
}
```

## Creating Custom Plugins

### Example: Backup Plugin

```rust
use async_trait::async_trait;
use lib_hive_daemon_client::DaemonClient;
use std::sync::Arc;

pub struct BackupPlugin {
    daemon_client: Option<Arc<DaemonClient>>,
    backup_dir: PathBuf,
}

impl BackupPlugin {
    pub fn new(backup_dir: PathBuf) -> Self {
        Self {
            daemon_client: None,
            backup_dir,
        }
    }

    async fn backup_all_services(&self) -> Result<()> {
        let client = self.daemon_client.as_ref().unwrap();

        // Get all sources
        let sources = client.list_sources().await?;

        for source in sources {
            // Get all services in source
            let services = client.list_services(
                Some(source.name.clone())
            ).await?;

            // Save service configs to backup dir
            for service in services {
                let backup_path = self.backup_dir
                    .join(&source.name)
                    .join(format!("{}.json", service.name));

                // Get service details and save
                // ...
            }
        }

        Ok(())
    }
}

#[async_trait]
impl CorePlugin for BackupPlugin {
    fn name(&self) -> &str {
        "backup"
    }

    async fn init(&mut self, client: Arc<DaemonClient>) -> Result<()> {
        self.daemon_client = Some(client);

        // Schedule periodic backups
        let plugin_clone = self.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(
                std::time::Duration::from_secs(3600)  // Hourly
            );

            loop {
                interval.tick().await;
                if let Err(e) = plugin_clone.backup_all_services().await {
                    error!("Backup failed: {}", e);
                }
            }
        });

        Ok(())
    }

    async fn on_event(&self, event: DaemonEvent) -> Result<()> {
        match event {
            DaemonEvent::ServiceCrashed { fqn, .. } => {
                // Trigger immediate backup on crash
                info!("Service {} crashed - triggering backup", fqn);
                self.backup_all_services().await?;
            }
            _ => {}
        }
        Ok(())
    }
}
```

## Integration with Daemon

```rust
// In daemon initialization (http/src/main.rs)

use adi_hive_core::core_plugins::{
    CorePluginRegistry,
    SignalingControlPlugin,
    AutoscalerPlugin,
};

#[tokio::main]
async fn main() -> Result<()> {
    // ... daemon initialization ...

    // Create daemon client
    let daemon_client = Arc::new(DaemonClient::new(socket_path)?);

    // Create plugin registry
    let mut plugins = CorePluginRegistry::new();

    // Register plugins
    plugins.register(SignalingControlPlugin::new(config.clone()));

    if config.enable_autoscaling {
        plugins.register(AutoscalerPlugin::new(
            AutoscalerConfig::default()
        ));
    }

    // Initialize plugins
    plugins.init_all(daemon_client).await?;

    // ... start daemon services ...

    // Broadcast startup event
    plugins.broadcast_event(DaemonEvent::DaemonStarted).await;

    // ... run daemon ...

    // On shutdown
    plugins.broadcast_event(DaemonEvent::DaemonShutdown).await;
    plugins.shutdown_all().await?;

    Ok(())
}
```

## Benefits

### 1. Unified Orchestration

**Before**:
- Local services: ServiceManager → Runners
- Remote cocoons: SignalingClient → DockerManager

**After**:
- Local services: ServiceManager → Runners
- Remote cocoons: SignalingControlPlugin → DaemonClient → ServiceManager → Runners

### 2. Runner Flexibility

Cocoons can now use **any runner**:
- `script` - Native processes
- `docker` - Docker containers
- `podman` - Podman containers
- `compose` - Docker Compose

### 3. Feature Inheritance

Cocoons automatically get:
- ✅ HTTP proxy with routing (`/cocoon/{id}/*`)
- ✅ Health checks (HTTP, TCP, command)
- ✅ Observability (logs, metrics)
- ✅ Blue-green deployments
- ✅ Service exposure (cocoon-to-cocoon communication)
- ✅ Environment injection
- ✅ Restart policies

### 4. Extensibility

Core plugins can:
- React to lifecycle events
- Implement custom orchestration logic
- Extend control interfaces (WebSocket, HTTP, gRPC)
- Add monitoring and automation

## Example Use Cases

### 1. Remote Cocoon Spawning

```
Platform API → Signaling → SignalingControlPlugin → DaemonClient
    → Daemon → ServiceManager → ScriptRunner → Native process
```

Access cocoon via HTTP proxy:
```bash
curl https://hive.example.com/cocoon/abc123/api/execute \
  -H "Content-Type: application/json" \
  -d '{"task": "process-data"}'
```

### 2. Auto-scaling Web Services

```
AutoscalerPlugin monitors CPU → Detects high load → DaemonClient
    → Daemon creates new instance → Load balancer updated
```

### 3. Disaster Recovery

```
BackupPlugin periodic task → DaemonClient lists services
    → Save configs to S3 → On failure, restore from backup
```

## Testing

```bash
# Unit tests
cargo test -p lib-hive-daemon-client
cargo test -p adi-hive-core

# Integration test with running daemon
adi hive start
cargo test --test core_plugins_integration
adi hive stop
```

## Future Enhancements

- [ ] Plugin discovery (load plugins from `~/.adi/hive/plugins/`)
- [ ] Plugin marketplace
- [ ] Hot reload plugins without daemon restart
- [ ] Plugin permissions/sandboxing
- [ ] Plugin health checks
- [ ] Inter-plugin communication
- [ ] Web UI for plugin management

## License

BSL-1.0
