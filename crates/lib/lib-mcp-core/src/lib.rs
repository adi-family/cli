//! MCP (Model Context Protocol) implementation for Rust.
//!
//! This crate provides a complete implementation of the [Model Context Protocol](https://modelcontextprotocol.io),
//! enabling Rust applications to create MCP servers and clients.
//!
//! # Features
//!
//! - **Protocol Types**: Full implementation of MCP protocol types (tools, resources, prompts, etc.)
//! - **JSON-RPC 2.0**: Complete JSON-RPC message handling
//! - **Transports**: Stdio and SSE transport implementations
//! - **Server**: Easy-to-use server builder with handler traits
//! - **Client**: Async client for connecting to MCP servers
//!
//! # Feature Flags
//!
//! - `stdio` (default): Enable stdio transport
//! - `sse-client`: Enable SSE client transport (adds reqwest dependency)
//! - `sse-server`: Enable SSE server transport (adds axum dependency)
//! - `full`: Enable all features
//!
//! # Quick Start - Server
//!
//! ```rust,ignore
//! use lib_mcp_core::{
//!     server::{McpServerBuilder, McpRouter},
//!     protocol::{Tool, ToolInputSchema, CallToolResult},
//!     transport::stdio::StdioTransport,
//! };
//!
//! #[tokio::main]
//! async fn main() {
//!     let server = McpServerBuilder::new("my-server", "1.0.0")
//!         .tool(
//!             Tool::new("greet", ToolInputSchema::new()
//!                 .string_property("name", "Name to greet", true))
//!                 .with_description("Greets someone"),
//!             |args| async move {
//!                 let name = args.get("name")
//!                     .and_then(|v| v.as_str())
//!                     .unwrap_or("World");
//!                 Ok(CallToolResult::text(format!("Hello, {}!", name)))
//!             },
//!         )
//!         .build();
//!
//!     let mut router = McpRouter::new(server);
//!     router.run(StdioTransport::new()).await.unwrap();
//! }
//! ```
//!
//! # Quick Start - Client
//!
//! ```rust,ignore
//! use lib_mcp_core::{
//!     client::McpClientBuilder,
//!     transport::stdio::CustomStdioTransport,
//! };
//! use std::process::{Command, Stdio};
//!
//! #[tokio::main]
//! async fn main() {
//!     // Spawn the MCP server as a subprocess
//!     let mut child = Command::new("my-mcp-server")
//!         .stdin(Stdio::piped())
//!         .stdout(Stdio::piped())
//!         .spawn()
//!         .unwrap();
//!
//!     let stdin = child.stdin.take().unwrap();
//!     let stdout = child.stdout.take().unwrap();
//!
//!     let transport = CustomStdioTransport::new(
//!         tokio::io::BufReader::new(tokio::process::ChildStdout::from_std(stdout).unwrap()),
//!         tokio::process::ChildStdin::from_std(stdin).unwrap(),
//!     );
//!
//!     let client = McpClientBuilder::new("my-client", "1.0.0")
//!         .connect(transport)
//!         .await
//!         .unwrap();
//!
//!     // List available tools
//!     let tools = client.list_tools().await.unwrap();
//!     println!("Available tools: {:?}", tools);
//!
//!     // Call a tool
//!     let mut args = std::collections::HashMap::new();
//!     args.insert("name".to_string(), serde_json::json!("Alice"));
//!     let result = client.call_tool("greet", args).await.unwrap();
//!     println!("Result: {:?}", result);
//! }
//! ```

#![warn(missing_docs)]
#![warn(rustdoc::missing_crate_level_docs)]

pub mod error;
pub mod jsonrpc;
pub mod messages;
pub mod protocol;
pub mod transport;

pub mod client;
pub mod server;

// Re-exports for convenience
pub use error::{Error, Result};
pub use jsonrpc::{JsonRpcError, Message, Notification, Request, RequestId, Response};
pub use protocol::*;

/// Prelude module for common imports.
pub mod prelude {
    pub use crate::client::{McpClient, McpClientBuilder};
    pub use crate::error::{Error, Result};
    pub use crate::jsonrpc::{Message, Request, Response};
    pub use crate::messages::*;
    pub use crate::protocol::*;
    pub use crate::server::{McpHandler, McpRouter, McpServerBuilder};

    #[cfg(feature = "stdio")]
    pub use crate::transport::stdio::{CustomStdioTransport, StdioTransport};

    #[cfg(feature = "sse-client")]
    pub use crate::transport::sse_client::{SseClientBuilder, SseClientTransport};

    #[cfg(feature = "sse-server")]
    pub use crate::transport::sse_server::{SseServerBuilder, SseServerState};
}
