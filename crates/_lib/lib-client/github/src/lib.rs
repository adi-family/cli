pub mod auth;
mod client;
mod error;
mod types;

pub use auth::{basic, no_auth, token, AuthStrategy, BasicAuth, NoAuth, TokenAuth};
pub use client::{Client, ClientBuilder};
pub use error::{GitHubError, Result};
pub use types::*;
