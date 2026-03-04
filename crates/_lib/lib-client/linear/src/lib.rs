//! Linear API client library for ADI.

pub use auth::{ApiKeyAuth, AuthStrategy};
pub use client::{Client, ClientBuilder};
pub use error::{Error, Result};
pub use graphql::GraphQLRequest;
pub use types::*;

mod auth;
mod client;
mod error;
mod graphql;
mod types;
