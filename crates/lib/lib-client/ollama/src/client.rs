//! Ollama API client implementation.

use crate::error::{OllamaError, Result};
use crate::types::{
    ChatRequest, ChatResponse, ErrorResponse, GenerateRequest, GenerateResponse, ModelInfo,
    ModelList,
};
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE};

const DEFAULT_HOST: &str = "http://localhost:11434";

/// Ollama API client.
pub struct Client {
    http: reqwest::Client,
    host: String,
}

impl Client {
    /// Create a new client builder.
    pub fn builder() -> ClientBuilder {
        ClientBuilder::new()
    }

    /// Create a new client with default settings.
    pub fn new() -> Self {
        ClientBuilder::new().build()
    }

    /// Generate a chat completion.
    pub async fn chat(&self, request: ChatRequest) -> Result<ChatResponse> {
        let url = format!("{}/api/chat", self.host);
        self.post(&url, &request).await
    }

    /// Generate a completion (non-chat).
    pub async fn generate(&self, request: GenerateRequest) -> Result<GenerateResponse> {
        let url = format!("{}/api/generate", self.host);
        self.post(&url, &request).await
    }

    /// List available models.
    pub async fn list_models(&self) -> Result<ModelList> {
        let url = format!("{}/api/tags", self.host);
        self.get(&url).await
    }

    /// Get information about a specific model.
    pub async fn show_model(&self, name: &str) -> Result<ModelInfo> {
        let url = format!("{}/api/show", self.host);
        let body = serde_json::json!({ "name": name });
        self.post(&url, &body).await
    }

    /// Pull a model from the registry.
    pub async fn pull_model(&self, name: &str) -> Result<()> {
        let url = format!("{}/api/pull", self.host);
        let body = serde_json::json!({ "name": name, "stream": false });

        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

        let response = self
            .http
            .post(&url)
            .headers(headers)
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                if e.is_connect() {
                    OllamaError::ConnectionRefused
                } else {
                    OllamaError::Request(e)
                }
            })?;

        let status = response.status();
        if status.is_success() {
            Ok(())
        } else {
            let body = response.text().await?;
            Err(OllamaError::Api {
                status: status.as_u16(),
                message: body,
            })
        }
    }

    /// Delete a model.
    pub async fn delete_model(&self, name: &str) -> Result<()> {
        let url = format!("{}/api/delete", self.host);
        let body = serde_json::json!({ "name": name });

        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

        let response = self
            .http
            .delete(&url)
            .headers(headers)
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                if e.is_connect() {
                    OllamaError::ConnectionRefused
                } else {
                    OllamaError::Request(e)
                }
            })?;

        let status = response.status();
        if status.is_success() {
            Ok(())
        } else {
            let body = response.text().await?;
            if status.as_u16() == 404 {
                Err(OllamaError::ModelNotFound(name.to_string()))
            } else {
                Err(OllamaError::Api {
                    status: status.as_u16(),
                    message: body,
                })
            }
        }
    }

    /// Check if Ollama is running.
    pub async fn is_running(&self) -> bool {
        let url = format!("{}/api/tags", self.host);
        self.http.get(&url).send().await.is_ok()
    }

    /// Send a GET request.
    async fn get<T>(&self, url: &str) -> Result<T>
    where
        T: serde::de::DeserializeOwned,
    {
        tracing::debug!(url = %url, "GET request");

        let response = self.http.get(url).send().await.map_err(|e| {
            if e.is_connect() {
                OllamaError::ConnectionRefused
            } else {
                OllamaError::Request(e)
            }
        })?;

        self.handle_response(response).await
    }

    /// Send a POST request with JSON body.
    async fn post<T, B>(&self, url: &str, body: &B) -> Result<T>
    where
        T: serde::de::DeserializeOwned,
        B: serde::Serialize,
    {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

        tracing::debug!(url = %url, "POST request");

        let response = self
            .http
            .post(url)
            .headers(headers)
            .json(body)
            .send()
            .await
            .map_err(|e| {
                if e.is_connect() {
                    OllamaError::ConnectionRefused
                } else {
                    OllamaError::Request(e)
                }
            })?;

        self.handle_response(response).await
    }

    /// Handle API response.
    async fn handle_response<T>(&self, response: reqwest::Response) -> Result<T>
    where
        T: serde::de::DeserializeOwned,
    {
        let status = response.status();
        let status_code = status.as_u16();

        if status.is_success() {
            let body = response.text().await?;
            tracing::debug!(status = %status_code, "Response received");
            serde_json::from_str(&body).map_err(OllamaError::from)
        } else {
            let body = response.text().await?;
            tracing::warn!(status = %status_code, body = %body, "API error");

            // Try to parse error response
            if let Ok(error_response) = serde_json::from_str::<ErrorResponse>(&body) {
                let message = error_response.error;

                return Err(if message.contains("not found") {
                    OllamaError::ModelNotFound(message)
                } else {
                    OllamaError::Api {
                        status: status_code,
                        message,
                    }
                });
            }

            Err(OllamaError::Api {
                status: status_code,
                message: body,
            })
        }
    }
}

impl Default for Client {
    fn default() -> Self {
        Self::new()
    }
}

/// Client builder.
pub struct ClientBuilder {
    host: String,
}

impl ClientBuilder {
    /// Create a new client builder.
    pub fn new() -> Self {
        Self {
            host: DEFAULT_HOST.to_string(),
        }
    }

    /// Set a custom host URL.
    pub fn host(mut self, host: impl Into<String>) -> Self {
        self.host = host.into();
        self
    }

    /// Build the client.
    pub fn build(self) -> Client {
        Client {
            http: reqwest::Client::new(),
            host: self.host,
        }
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
    use crate::types::Message;

    #[test]
    fn test_builder() {
        let client = Client::builder().host("http://custom:8080").build();
        assert_eq!(client.host, "http://custom:8080");
    }

    #[test]
    fn test_default_host() {
        let client = Client::new();
        assert_eq!(client.host, "http://localhost:11434");
    }

    #[test]
    fn test_chat_request() {
        let request = ChatRequest::new("llama3.2", vec![Message::user("Hello")]);
        assert_eq!(request.model, "llama3.2");
        assert!(!request.stream);
    }
}
