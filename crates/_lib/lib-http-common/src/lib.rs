//! Common HTTP utilities for ADI services.
//!
//! This library provides shared middleware and utilities for HTTP services.

pub mod bucket;
mod version_header;

pub use bucket::{Bucket, BucketResponse, Bucketable, EntityId};
pub use version_header::{version_header_layer, VersionHeaderConfig};
