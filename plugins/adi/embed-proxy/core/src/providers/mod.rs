pub mod cohere;
pub mod google;
pub mod openai;
pub mod traits;

pub use cohere::CohereEmbedProvider;
pub use google::GoogleEmbedProvider;
pub use openai::OpenAIEmbedProvider;
pub use traits::{EmbeddingProvider, ProviderError};

use crate::types::ProviderType;

/// Create a provider instance based on type.
pub fn create_provider(
    provider_type: ProviderType,
    base_url: Option<String>,
) -> Box<dyn EmbeddingProvider> {
    match provider_type {
        ProviderType::OpenAI => Box::new(OpenAIEmbedProvider::new(base_url)),
        ProviderType::Cohere => Box::new(CohereEmbedProvider::new(base_url)),
        ProviderType::Google => Box::new(GoogleEmbedProvider::new(base_url)),
        ProviderType::Custom => Box::new(OpenAIEmbedProvider::custom(
            base_url.unwrap_or_else(|| "http://localhost:8080".to_string()),
        )),
    }
}
