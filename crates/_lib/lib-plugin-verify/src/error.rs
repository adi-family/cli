//! Error types for verification.

use thiserror::Error;

/// Errors that can occur during verification.
#[derive(Debug, Error)]
pub enum VerifyError {
    /// Invalid checksum format
    #[error("Invalid checksum format: {0}")]
    InvalidChecksumFormat(String),

    /// Checksum mismatch
    #[error("Checksum mismatch: expected {expected}, got {actual}")]
    ChecksumMismatch { expected: String, actual: String },

    /// Invalid signature format
    #[error("Invalid signature format: {0}")]
    InvalidSignatureFormat(String),

    /// Invalid public key
    #[error("Invalid public key: {0}")]
    InvalidPublicKey(String),

    /// Signature verification failed
    #[error("Signature verification failed")]
    SignatureInvalid,

    /// Base64 decoding error
    #[error("Base64 decode error: {0}")]
    Base64Error(#[from] base64::DecodeError),

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
