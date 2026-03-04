use reqwest::header::HeaderMap;
use serde::de::DeserializeOwned;
use std::sync::Arc;
use tracing::{debug, warn};

use crate::auth::AuthStrategy;
use crate::error::{Error, Result};
use crate::types::*;

pub struct ClientBuilder<A> {
    auth: A,
    base_url: Option<String>,
}

impl ClientBuilder<()> {
    pub fn new() -> Self {
        Self {
            auth: (),
            base_url: None,
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
    /// Set the Confluence instance URL (e.g., "https://your-domain.atlassian.net/wiki").
    pub fn base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = Some(url.into());
        self
    }

    pub fn build(self) -> Result<Client> {
        let base_url = self
            .base_url
            .ok_or_else(|| Error::InvalidRequest("base_url is required".to_string()))?;

        Ok(Client {
            http: reqwest::Client::new(),
            auth: Arc::new(self.auth),
            base_url,
        })
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

    async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        self.request(reqwest::Method::GET, path, None::<&()>).await
    }

    async fn post<T: DeserializeOwned, B: serde::Serialize>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T> {
        self.request(reqwest::Method::POST, path, Some(body)).await
    }

    async fn put<T: DeserializeOwned, B: serde::Serialize>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T> {
        self.request(reqwest::Method::PUT, path, Some(body)).await
    }

    async fn delete(&self, path: &str) -> Result<()> {
        let url = format!("{}/rest/api{}", self.base_url, path);
        debug!("Confluence API request: DELETE {}", url);

        let mut headers = HeaderMap::new();
        self.auth.apply(&mut headers).await?;

        let response = self.http.delete(&url).headers(headers).send().await?;

        let status = response.status();
        if status.is_success() {
            Ok(())
        } else {
            let status_code = status.as_u16();
            let body = response.text().await.unwrap_or_default();
            warn!("Confluence API error ({}): {}", status_code, body);

            match status_code {
                401 => Err(Error::Unauthorized),
                403 => Err(Error::Forbidden(body)),
                404 => Err(Error::NotFound(body)),
                _ => Err(Error::Api {
                    status: status_code,
                    message: body,
                }),
            }
        }
    }

    async fn request<T: DeserializeOwned>(
        &self,
        method: reqwest::Method,
        path: &str,
        body: Option<&impl serde::Serialize>,
    ) -> Result<T> {
        let url = format!("{}/rest/api{}", self.base_url, path);
        debug!("Confluence API request: {} {}", method, url);

        let mut headers = HeaderMap::new();
        self.auth.apply(&mut headers).await?;
        headers.insert("Content-Type", "application/json".parse().unwrap());

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
            warn!("Confluence API error ({}): {}", status_code, body);

            match status_code {
                401 => Err(Error::Unauthorized),
                403 => Err(Error::Forbidden(body)),
                404 => Err(Error::NotFound(body)),
                409 => Err(Error::Conflict(body)),
                429 => Err(Error::RateLimited { retry_after: 60 }),
                _ => Err(Error::Api {
                    status: status_code,
                    message: body,
                }),
            }
        }
    }

    /// Get a page by ID.
    pub async fn get_page(&self, id: &str) -> Result<Page> {
        self.get(&format!(
            "/content/{}?expand=body.storage,version,space",
            id
        ))
        .await
    }

    /// Create a page.
    pub async fn create_page(&self, input: CreatePageInput) -> Result<Page> {
        self.post("/content", &input).await
    }

    /// Update a page.
    pub async fn update_page(&self, id: &str, input: UpdatePageInput) -> Result<Page> {
        self.put(&format!("/content/{}", id), &input).await
    }

    /// Delete a page.
    pub async fn delete_page(&self, id: &str) -> Result<()> {
        self.delete(&format!("/content/{}", id)).await
    }

    /// Search content using CQL.
    pub async fn search_content(&self, cql: &str, limit: Option<u32>) -> Result<SearchResult> {
        let limit = limit.unwrap_or(25);
        let encoded_cql = urlencoding::encode(cql);
        self.get(&format!(
            "/content/search?cql={}&limit={}&expand=body.storage,version",
            encoded_cql, limit
        ))
        .await
    }

    /// List all spaces.
    pub async fn list_spaces(&self, limit: Option<u32>) -> Result<SpacesResult> {
        let limit = limit.unwrap_or(25);
        self.get(&format!("/space?limit={}", limit)).await
    }

    /// Get a space by key.
    pub async fn get_space(&self, key: &str) -> Result<Space> {
        self.get(&format!("/space/{}", key)).await
    }

    /// Get page children.
    pub async fn get_page_children(&self, id: &str) -> Result<Vec<Page>> {
        let result: ChildrenResult = self
            .get(&format!(
                "/content/{}/child?expand=page.body.storage,page.version",
                id
            ))
            .await?;
        Ok(result.page.map(|p| p.results).unwrap_or_default())
    }

    /// Get page attachments.
    pub async fn get_page_attachments(&self, id: &str) -> Result<Vec<Attachment>> {
        let result: AttachmentsResult = self
            .get(&format!("/content/{}/child/attachment", id))
            .await?;
        Ok(result.results)
    }

    /// Get pages in a space.
    pub async fn get_space_content(
        &self,
        space_key: &str,
        limit: Option<u32>,
    ) -> Result<SearchResult> {
        let limit = limit.unwrap_or(25);
        self.get(&format!(
            "/content?spaceKey={}&limit={}&expand=body.storage,version",
            space_key, limit
        ))
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::BasicAuth;

    #[test]
    fn test_builder_requires_base_url() {
        let result = Client::builder()
            .auth(BasicAuth::new("email@example.com", "api_token"))
            .build();
        assert!(result.is_err());
    }

    #[test]
    fn test_builder_with_base_url() {
        let result = Client::builder()
            .auth(BasicAuth::new("email@example.com", "api_token"))
            .base_url("https://example.atlassian.net/wiki")
            .build();
        assert!(result.is_ok());
    }
}
