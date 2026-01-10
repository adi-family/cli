//! Anthropic Claude API client library.
//!
//! A type-safe, async client for the Anthropic Messages API.

mod auth;
mod client;
mod error;
mod types;

pub use auth::{ApiKeyAuth, AuthStrategy};
pub use client::{Client, ClientBuilder};
pub use error::{AnthropicError, Result};
pub use types::*;
