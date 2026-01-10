//! Data types for the Uzu client.

use serde::{Deserialize, Serialize};

/// Generation request configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateRequest {
    /// Input prompt text.
    pub prompt: String,

    /// Maximum number of tokens to generate.
    #[serde(default = "default_max_tokens")]
    pub max_tokens: usize,

    /// Temperature for sampling (0.0 = deterministic).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,

    /// Top-p sampling parameter.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,

    /// Stop sequences to end generation.
    #[serde(default)]
    pub stop_sequences: Vec<String>,
}

impl GenerateRequest {
    /// Create a new generation request.
    pub fn new(prompt: impl Into<String>) -> Self {
        Self {
            prompt: prompt.into(),
            max_tokens: default_max_tokens(),
            temperature: None,
            top_p: None,
            stop_sequences: Vec::new(),
        }
    }

    /// Set maximum tokens.
    pub fn max_tokens(mut self, max_tokens: usize) -> Self {
        self.max_tokens = max_tokens;
        self
    }

    /// Set temperature.
    pub fn temperature(mut self, temperature: f32) -> Self {
        self.temperature = Some(temperature);
        self
    }

    /// Set top-p.
    pub fn top_p(mut self, top_p: f32) -> Self {
        self.top_p = Some(top_p);
        self
    }

    /// Add stop sequence.
    pub fn stop_sequence(mut self, seq: impl Into<String>) -> Self {
        self.stop_sequences.push(seq.into());
        self
    }
}

/// Generation response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateResponse {
    /// Generated text.
    pub text: String,

    /// Number of tokens generated.
    pub tokens_generated: usize,

    /// Whether generation was stopped early.
    pub stopped: bool,

    /// Reason for stopping (if stopped early).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_reason: Option<String>,
}

/// Model information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    /// Model name/path.
    pub name: String,

    /// Model size in bytes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,

    /// Whether the model is currently loaded.
    pub loaded: bool,
}

/// Default max tokens value.
fn default_max_tokens() -> usize {
    128
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_request() {
        let request = GenerateRequest::new("Hello")
            .max_tokens(256)
            .temperature(0.7)
            .stop_sequence("\n");

        assert_eq!(request.prompt, "Hello");
        assert_eq!(request.max_tokens, 256);
        assert_eq!(request.temperature, Some(0.7));
        assert_eq!(request.stop_sequences.len(), 1);
    }

    #[test]
    fn test_default_max_tokens() {
        let request = GenerateRequest::new("Hello");
        assert_eq!(request.max_tokens, 128);
    }
}
