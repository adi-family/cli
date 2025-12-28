//! GitLab API client library for ADI.

pub use auth::{AuthStrategy, JobTokenAuth, OAuthAuth, PrivateTokenAuth};
pub use client::{Client, ClientBuilder};
pub use error::{Error, Result};
pub use types::*;

mod auth;
mod client;
mod error;
mod types;
