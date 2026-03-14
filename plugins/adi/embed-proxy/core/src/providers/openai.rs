use async_trait::async_trait;
use reqwest::Client;
use rust_decimal::Decimal;

use super::traits::{self, EmbeddingProvider, ProviderError};
use crate::types::{EmbedModelInfo, EmbedResponse, EmbedUsageInfo, ProviderType};

const OPENAI_BASE_URL: &str = "https://api.openai.com";

/// OpenAI-compatible embedding provider.
pub struct OpenAIEmbedProvider {
    client: Client,
    base_url: String,
    provider_type: ProviderType,
    optional_auth: bool,
}

impl OpenAIEmbedProvider {
    pub fn new(base_url: Option<String>) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.unwrap_or_else(|| OPENAI_BASE_URL.to_string()),
            provider_type: ProviderType::OpenAI,
            optional_auth: false,
        }
    }

    pub fn custom(base_url: String) -> Self {
        Self {
            client: Client::new(),
            base_url,
            provider_type: ProviderType::Custom,
            optional_auth: true,
        }
    }

    fn auth_headers(&self, api_key: &str) -> Vec<(&str, String)> {
        if self.optional_auth && api_key.is_empty() {
            vec![]
        } else {
            vec![("Authorization", self.auth_header(api_key))]
        }
    }
}

#[async_trait]
impl EmbeddingProvider for OpenAIEmbedProvider {
    fn provider_type(&self) -> ProviderType {
        self.provider_type
    }

    fn base_url(&self) -> &str {
        &self.base_url
    }

    async fn embed(
        &self,
        api_key: &str,
        body: serde_json::Value,
        timeout_secs: u64,
    ) -> Result<EmbedResponse, ProviderError> {
        let url = traits::build_url(&self.base_url, "/v1/embeddings");
        traits::send_embed_request(
            &self.client,
            &url,
            &body,
            self.auth_headers(api_key),
            timeout_secs,
        )
        .await
    }

    fn extract_usage(&self, response: &EmbedResponse) -> Option<EmbedUsageInfo> {
        traits::extract_openai_embed_usage(response)
    }

    fn extract_cost(&self, _response: &EmbedResponse) -> Option<Decimal> {
        None
    }

    fn extract_request_id(&self, response: &EmbedResponse) -> Option<String> {
        let headers = if self.optional_auth {
            &["x-request-id", "request-id", "x-trace-id"][..]
        } else {
            &["x-request-id"][..]
        };
        traits::extract_request_id_from(response, headers)
    }

    fn extract_model(&self, response: &EmbedResponse) -> Option<String> {
        traits::extract_model_from_body(response)
    }

    async fn list_models(&self, api_key: &str) -> Result<Vec<EmbedModelInfo>, ProviderError> {
        let url = traits::build_url(&self.base_url, "/v1/models");

        let mut builder = self.client.get(&url);
        if !api_key.is_empty() || !self.optional_auth {
            builder = builder.header("Authorization", self.auth_header(api_key));
        }

        let response = builder.send().await?;

        if !response.status().is_success() {
            if self.optional_auth {
                return Ok(vec![]);
            }
            let error = response.text().await.unwrap_or_default();
            return Err(ProviderError::RequestFailed(error));
        }

        let body: serde_json::Value = response
            .json()
            .await
            .map_err(|e| ProviderError::Parse(e.to_string()))?;

        let models = body
            .get("data")
            .and_then(|d| d.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|m| {
                        let id = m.get("id")?.as_str()?;
                        // Filter to embedding models only
                        if !id.contains("embed") {
                            return None;
                        }
                        Some(EmbedModelInfo {
                            id: id.to_string(),
                            name: m.get("name").and_then(|n| n.as_str()).map(String::from),
                            dimensions: None,
                            provider: self.provider_type,
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        Ok(models)
    }
}
