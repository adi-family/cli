//! OpenRouter API client library.
//!
//! A type-safe, async client for the OpenRouter API.
//! OpenRouter provides access to multiple AI models through a unified OpenAI-compatible API.

mod auth;
mod client;
mod error;
mod types;

pub use auth::{ApiKeyAuth, AuthStrategy};
pub use client::{Client, ClientBuilder};
pub use error::{OpenRouterError, Result};
pub use types::*;
