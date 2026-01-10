//! OpenAI API client library.
//!
//! A type-safe, async client for the OpenAI Chat Completions API.

mod auth;
mod client;
mod error;
mod types;

pub use auth::{ApiKeyAuth, AuthStrategy};
pub use client::{Client, ClientBuilder};
pub use error::{OpenAiError, Result};
pub use types::*;
