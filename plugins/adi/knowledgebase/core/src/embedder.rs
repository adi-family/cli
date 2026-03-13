use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

#[derive(Debug, thiserror::Error)]
pub enum EmbedderError {
    #[error("Embedding generation failed: {0}")]
    Failed(String),
}

pub trait Embedder: Send + Sync {
    fn embed(&self, texts: &[&str]) -> std::result::Result<Vec<Vec<f32>>, EmbedderError>;
    fn dimensions(&self) -> u32;
}

/// Google Embedding API embedder.
///
/// Calls the Google text-embedding endpoint. Requires a valid API key.
pub struct GoogleEmbedder {
    api_key: String,
    model: String,
    client: reqwest::Client,
}

impl GoogleEmbedder {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            model: "text-embedding-004".to_string(),
            client: reqwest::Client::new(),
        }
    }

    pub fn with_model(mut self, model: String) -> Self {
        self.model = model;
        self
    }
}

impl Embedder for GoogleEmbedder {
    fn embed(&self, texts: &[&str]) -> std::result::Result<Vec<Vec<f32>>, EmbedderError> {
        let rt = tokio::runtime::Handle::try_current()
            .map_err(|e| EmbedderError::Failed(format!("No tokio runtime: {e}")))?;

        rt.block_on(async {
            let url = format!(
                "https://generativelanguage.googleapis.com/v1beta/models/{}:embedContent?key={}",
                self.model, self.api_key
            );

            let mut results = Vec::with_capacity(texts.len());
            for text in texts {
                let body = serde_json::json!({
                    "model": format!("models/{}", self.model),
                    "content": { "parts": [{ "text": text }] }
                });

                let resp = self
                    .client
                    .post(&url)
                    .json(&body)
                    .send()
                    .await
                    .map_err(|e| EmbedderError::Failed(e.to_string()))?;

                let json: serde_json::Value = resp
                    .json()
                    .await
                    .map_err(|e| EmbedderError::Failed(e.to_string()))?;

                let values = json["embedding"]["values"]
                    .as_array()
                    .ok_or_else(|| EmbedderError::Failed("Missing embedding.values".into()))?
                    .iter()
                    .filter_map(|v| v.as_f64().map(|f| f as f32))
                    .collect::<Vec<f32>>();

                if values.is_empty() {
                    return Err(EmbedderError::Failed("Empty embedding returned".into()));
                }

                results.push(values);
            }

            Ok(results)
        })
    }

    fn dimensions(&self) -> u32 {
        768
    }
}

/// Hash-based deterministic embedder for development and testing.
///
/// Produces consistent vectors for identical text, enabling duplicate detection
/// in tests without requiring an external API.
pub struct DummyEmbedder {
    dimensions: u32,
}

impl DummyEmbedder {
    pub fn new(dimensions: u32) -> Self {
        Self { dimensions }
    }
}

impl Embedder for DummyEmbedder {
    fn embed(&self, texts: &[&str]) -> std::result::Result<Vec<Vec<f32>>, EmbedderError> {
        Ok(texts.iter().map(|text| hash_to_vector(text, self.dimensions)).collect())
    }

    fn dimensions(&self) -> u32 {
        self.dimensions
    }
}

fn hash_to_vector(text: &str, dimensions: u32) -> Vec<f32> {
    (0..dimensions)
        .map(|i| {
            let mut hasher = DefaultHasher::new();
            text.hash(&mut hasher);
            i.hash(&mut hasher);
            let h = hasher.finish();
            // Normalize to [-1, 1]
            (h as f32 / u64::MAX as f32) * 2.0 - 1.0
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dummy_embedder_deterministic() {
        let embedder = DummyEmbedder::new(128);
        let a = embedder.embed(&["hello world"]).unwrap();
        let b = embedder.embed(&["hello world"]).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn dummy_embedder_different_texts_differ() {
        let embedder = DummyEmbedder::new(128);
        let a = embedder.embed(&["hello"]).unwrap();
        let b = embedder.embed(&["world"]).unwrap();
        assert_ne!(a[0], b[0]);
    }

    #[test]
    fn dummy_embedder_correct_dimensions() {
        let embedder = DummyEmbedder::new(64);
        assert_eq!(embedder.dimensions(), 64);
        let result = embedder.embed(&["test"]).unwrap();
        assert_eq!(result[0].len(), 64);
    }

    #[test]
    fn dummy_embedder_batch() {
        let embedder = DummyEmbedder::new(32);
        let results = embedder.embed(&["a", "b", "c"]).unwrap();
        assert_eq!(results.len(), 3);
        assert_ne!(results[0], results[1]);
        assert_ne!(results[1], results[2]);
    }

    #[test]
    fn dummy_embedder_values_in_range() {
        let embedder = DummyEmbedder::new(256);
        let result = embedder.embed(&["test"]).unwrap();
        for val in &result[0] {
            assert!(*val >= -1.0 && *val <= 1.0);
        }
    }
}
