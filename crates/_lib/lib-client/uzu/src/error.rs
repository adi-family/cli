//! Error types for the Uzu client.

use std::io;
use thiserror::Error;

/// Uzu client error types.
#[derive(Debug, Error)]
pub enum UzuError {
    /// Model file not found.
    #[error("Model not found at path: {0}")]
    ModelNotFound(String),

    /// Failed to load model.
    #[error("Failed to load model: {0}")]
    ModelLoad(String),

    /// Failed to run inference.
    #[error("Failed to run inference: {0}")]
    InferenceError(String),

    /// I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    /// Invalid configuration.
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    /// Invalid input.
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    /// Other error.
    #[error("Error: {0}")]
    Other(String),
}

/// Result type for Uzu operations.
pub type Result<T> = std::result::Result<T, UzuError>;
