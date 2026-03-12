//! Plugin signature and checksum verification.
//!
//! Provides optional verification for plugin binaries:
//! - SHA256 checksums for integrity verification
//! - Ed25519 signatures for authenticity verification
//!
//! # Example
//!
//! ```rust,ignore
//! use lib_plugin_verify::{Verifier, VerifyResult};
//!
//! let verifier = Verifier::new()
//!     .with_trusted_key("base64-public-key")
//!     .require_signatures(false);
//!
//! // Verify checksum
//! let valid = Verifier::verify_checksum(data, "sha256:abc123...");
//!
//! // Verify signature
//! let result = verifier.verify_signature(data, signature, public_key);
//! match result {
//!     VerifyResult::Verified { key_id } => println!("Verified with {}", key_id),
//!     VerifyResult::UntrustedKey { key } => println!("Valid but untrusted"),
//!     VerifyResult::NoSignature => println!("No signature provided"),
//!     VerifyResult::Invalid { reason } => println!("Invalid: {}", reason),
//! }
//! ```

mod checksum;
mod error;
mod signature;

pub use checksum::*;
pub use error::*;
pub use signature::*;
