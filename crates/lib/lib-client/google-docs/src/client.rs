use lib_client_google_auth::AuthStrategy;
use reqwest::header::HeaderMap;
use serde::de::DeserializeOwned;
use std::sync::Arc;
use tracing::{debug, warn};

use crate::error::{Error, Result};
use crate::types::*;

const DEFAULT_BASE_URL: &str = "https://docs.googleapis.com/v1";

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
        debug!("Docs API request: {} {}", method, url);

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
            warn!("Docs API error ({}): {}", status_code, body);

            match status_code {
                401 => Err(Error::Unauthorized),
                404 => Err(Error::NotFound(body)),
                429 => Err(Error::RateLimited { retry_after: 60 }),
                _ => Err(Error::Api {
                    status: status_code,
                    message: body,
                }),
            }
        }
    }

    /// Get a document.
    pub async fn get_document(&self, id: &str) -> Result<Document> {
        self.request(
            reqwest::Method::GET,
            &format!("/documents/{}", id),
            None::<&()>,
        )
        .await
    }

    /// Create a document.
    pub async fn create_document(&self, request: CreateDocumentRequest) -> Result<Document> {
        self.request(reqwest::Method::POST, "/documents", Some(&request))
            .await
    }

    /// Batch update a document.
    pub async fn batch_update(
        &self,
        id: &str,
        request: BatchUpdateRequest,
    ) -> Result<BatchUpdateResponse> {
        self.request(
            reqwest::Method::POST,
            &format!("/documents/{}:batchUpdate", id),
            Some(&request),
        )
        .await
    }

    /// Insert text at a specific index.
    pub async fn insert_text(
        &self,
        id: &str,
        index: i32,
        text: &str,
    ) -> Result<BatchUpdateResponse> {
        let request = BatchUpdateRequest {
            requests: vec![Request::insert_text(text, index)],
        };
        self.batch_update(id, request).await
    }

    /// Delete content in a range.
    pub async fn delete_content(
        &self,
        id: &str,
        start: i32,
        end: i32,
    ) -> Result<BatchUpdateResponse> {
        let request = BatchUpdateRequest {
            requests: vec![Request::delete_range(start, end)],
        };
        self.batch_update(id, request).await
    }

    /// Get document text content.
    pub async fn get_text(&self, id: &str) -> Result<String> {
        let doc = self.get_document(id).await?;
        Ok(extract_text(&doc))
    }
}

/// Extract plain text from a document.
fn extract_text(doc: &Document) -> String {
    let mut text = String::new();
    if let Some(body) = &doc.body {
        if let Some(content) = &body.content {
            for element in content {
                if let Some(paragraph) = &element.paragraph {
                    for elem in &paragraph.elements {
                        if let Some(text_run) = &elem.text_run {
                            text.push_str(&text_run.content);
                        }
                    }
                }
            }
        }
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;
    use lib_client_google_auth::ApiKeyAuth;

    #[test]
    fn test_builder() {
        let client = Client::builder().auth(ApiKeyAuth::new("test-key")).build();
        assert_eq!(client.base_url, DEFAULT_BASE_URL);
    }
}
