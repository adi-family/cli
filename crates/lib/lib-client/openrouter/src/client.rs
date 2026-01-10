//! OpenRouter API client implementation.

use crate::auth::AuthStrategy;
use crate::error::{OpenRouterError, Result};
use crate::types::{
    CreateChatCompletionRequest, CreateChatCompletionResponse, CreditsResponse, ErrorResponse,
    GenerationStats, Model, ModelList,
};
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE};
use std::sync::Arc;

const DEFAULT_BASE_URL: &str = "https://openrouter.ai/api/v1";

/// OpenRouter API client.
pub struct Client {
    http: reqwest::Client,
    auth: Arc<dyn AuthStrategy>,
    base_url: String,
}

impl Client {
    /// Create a new client builder.
    pub fn builder() -> ClientBuilder<()> {
        ClientBuilder::new()
    }

    /// Create a chat completion.
    pub async fn create_chat_completion(
        &self,
        request: CreateChatCompletionRequest,
    ) -> Result<CreateChatCompletionResponse> {
        let url = format!("{}/chat/completions", self.base_url);
        self.post(&url, &request).await
    }

    /// List available models.
    pub async fn list_models(&self) -> Result<ModelList> {
        let url = format!("{}/models", self.base_url);
        self.get(&url).await
    }

    /// Get a specific model by ID.
    pub async fn get_model(&self, model_id: &str) -> Result<Model> {
        let models = self.list_models().await?;
        models
            .data
            .into_iter()
            .find(|m| m.id == model_id)
            .ok_or_else(|| OpenRouterError::NotFound(format!("Model not found: {}", model_id)))
    }

    /// Get generation statistics by ID.
    pub async fn get_generation(&self, generation_id: &str) -> Result<GenerationStats> {
        let url = format!("{}/generation?id={}", self.base_url, generation_id);
        self.get(&url).await
    }

    /// Get account credits/balance.
    pub async fn get_credits(&self) -> Result<CreditsResponse> {
        // Note: This endpoint is at /api/v1/auth/key
        let url = format!("{}/auth/key", self.base_url);
        self.get(&url).await
    }

    /// Send a GET request.
    async fn get<T>(&self, url: &str) -> Result<T>
    where
        T: serde::de::DeserializeOwned,
    {
        let mut headers = HeaderMap::new();
        self.auth.apply(&mut headers).await?;

        tracing::debug!(url = %url, "GET request");

        let response = self.http.get(url).headers(headers).send().await?;

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
        self.auth.apply(&mut headers).await?;

        tracing::debug!(url = %url, "POST request");

        let response = self
            .http
            .post(url)
            .headers(headers)
            .json(body)
            .send()
            .await?;

        self.handle_response(response).await
    }

    /// Handle API response.
    async fn handle_response<T>(&self, response: reqwest::Response) -> Result<T>
    where
        T: serde::de::DeserializeOwned,
    {
        let status = response.status();
        let status_code = status.as_u16();

        // Extract rate limit headers before consuming response
        let retry_after = response
            .headers()
            .get("retry-after")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse().ok());

        if status.is_success() {
            let body = response.text().await?;
            tracing::debug!(status = %status_code, "Response received");
            serde_json::from_str(&body).map_err(OpenRouterError::from)
        } else {
            let body = response.text().await?;
            tracing::warn!(status = %status_code, body = %body, "API error");

            // Try to parse error response
            if let Ok(error_response) = serde_json::from_str::<ErrorResponse>(&body) {
                let message = error_response.error.message;
                let code = error_response.error.code;

                return Err(match status_code {
                    401 => OpenRouterError::Unauthorized,
                    402 => OpenRouterError::InsufficientCredits(message),
                    403 => OpenRouterError::Forbidden(message),
                    404 => OpenRouterError::NotFound(message),
                    429 => OpenRouterError::RateLimited {
                        retry_after: retry_after.unwrap_or(60),
                    },
                    500..=599 => OpenRouterError::ServerError(message),
                    _ => match code {
                        Some(400) => OpenRouterError::InvalidRequest(message),
                        Some(404) => OpenRouterError::ModelNotAvailable(message),
                        _ => OpenRouterError::Api {
                            status: status_code,
                            message,
                        },
                    },
                });
            }

            Err(OpenRouterError::Api {
                status: status_code,
                message: body,
            })
        }
    }
}

/// Client builder.
pub struct ClientBuilder<A> {
    auth: A,
    base_url: String,
}

impl ClientBuilder<()> {
    /// Create a new client builder.
    pub fn new() -> Self {
        Self {
            auth: (),
            base_url: DEFAULT_BASE_URL.to_string(),
        }
    }

    /// Set the authentication strategy.
    pub fn auth<S: AuthStrategy + 'static>(self, strategy: S) -> ClientBuilder<S> {
        ClientBuilder {
            auth: strategy,
            base_url: self.base_url,
        }
    }
}

impl Default for ClientBuilder<()> {
    fn default() -> Self {
        Self::new()
    }
}

impl<A: AuthStrategy + 'static> ClientBuilder<A> {
    /// Set a custom base URL.
    pub fn base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    /// Build the client.
    pub fn build(self) -> Client {
        Client {
            http: reqwest::Client::new(),
            auth: Arc::new(self.auth),
            base_url: self.base_url,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::ApiKeyAuth;
    use crate::types::Message;

    #[test]
    fn test_builder() {
        let client = Client::builder()
            .auth(ApiKeyAuth::new("test-key"))
            .base_url("https://custom.api.com")
            .build();

        assert_eq!(client.base_url, "https://custom.api.com");
    }

    #[test]
    fn test_create_chat_completion_request() {
        let request =
            CreateChatCompletionRequest::new("openai/gpt-4o", vec![Message::user("Hello")])
                .with_max_tokens(1024)
                .with_temperature(0.7);

        assert_eq!(request.model, "openai/gpt-4o");
        assert_eq!(request.max_tokens, Some(1024));
        assert_eq!(request.temperature, Some(0.7));
    }

    #[test]
    fn test_auth_with_site_info() {
        let auth = ApiKeyAuth::new("sk-or-test")
            .with_site_url("https://myapp.com")
            .with_site_name("My App");

        let _client = Client::builder().auth(auth).build();
    }
}
