//! Jira API client library for ADI.

pub use auth::{AuthStrategy, BasicAuth, BearerAuth};
pub use client::{Client, ClientBuilder};
pub use error::{Error, Result};
pub use types::*;

mod auth;
mod client;
mod error;
mod types;
