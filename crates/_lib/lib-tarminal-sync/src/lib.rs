//! # Tarminal Sync Protocol
//!
//! Client-agnostic synchronization protocol for Tarminal terminal emulator.
//! Supports Rust, JavaScript/TypeScript, Swift, and any language with JSON serialization.
//!
//! ## Features
//! - CRDT-based conflict resolution using Version Vectors
//! - Device pairing and discovery
//! - Incremental and full-state synchronization
//! - Terminal grid delta/snapshot sync
//! - Transport-agnostic (works with WebSocket, peer-to-peer, etc.)

pub mod grid;
pub mod messages;
pub mod metadata;
pub mod transport;
pub mod version_vector;

pub use grid::*;
pub use messages::*;
pub use metadata::*;
pub use transport::*;
pub use version_vector::*;
