//! Authentication middleware for adi-api-proxy.

pub mod jwt_auth;
pub mod proxy_auth;

pub use jwt_auth::{AdminUser, AuthUser};
pub use proxy_auth::ProxyAuth;
