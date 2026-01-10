//! ADI Coolify Core
//!
//! Core library for Coolify integration providing async API client
//! and deployment management capabilities.

mod client;
mod error;
mod types;

pub use client::CoolifyClient;
pub use error::{CoolifyError, Result};
pub use types::{Deployment, DeploymentStatus, Service, ServiceStatus};
