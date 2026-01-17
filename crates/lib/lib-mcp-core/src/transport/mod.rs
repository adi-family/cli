//! Transport layer for MCP communication.
//!
//! MCP supports multiple transport mechanisms:
//! - **stdio**: Standard input/output (default for CLI tools)
//! - **SSE**: Server-Sent Events over HTTP (for web servers)

mod traits;

#[cfg(feature = "stdio")]
pub mod stdio;

#[cfg(feature = "sse-client")]
pub mod sse_client;

#[cfg(feature = "sse-server")]
pub mod sse_server;

pub use traits::*;
