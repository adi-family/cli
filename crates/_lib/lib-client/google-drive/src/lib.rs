//! Google Drive API client library for ADI.

pub use client::{Client, ClientBuilder};
pub use error::{Error, Result};
pub use lib_client_google_auth::{ApiKeyAuth, AuthStrategy, OAuth2Auth, ServiceAccountAuth};
pub use types::*;

mod client;
mod error;
mod types;
