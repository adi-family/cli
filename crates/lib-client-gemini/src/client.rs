use reqwest::header::HeaderMap;
use serde::de::DeserializeOwned;
use std::sync::Arc;
use tracing::{debug, warn};

use crate::auth::AuthStrategy;
use crate::error::{Error, Result};
use crate::types::*;

const DEFAULT_BASE_URL: &str = "https://generativelanguage.googleapis.com/v1beta";

pub struct ClientBuilder<A> {
    auth: A,
    base_url: String,
}

impl ClientBuilder<()> {
    pub fn new() -> Self {
        Self {
            auth: (),
            base_url: DEFAULT_BASE_URL.to_string(),
        }
    }

    pub fn auth<S: AuthStrategy + 'static>(self, auth: S) -> ClientBuilder<S> {
        ClientBuilder {
            auth,
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
    pub fn base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    pub fn build(self) -> Client {
        Client {
            http: reqwest::Client::new(),
            auth: Arc::new(self.auth),
            base_url: self.base_url,
        }
    }
}

#[derive(Clone)]
pub struct Client {
    http: reqwest::Client,
    auth: Arc<dyn AuthStrategy>,
    base_url: String,
}

impl Client {
    pub fn builder() -> ClientBuilder<()> {
        ClientBuilder::new()
    }

    async fn request<T: DeserializeOwned>(
        &self,
        method: reqwest::Method,
        path: &str,
        body: Option<&impl serde::Serialize>,
    ) -> Result<T> {
        let url = format!("{}{}", self.base_url, path);
        debug!("Gemini API request: {} {}", method, url);

        let mut headers = HeaderMap::new();
        self.auth.apply(&mut headers).await?;

        let mut request = self.http.request(method, &url).headers(headers);

        if let Some(body) = body {
            request = request.json(body);
        }

        let response = request.send().await?;
        self.handle_response(response).await
    }

    async fn handle_response<T: DeserializeOwned>(&self, response: reqwest::Response) -> Result<T> {
        let status = response.status();

        if status.is_success() {
            let body = response.text().await?;
            serde_json::from_str(&body).map_err(Error::from)
        } else {
            let status_code = status.as_u16();
            let body = response.text().await.unwrap_or_default();
            warn!("Gemini API error ({}): {}", status_code, body);

            match status_code {
                401 => Err(Error::Unauthorized),
                403 => Err(Error::Forbidden(body)),
                404 => Err(Error::NotFound(body)),
                429 => {
                    let retry_after = 60;
                    Err(Error::RateLimited { retry_after })
                }
                _ => Err(Error::Api {
                    status: status_code,
                    message: body,
                }),
            }
        }
    }

    /// Generate content using the specified model.
    pub async fn generate_content(
        &self,
        model: &str,
        request: GenerateContentRequest,
    ) -> Result<GenerateContentResponse> {
        let path = format!("/models/{}:generateContent", model);
        self.request(reqwest::Method::POST, &path, Some(&request))
            .await
    }

    /// Count tokens for the given content.
    pub async fn count_tokens(
        &self,
        model: &str,
        contents: Vec<Content>,
    ) -> Result<CountTokensResponse> {
        let path = format!("/models/{}:countTokens", model);
        let request = CountTokensRequest { contents };
        self.request(reqwest::Method::POST, &path, Some(&request))
            .await
    }

    /// List available models.
    pub async fn list_models(&self) -> Result<ListModelsResponse> {
        self.request::<ListModelsResponse>(reqwest::Method::GET, "/models", None::<&()>)
            .await
    }

    /// Get a specific model.
    pub async fn get_model(&self, model: &str) -> Result<Model> {
        let path = format!("/models/{}", model);
        self.request(reqwest::Method::GET, &path, None::<&()>)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::ApiKeyAuth;

    #[test]
    fn test_builder() {
        let client = Client::builder()
            .auth(ApiKeyAuth::new("test-key"))
            .base_url("https://custom.api.com")
            .build();
        assert_eq!(client.base_url, "https://custom.api.com");
    }
}
