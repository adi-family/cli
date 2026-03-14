use async_trait::async_trait;
use reqwest::Client;
use rust_decimal::Decimal;
use std::time::Duration;

use super::traits::{self, EmbeddingProvider, ProviderError};
use crate::types::{EmbedModelInfo, EmbedResponse, EmbedUsageInfo, ProviderType};

const GOOGLE_BASE_URL: &str = "https://generativelanguage.googleapis.com";

/// Google embedding provider (Gemini API).
pub struct GoogleEmbedProvider {
    client: Client,
    base_url: String,
}

impl GoogleEmbedProvider {
    pub fn new(base_url: Option<String>) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.unwrap_or_else(|| GOOGLE_BASE_URL.to_string()),
        }
    }
}

#[async_trait]
impl EmbeddingProvider for GoogleEmbedProvider {
    fn provider_type(&self) -> ProviderType {
        ProviderType::Google
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
        // Google uses model in the URL path and API key as query param
        let model = body
            .get("model")
            .and_then(|m| m.as_str())
            .unwrap_or("text-embedding-004");

        let url = format!(
            "{}/v1beta/models/{}:embedContent?key={}",
            self.base_url.trim_end_matches('/'),
            model,
            api_key
        );

        // Transform request body to Google format
        let google_body = transform_to_google_format(&body);

        let response = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .timeout(Duration::from_secs(timeout_secs))
            .json(&google_body)
            .send()
            .await?;

        let status = response.status();
        let headers = response.headers().clone();

        if !status.is_success() {
            let error_body = response.text().await.unwrap_or_default();
            return Err(traits::map_error_status(status.as_u16(), error_body));
        }

        let resp_body: serde_json::Value = response
            .json()
            .await
            .map_err(|e| ProviderError::Parse(e.to_string()))?;

        // Transform Google response to OpenAI format
        let openai_body = transform_from_google_format(&resp_body, model);

        Ok(EmbedResponse {
            status,
            headers,
            body: openai_body,
        })
    }

    fn extract_usage(&self, _response: &EmbedResponse) -> Option<EmbedUsageInfo> {
        // Google doesn't report token usage for embeddings
        None
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
        Ok(vec![
            EmbedModelInfo {
                id: "text-embedding-004".to_string(),
                name: Some("Text Embedding 004".to_string()),
                dimensions: Some(768),
                provider: ProviderType::Google,
            },
            EmbedModelInfo {
                id: "text-multilingual-embedding-002".to_string(),
                name: Some("Text Multilingual Embedding 002".to_string()),
                dimensions: Some(768),
                provider: ProviderType::Google,
            },
        ])
    }
}

fn transform_to_google_format(body: &serde_json::Value) -> serde_json::Value {
    let input = body.get("input");
    let text = match input {
        Some(serde_json::Value::String(s)) => s.clone(),
        Some(serde_json::Value::Array(arr)) => arr
            .first()
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        _ => String::new(),
    };

    serde_json::json!({
        "content": {
            "parts": [{ "text": text }]
        }
    })
}

fn transform_from_google_format(body: &serde_json::Value, model: &str) -> serde_json::Value {
    let values = body
        .get("embedding")
        .and_then(|e| e.get("values"))
        .cloned()
        .unwrap_or(serde_json::json!([]));

    serde_json::json!({
        "object": "list",
        "data": [{
            "object": "embedding",
            "embedding": values,
            "index": 0
        }],
        "model": model,
        "usage": {
            "prompt_tokens": 0,
            "total_tokens": 0
        }
    })
}
