//! Common HTTP utilities for ADI services.
//!
//! This library provides shared middleware and utilities for HTTP services.

mod version_header;

pub use version_header::{version_header_layer, VersionHeaderConfig};
