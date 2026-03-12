//! Stable ABI definitions for indexer language plugins.
//!
//! This crate provides FFI-safe types that language analyzer plugins compile against.
//! It uses `abi_stable` to ensure binary compatibility across Rust versions.
//!
//! # Plugin Authors
//!
//! To create a language plugin:
//!
//! 1. Implement the analyzer service with methods:
//!    - `get_grammar_path` - returns path to tree-sitter grammar .so
//!    - `extract_symbols` - extracts symbols from source code
//!    - `extract_references` - extracts references from source code
//!    - `get_info` - returns language metadata
//!
//! 2. Register service with ID `adi.indexer.lang.<language>`
//!
//! See `lib-plugin-abi` for the plugin entry point pattern.

mod service;
mod types;

pub use service::*;
pub use types::*;
