use async_trait::async_trait;
use reqwest::Client;
use rust_decimal::Decimal;

use super::traits::{self, EmbeddingProvider, ProviderError};
use crate::types::{EmbedModelInfo, EmbedResponse, EmbedUsageInfo, ProviderType};

const COHERE_BASE_URL: &str = "https://api.cohere.com";

/// Cohere embedding provider.
pub struct CohereEmbedProvider {
    client: Client,
    base_url: String,
}

impl CohereEmbedProvider {
    pub fn new(base_url: Option<String>) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.unwrap_or_else(|| COHERE_BASE_URL.to_string()),
        }
    }
}

#[async_trait]
impl EmbeddingProvider for CohereEmbedProvider {
    fn provider_type(&self) -> ProviderType {
        ProviderType::Cohere
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
        // Cohere uses /v2/embed with Bearer auth
        let url = traits::build_url(&self.base_url, "/v2/embed");
        traits::send_embed_request(
            &self.client,
            &url,
            &body,
            vec![("Authorization", self.auth_header(api_key))],
            timeout_secs,
        )
        .await
    }

    fn extract_usage(&self, response: &EmbedResponse) -> Option<EmbedUsageInfo> {
        let meta = response.body.get("meta")?;
        let billed_units = meta.get("billed_units")?;
        Some(EmbedUsageInfo {
            input_tokens: billed_units
                .get("input_tokens")
                .and_then(|v| v.as_i64())
                .map(|v| v as i32),
            total_tokens: None,
        })
    }

    fn extract_cost(&self, _response: &EmbedResponse) -> Option<Decimal> {
        None
    }

    fn extract_request_id(&self, response: &EmbedResponse) -> Option<String> {
        traits::extract_request_id_from(response, &["x-request-id"])
    }

    fn extract_model(&self, response: &EmbedResponse) -> Option<String> {
        traits::extract_model_from_body(response)
    }

    async fn list_models(&self, _api_key: &str) -> Result<Vec<EmbedModelInfo>, ProviderError> {
        // Cohere doesn't have a models list endpoint for embeddings; return known models
        Ok(vec![
            EmbedModelInfo {
                id: "embed-v4.0".to_string(),
                name: Some("Embed v4.0".to_string()),
                dimensions: Some(1024),
                provider: ProviderType::Cohere,
            },
            EmbedModelInfo {
                id: "embed-multilingual-v3.0".to_string(),
                name: Some("Embed Multilingual v3.0".to_string()),
                dimensions: Some(1024),
                provider: ProviderType::Cohere,
            },
            EmbedModelInfo {
                id: "embed-english-v3.0".to_string(),
                name: Some("Embed English v3.0".to_string()),
                dimensions: Some(1024),
                provider: ProviderType::Cohere,
            },
        ])
    }
}
