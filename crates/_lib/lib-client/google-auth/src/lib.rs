//! Google OAuth2 authentication library for ADI.
//!
//! Provides authentication strategies for Google APIs:
//! - API key authentication
//! - Service account authentication (JWT)
//! - OAuth2 authentication (user consent flow)

pub use auth::{ApiKeyAuth, AuthStrategy, OAuth2Auth, ServiceAccountAuth};
pub use credentials::{Credentials, ServiceAccountCredentials};
pub use error::{Error, Result};
pub use token::{Token, TokenStore};

mod auth;
mod credentials;
mod error;
mod token;
