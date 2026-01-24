//! Mock message handlers for testing
//!
//! Provides simulated responses for PTY, Silk, and FileSystem protocols.

pub mod pty;
pub mod silk;
pub mod filesystem;

pub use pty::PtyHandler;
pub use silk::SilkHandler;
pub use filesystem::FileSystemHandler;

use serde::{Deserialize, Serialize};

/// Common trait for message handlers
pub trait MessageHandler: Send + Sync {
    /// Handle an incoming message and optionally return a response
    fn handle(&self, data: &str) -> Option<String>;
    
    /// Get the channel name this handler is for
    fn channel(&self) -> &'static str;
}

/// Generic response wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum HandlerResponse {
    Pty(pty::PtyResponse),
    Silk(silk::SilkResponse),
    FileSystem(filesystem::FileSystemResponse),
}
