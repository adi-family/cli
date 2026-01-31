//! Generic Daemon Management Library
//!
//! Provides reusable infrastructure for building daemon processes with:
//! - PID file management
//! - Unix socket IPC
//! - Request/response protocol framework
//! - Graceful shutdown coordination
//!
//! ## Example
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
//!
//!     // Write PID file
//!     pid_file.write()?;
//!
//!     // Start Unix socket server
//!     let server = UnixSocketServer::bind(config.socket_path()).await?;
//!
//!     // Accept connections and handle requests...
//!
//!     Ok(())
//! }
//! ```

pub mod config;
pub mod error;
pub mod ipc;
pub mod lifecycle;
pub mod pid;
pub mod socket;

pub use config::DaemonConfig;
pub use error::{DaemonError, Result};
pub use ipc::DaemonProtocol;
pub use lifecycle::{ShutdownCoordinator, ShutdownHandle};
pub use pid::PidFile;
pub use socket::{UnixSocketClient, UnixSocketServer};
