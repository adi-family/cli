//! Slack API client library for ADI.

pub use auth::{AuthStrategy, BotTokenAuth};
pub use blocks::*;
pub use client::{Client, ClientBuilder};
pub use error::{Error, Result};
pub use types::*;

mod auth;
mod blocks;
mod client;
mod error;
mod types;
