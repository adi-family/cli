//! Uzu inference engine client library for Apple Silicon.
//!
//! This library provides a high-level, idiomatic Rust interface to the
//! [Uzu](https://github.com/trymirai/uzu) inference engine, which is
//! optimized for running large language models on Apple Silicon devices.
//!
//! # Features
//!
//! - Simple API for local LLM inference
//! - Optimized for Apple Silicon (M1/M2/M3)
//! - Synchronous (blocking) API
//! - Support for various model formats (GGUF, etc.)
//! - No network requests - fully local execution
//!
//! # Example
//!
//! ```no_run
//! use lib_client_uzu::{Client, GenerateRequest};
//!
//! // Load a model
//! let mut client = Client::new("models/llama-3.2-1b.gguf")?;
//!
//! // Generate text
//! let request = GenerateRequest::new("Tell me about Rust programming")
//!     .max_tokens(256)
//!     .temperature(0.7);
//!
//! let response = client.generate(request)?;
//! println!("{}", response.text);
//! # Ok::<(), lib_client_uzu::UzuError>(())
//! ```
//!
//! # Performance
//!
//! Uzu is specifically optimized for Apple Silicon and leverages:
//! - Unified memory architecture
//! - Metal GPU acceleration
//! - Optimized kernels for M-series chips
//!
//! Typical performance on Apple M2:
//! - Llama-3.2-1B: ~35 tokens/second
//! - Competitive with llama.cpp on Apple Silicon
//!
//! # Requirements
//!
//! - macOS with Apple Silicon (M1/M2/M3 or later)
//! - Xcode and Metal toolchain
//! - Model files in supported format (GGUF recommended)

mod client;
mod error;
mod types;

pub use client::{Client, ClientBuilder};
pub use error::{Result, UzuError};
pub use types::{GenerateRequest, GenerateResponse, ModelInfo};
