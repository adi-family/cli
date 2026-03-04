//! Ollama API client library.
//!
//! A type-safe, async client for the Ollama REST API.

mod client;
mod error;
mod types;

pub use client::{Client, ClientBuilder};
pub use error::{OllamaError, Result};
pub use types::*;
