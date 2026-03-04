use bytes::Bytes;
use lib_client_google_auth::AuthStrategy;
use reqwest::header::HeaderMap;
use serde::de::DeserializeOwned;
use std::sync::Arc;
use tracing::{debug, warn};

use crate::error::{Error, Result};
use crate::types::*;

const DEFAULT_BASE_URL: &str = "https://www.googleapis.com/drive/v3";
const UPLOAD_BASE_URL: &str = "https://www.googleapis.com/upload/drive/v3";

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
        debug!("Drive API request: {} {}", method, url);

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
            warn!("Drive API error ({}): {}", status_code, body);

            match status_code {
                401 => Err(Error::Unauthorized),
                403 => Err(Error::Forbidden(body)),
                404 => Err(Error::NotFound(body)),
                429 => Err(Error::RateLimited { retry_after: 60 }),
                _ => Err(Error::Api {
                    status: status_code,
                    message: body,
                }),
            }
        }
    }

    /// Get a file's metadata.
    pub async fn get_file(&self, id: &str) -> Result<File> {
        self.request(
            reqwest::Method::GET,
            &format!("/files/{}?fields=*", id),
            None::<&()>,
        )
        .await
    }

    /// List files.
    pub async fn list_files(
        &self,
        query: Option<&str>,
        page_token: Option<&str>,
        page_size: Option<u32>,
    ) -> Result<FileList> {
        let mut params = vec![("fields", "files(*),nextPageToken".to_string())];

        if let Some(q) = query {
            params.push(("q", q.to_string()));
        }
        if let Some(token) = page_token {
            params.push(("pageToken", token.to_string()));
        }
        if let Some(size) = page_size {
            params.push(("pageSize", size.to_string()));
        }

        let query_string: String = params
            .iter()
            .map(|(k, v)| format!("{}={}", k, urlencoding::encode(v)))
            .collect::<Vec<_>>()
            .join("&");

        self.request(
            reqwest::Method::GET,
            &format!("/files?{}", query_string),
            None::<&()>,
        )
        .await
    }

    /// Create a file (metadata only, for folders).
    pub async fn create_file(&self, metadata: FileMetadata) -> Result<File> {
        self.request(reqwest::Method::POST, "/files", Some(&metadata))
            .await
    }

    /// Create a file with content.
    pub async fn create_file_with_content(
        &self,
        metadata: FileMetadata,
        content: Vec<u8>,
        mime_type: &str,
    ) -> Result<File> {
        let url = format!(
            "{}{}?uploadType=multipart&fields=*",
            UPLOAD_BASE_URL, "/files"
        );
        debug!("Drive API upload: POST {}", url);

        let mut headers = HeaderMap::new();
        self.auth.apply(&mut headers).await?;

        let metadata_json = serde_json::to_string(&metadata)?;
        let form = reqwest::multipart::Form::new()
            .part(
                "metadata",
                reqwest::multipart::Part::text(metadata_json).mime_str("application/json")?,
            )
            .part(
                "file",
                reqwest::multipart::Part::bytes(content).mime_str(mime_type)?,
            );

        let response = self
            .http
            .post(&url)
            .headers(headers)
            .multipart(form)
            .send()
            .await?;
        self.handle_response(response).await
    }

    /// Update a file's metadata.
    pub async fn update_file(&self, id: &str, metadata: FileMetadata) -> Result<File> {
        self.request(
            reqwest::Method::PATCH,
            &format!("/files/{}?fields=*", id),
            Some(&metadata),
        )
        .await
    }

    /// Delete a file.
    pub async fn delete_file(&self, id: &str) -> Result<()> {
        let url = format!("{}/files/{}", self.base_url, id);
        debug!("Drive API request: DELETE {}", url);

        let mut headers = HeaderMap::new();
        self.auth.apply(&mut headers).await?;

        let response = self.http.delete(&url).headers(headers).send().await?;
        let status = response.status();

        if status.is_success() {
            Ok(())
        } else {
            let status_code = status.as_u16();
            let body = response.text().await.unwrap_or_default();
            Err(Error::Api {
                status: status_code,
                message: body,
            })
        }
    }

    /// Download a file's content.
    pub async fn download_file(&self, id: &str) -> Result<Bytes> {
        let url = format!("{}/files/{}?alt=media", self.base_url, id);
        debug!("Drive API download: GET {}", url);

        let mut headers = HeaderMap::new();
        self.auth.apply(&mut headers).await?;

        let response = self.http.get(&url).headers(headers).send().await?;
        let status = response.status();

        if status.is_success() {
            Ok(response.bytes().await?)
        } else {
            let status_code = status.as_u16();
            let body = response.text().await.unwrap_or_default();
            Err(Error::Api {
                status: status_code,
                message: body,
            })
        }
    }

    /// Create a folder.
    pub async fn create_folder(&self, name: &str, parent: Option<&str>) -> Result<File> {
        let mut metadata = FileMetadata::folder(name);
        if let Some(p) = parent {
            metadata = metadata.parent(p);
        }
        self.create_file(metadata).await
    }

    /// Share a file.
    pub async fn share_file(&self, id: &str, permission: Permission) -> Result<Permission> {
        self.request(
            reqwest::Method::POST,
            &format!("/files/{}/permissions", id),
            Some(&permission),
        )
        .await
    }

    /// List file permissions.
    pub async fn list_permissions(&self, id: &str) -> Result<PermissionList> {
        self.request(
            reqwest::Method::GET,
            &format!("/files/{}/permissions", id),
            None::<&()>,
        )
        .await
    }

    /// Get about info.
    pub async fn about(&self) -> Result<About> {
        self.request(
            reqwest::Method::GET,
            "/about?fields=user,storageQuota",
            None::<&()>,
        )
        .await
    }
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
