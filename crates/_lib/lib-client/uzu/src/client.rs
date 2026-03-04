//! Uzu client implementation.

use crate::error::{Result, UzuError};
use crate::types::{GenerateRequest, GenerateResponse, ModelInfo};
use std::path::{Path, PathBuf};
use uzu::session::config::{DecodingConfig, RunConfig};
use uzu::session::types::{Input, Output};
use uzu::session::Session;

/// Uzu inference client.
///
/// Wraps the Uzu Session for running local LLM inference on Apple Silicon.
pub struct Client {
    session: Session,
    model_path: PathBuf,
}

impl Client {
    /// Create a new client builder.
    pub fn builder() -> ClientBuilder {
        ClientBuilder::new()
    }

    /// Create a new client with a model path.
    ///
    /// # Arguments
    ///
    /// * `model_path` - Path to the model file (.gguf or native format)
    ///
    /// # Example
    ///
    /// ```no_run
    /// use lib_client_uzu::Client;
    ///
    /// let client = Client::new("models/llama-3.2-1b.gguf")?;
    /// # Ok::<(), lib_client_uzu::UzuError>(())
    /// ```
    pub fn new(model_path: impl AsRef<Path>) -> Result<Self> {
        ClientBuilder::new().model_path(model_path).build()
    }

    /// Generate text completion.
    ///
    /// # Arguments
    ///
    /// * `request` - Generation request with prompt and configuration
    ///
    /// # Example
    ///
    /// ```no_run
    /// use lib_client_uzu::{Client, GenerateRequest};
    ///
    /// let client = Client::new("models/llama-3.2-1b.gguf")?;
    /// let request = GenerateRequest::new("Tell me about Rust")
    ///     .max_tokens(256)
    ///     .temperature(0.7);
    /// let response = client.generate(request)?;
    /// println!("{}", response.text);
    /// # Ok::<(), lib_client_uzu::UzuError>(())
    /// ```
    pub fn generate(&mut self, request: GenerateRequest) -> Result<GenerateResponse> {
        tracing::debug!(
            prompt = %request.prompt,
            max_tokens = request.max_tokens,
            "Generating completion"
        );

        let input = Input::Text(request.prompt.clone());

        // Configure run settings
        let mut run_config = RunConfig::default().tokens_limit(request.max_tokens);

        // Apply temperature if specified
        if let Some(temp) = request.temperature {
            run_config = run_config.temperature(temp);
        }

        // Run inference with optional callback
        let mut stopped = false;
        let mut stop_reason = None;

        let output = self
            .session
            .run(input, run_config, Some(|_: Output| true))
            .map_err(|e| UzuError::InferenceError(e.to_string()))?;

        // Check if we hit stop sequences
        for stop_seq in &request.stop_sequences {
            if output.text.original.contains(stop_seq) {
                stopped = true;
                stop_reason = Some(format!("stop_sequence: {}", stop_seq));
                break;
            }
        }

        let response = GenerateResponse {
            text: output.text.original,
            tokens_generated: output.tokens_generated,
            stopped,
            stop_reason,
        };

        tracing::debug!(
            tokens_generated = response.tokens_generated,
            stopped = response.stopped,
            "Generation completed"
        );

        Ok(response)
    }

    /// Get model information.
    pub fn model_info(&self) -> ModelInfo {
        let size = std::fs::metadata(&self.model_path)
            .ok()
            .map(|m| m.len());

        ModelInfo {
            name: self
                .model_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string(),
            size,
            loaded: true,
        }
    }

    /// Get the model path.
    pub fn model_path(&self) -> &Path {
        &self.model_path
    }
}

/// Client builder.
pub struct ClientBuilder {
    model_path: Option<PathBuf>,
    decoding_config: DecodingConfig,
}

impl ClientBuilder {
    /// Create a new client builder.
    pub fn new() -> Self {
        Self {
            model_path: None,
            decoding_config: DecodingConfig::default(),
        }
    }

    /// Set the model path.
    pub fn model_path(mut self, path: impl AsRef<Path>) -> Self {
        self.model_path = Some(path.as_ref().to_path_buf());
        self
    }

    /// Build the client.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - No model path was specified
    /// - The model file doesn't exist
    /// - The model fails to load
    pub fn build(self) -> Result<Client> {
        let model_path = self
            .model_path
            .ok_or_else(|| UzuError::InvalidConfig("model_path is required".to_string()))?;

        // Check if model file exists
        if !model_path.exists() {
            return Err(UzuError::ModelNotFound(
                model_path.to_string_lossy().to_string(),
            ));
        }

        tracing::info!(
            model_path = %model_path.display(),
            "Loading model"
        );

        // Create session
        let session = Session::new(model_path.clone(), self.decoding_config)
            .map_err(|e| UzuError::ModelLoad(e.to_string()))?;

        tracing::info!("Model loaded successfully");

        Ok(Client {
            session,
            model_path,
        })
    }
}

impl Default for ClientBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_requires_model_path() {
        let result = ClientBuilder::new().build();
        assert!(result.is_err());
        assert!(matches!(result, Err(UzuError::InvalidConfig(_))));
    }

    #[test]
    fn test_builder_checks_file_exists() {
        let result = ClientBuilder::new()
            .model_path("/nonexistent/model.gguf")
            .build();
        assert!(result.is_err());
        assert!(matches!(result, Err(UzuError::ModelNotFound(_))));
    }
}
