//! Generic Daemon Management Library
//!
//! Provides reusable infrastructure for building daemon processes with:
//! - PID file management
//! - Cross-platform IPC (Unix sockets, Named pipes, TCP fallback)
//! - Request/response protocol framework
//! - Graceful shutdown coordination
//! - System service integration (systemd, launchd)
//! - Background process spawning
//! - Autostart configuration
//!
//! ## Quick Start with DaemonBuilder
//!
//! ```no_run
//! use lib_daemon_core::{DaemonBuilder, Result};
//!
//! #[tokio::main]
//! async fn main() -> Result<()> {
//!     // Create and configure a daemon
//!     let daemon = DaemonBuilder::new("my-service")
//!         .executable("/usr/bin/my-service")
//!         .description("My background service")
//!         .working_dir("/var/lib/my-service")
//!         .log_file("/var/log/my-service.log")
//!         .env("RUST_LOG", "info")
//!         .autostart(true)
//!         .build()?;
//!
//!     // Install as system service and start
//!     daemon.install().await?;
//!     daemon.start().await?;
//!
//!     // Or just spawn as background process
//!     // daemon.spawn().await?;
//!
//!     // Check status
//!     let status = daemon.status().await?;
//!     if status.is_running() {
//!         println!("Service running with PID {:?}", status.pid());
//!     }
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Low-Level Example
//!
//! ```no_run
//! use lib_daemon_core::{DaemonConfig, PidFile, UnixSocketServer, DaemonProtocol};
//! use serde::{Serialize, Deserialize};
//! use anyhow::Result;
//!
//! #[derive(Debug, Clone, Serialize, Deserialize)]
//! enum MyRequest {
//!     Status,
//!     Shutdown,
//! }
//!
//! #[derive(Debug, Clone, Serialize, Deserialize)]
//! enum MyResponse {
//!     Ok { message: String },
//!     Error { message: String },
//! }
//!
//! impl DaemonProtocol for MyRequest {
//!     type Response = MyResponse;
//! }
//!
//! #[tokio::main]
//! async fn main() -> Result<()> {
//!     let config = DaemonConfig::new("/var/run/mydaemon");
//!
//!     // Check if already running
//!     let pid_file = PidFile::new(config.pid_path());
//!     if let Some(pid) = pid_file.is_running()? {
//!         println!("Daemon already running with PID {}", pid);
//!         return Ok(());
//!     }
//!     drop(pid_file);
//!
//!     // Create new PID file
//!     let mut pid_file = PidFile::new(config.pid_path());
//!     pid_file.write()?;
//!
//!     // Start Unix socket server
//!     let server = UnixSocketServer::bind(config.socket_path()).await?;
//!
//!     // Accept connections and handle requests...
//!     Ok(())
//! }
//! ```
//!
//! ## Platform Support
//!
//! | Feature | Linux | macOS | Windows |
//! |---------|-------|-------|---------|
//! | Background spawn | ✅ fork | ✅ fork | ✅ CREATE_NEW_PROCESS_GROUP |
//! | IPC | ✅ Unix socket | ✅ Unix socket | ✅ Named pipes |
//! | Service install | ✅ systemd | ✅ launchd | 🔶 stub |
//! | Autostart | ✅ systemctl enable | ✅ RunAtLoad | 🔶 stub |
//! | PID checking | ✅ kill(0) | ✅ kill(0) | ✅ OpenProcess |
//! | Graceful shutdown | ✅ SIGTERM | ✅ SIGTERM | ✅ TerminateProcess |

// Core modules (existing)
pub mod config;
pub mod error;
pub mod ipc;
pub mod lifecycle;
pub mod pid;
pub mod socket;

// New cross-platform modules
pub mod builder;
pub mod ipc_transport;
pub mod platform;
pub mod service;

// Re-exports from existing modules
pub use config::DaemonConfig;
pub use error::{DaemonError, Result};
pub use ipc::DaemonProtocol;
pub use lifecycle::{ShutdownCoordinator, ShutdownHandle};
pub use pid::PidFile;
pub use socket::{UnixSocketClient, UnixSocketServer};

// Re-exports from new modules
pub use builder::{daemon, Daemon, DaemonBuilder, DaemonStatus};
pub use ipc_transport::{IpcClient, IpcEndpoint, IpcServer, IpcStream};
pub use platform::{is_process_running, kill_process, spawn_background, wait_for_exit, Platform, SpawnConfig};
pub use service::{
    get_service_manager, LaunchdManager, RestartPolicy, ServiceConfig, ServiceManager,
    ServiceStatus, SystemdManager,
};
