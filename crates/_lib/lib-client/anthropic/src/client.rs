//! Anthropic API client implementation.

use crate::auth::AuthStrategy;
use crate::error::{AnthropicError, Result};
use crate::types::{CreateMessageRequest, CreateMessageResponse, ErrorResponse};
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE};
use std::sync::Arc;

const DEFAULT_BASE_URL: &str = "https://api.anthropic.com";
const ANTHROPIC_VERSION: &str = "2023-06-01";

/// Anthropic API client.
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

    /// Create a message (non-streaming).
    pub async fn create_message(
        &self,
        request: CreateMessageRequest,
    ) -> Result<CreateMessageResponse> {
        let url = format!("{}/v1/messages", self.base_url);
        self.post(&url, &request).await
    }

    /// Send a POST request with JSON body.
    async fn post<T, B>(&self, url: &str, body: &B) -> Result<T>
    where
        T: serde::de::DeserializeOwned,
        B: serde::Serialize,
    {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert(
            "anthropic-version",
            HeaderValue::from_static(ANTHROPIC_VERSION),
        );

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

        if status.is_success() {
            let body = response.text().await?;
            tracing::debug!(status = %status_code, "Response received");
            serde_json::from_str(&body).map_err(AnthropicError::from)
        } else {
            let body = response.text().await?;
            tracing::warn!(status = %status_code, body = %body, "API error");

            // Try to parse error response
            if let Ok(error_response) = serde_json::from_str::<ErrorResponse>(&body) {
                let message = error_response.error.message;
                let error_type = error_response.error.error_type.as_str();

                return Err(match status_code {
                    401 => AnthropicError::Unauthorized,
                    403 => AnthropicError::Forbidden(message),
                    404 => AnthropicError::NotFound(message),
                    429 => {
                        // Try to extract retry-after from message
                        let retry_after = extract_retry_after(&message).unwrap_or(60);
                        AnthropicError::RateLimited { retry_after }
                    }
                    529 => AnthropicError::Overloaded,
                    _ => match error_type {
                        "invalid_request_error" => AnthropicError::InvalidRequest(message),
                        _ => AnthropicError::Api {
                            status: status_code,
                            message,
                        },
                    },
                });
            }

            Err(AnthropicError::Api {
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

/// Extract retry-after value from error message.
fn extract_retry_after(message: &str) -> Option<u64> {
    // Try to find a number in the message that could be retry-after seconds
    message.split_whitespace().find_map(|word| {
        word.trim_matches(|c: char| !c.is_ascii_digit())
            .parse()
            .ok()
    })
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
    fn test_create_message_request() {
        let request = CreateMessageRequest::new(
            "claude-sonnet-4-20250514",
            vec![Message::user("Hello")],
            1024,
        )
        .with_system("You are helpful")
        .with_temperature(0.7);

        assert_eq!(request.model, "claude-sonnet-4-20250514");
        assert_eq!(request.max_tokens, 1024);
        assert_eq!(request.system, Some("You are helpful".to_string()));
        assert_eq!(request.temperature, Some(0.7));
    }

    #[test]
    fn test_extract_retry_after() {
        assert_eq!(extract_retry_after("retry after 30 seconds"), Some(30));
        assert_eq!(extract_retry_after("wait 60s"), Some(60));
        assert_eq!(extract_retry_after("no number here"), None);
    }
}
