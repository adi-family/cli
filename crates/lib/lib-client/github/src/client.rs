use std::sync::Arc;

use crate::auth::AuthStrategy;
use crate::error::{GitHubError, Result};
use crate::types::*;
use base64::{engine::general_purpose::STANDARD, Engine};
use reqwest::{header, Client as HttpClient, Response, StatusCode};
use serde::de::DeserializeOwned;
use serde_json::json;
use tracing::debug;

const GITHUB_API_URL: &str = "https://api.github.com";
const GITHUB_API_VERSION: &str = "2022-11-28";

pub struct ClientBuilder<A> {
    auth: A,
    base_url: String,
    user_agent: String,
}

impl ClientBuilder<()> {
    pub fn new() -> Self {
        Self {
            auth: (),
            base_url: GITHUB_API_URL.to_string(),
            user_agent: "lib-github-client".to_string(),
        }
    }
}

impl Default for ClientBuilder<()> {
    fn default() -> Self {
        Self::new()
    }
}

impl<A> ClientBuilder<A> {
    pub fn auth<S: AuthStrategy + 'static>(self, auth: S) -> ClientBuilder<S> {
        ClientBuilder {
            auth,
            base_url: self.base_url,
            user_agent: self.user_agent,
        }
    }

    pub fn base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    pub fn user_agent(mut self, agent: impl Into<String>) -> Self {
        self.user_agent = agent.into();
        self
    }
}

impl<A: AuthStrategy + 'static> ClientBuilder<A> {
    pub fn build(self) -> Result<Client> {
        let mut headers = header::HeaderMap::new();
        headers.insert(
            header::ACCEPT,
            header::HeaderValue::from_static("application/vnd.github+json"),
        );
        headers.insert(
            "X-GitHub-Api-Version",
            header::HeaderValue::from_static(GITHUB_API_VERSION),
        );

        let http = HttpClient::builder()
            .default_headers(headers)
            .user_agent(&self.user_agent)
            .build()?;

        Ok(Client {
            http,
            auth: Arc::new(self.auth),
            base_url: self.base_url,
        })
    }
}

pub struct Client {
    http: HttpClient,
    auth: Arc<dyn AuthStrategy>,
    base_url: String,
}

impl Client {
    pub fn builder() -> ClientBuilder<()> {
        ClientBuilder::new()
    }

    pub fn new(auth: impl AuthStrategy + 'static) -> Result<Self> {
        Self::builder().auth(auth).build()
    }

    async fn request<T: DeserializeOwned>(&self, method: reqwest::Method, path: &str) -> Result<T> {
        let url = self.url(path);
        debug!("{} {}", method, url);

        let mut headers = header::HeaderMap::new();
        self.auth.apply(&mut headers).await?;

        let response = self
            .http
            .request(method, &url)
            .headers(headers)
            .send()
            .await?;

        self.handle_response(response).await
    }

    async fn request_with_body<T: DeserializeOwned, B: serde::Serialize>(
        &self,
        method: reqwest::Method,
        path: &str,
        body: &B,
    ) -> Result<T> {
        let url = self.url(path);
        debug!("{} {}", method, url);

        let mut headers = header::HeaderMap::new();
        self.auth.apply(&mut headers).await?;

        let response = self
            .http
            .request(method, &url)
            .headers(headers)
            .json(body)
            .send()
            .await?;

        self.handle_response(response).await
    }

    async fn handle_response<T: DeserializeOwned>(&self, response: Response) -> Result<T> {
        let status = response.status();

        if status.is_success() {
            return Ok(response.json().await?);
        }

        let body = response.text().await.unwrap_or_default();
        debug!("GitHub API error response: {}", body);

        match status {
            StatusCode::UNAUTHORIZED => Err(GitHubError::Unauthorized),
            StatusCode::FORBIDDEN => {
                if body.contains("rate limit") {
                    Err(GitHubError::RateLimited { retry_after: 60 })
                } else {
                    Err(GitHubError::Forbidden)
                }
            }
            StatusCode::NOT_FOUND => Err(GitHubError::NotFound(body)),
            _ => Err(GitHubError::Api {
                status: status.as_u16(),
                message: body,
            }),
        }
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }

    // Repository operations

    pub async fn get_repo(&self, owner: &str, repo: &str) -> Result<Repository> {
        self.request(reqwest::Method::GET, &format!("/repos/{}/{}", owner, repo))
            .await
    }

    pub async fn list_branches(&self, owner: &str, repo: &str) -> Result<Vec<Branch>> {
        self.request(
            reqwest::Method::GET,
            &format!("/repos/{}/{}/branches", owner, repo),
        )
        .await
    }

    pub async fn get_branch(&self, owner: &str, repo: &str, branch: &str) -> Result<Branch> {
        self.request(
            reqwest::Method::GET,
            &format!("/repos/{}/{}/branches/{}", owner, repo, branch),
        )
        .await
    }

    // Content operations

    pub async fn get_content(
        &self,
        owner: &str,
        repo: &str,
        path: &str,
        git_ref: Option<&str>,
    ) -> Result<FileContent> {
        let path = match git_ref {
            Some(r) => format!("/repos/{}/{}/contents/{}?ref={}", owner, repo, path, r),
            None => format!("/repos/{}/{}/contents/{}", owner, repo, path),
        };
        self.request(reqwest::Method::GET, &path).await
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn create_or_update_file(
        &self,
        owner: &str,
        repo: &str,
        path: &str,
        message: &str,
        content: &str,
        sha: Option<&str>,
        branch: Option<&str>,
    ) -> Result<serde_json::Value> {
        let content_base64 = STANDARD.encode(content.as_bytes());

        let mut body = json!({
            "message": message,
            "content": content_base64,
        });

        if let Some(s) = sha {
            body["sha"] = json!(s);
        }
        if let Some(b) = branch {
            body["branch"] = json!(b);
        }

        self.request_with_body(
            reqwest::Method::PUT,
            &format!("/repos/{}/{}/contents/{}", owner, repo, path),
            &body,
        )
        .await
    }

    // Git Data operations

    pub async fn get_ref(&self, owner: &str, repo: &str, ref_path: &str) -> Result<Reference> {
        self.request(
            reqwest::Method::GET,
            &format!("/repos/{}/{}/git/ref/{}", owner, repo, ref_path),
        )
        .await
    }

    pub async fn create_ref(
        &self,
        owner: &str,
        repo: &str,
        ref_name: &str,
        sha: &str,
    ) -> Result<Reference> {
        let body = json!({
            "ref": ref_name,
            "sha": sha,
        });

        self.request_with_body(
            reqwest::Method::POST,
            &format!("/repos/{}/{}/git/refs", owner, repo),
            &body,
        )
        .await
    }

    pub async fn update_ref(
        &self,
        owner: &str,
        repo: &str,
        ref_path: &str,
        sha: &str,
        force: bool,
    ) -> Result<Reference> {
        let body = json!({
            "sha": sha,
            "force": force,
        });

        self.request_with_body(
            reqwest::Method::PATCH,
            &format!("/repos/{}/{}/git/refs/{}", owner, repo, ref_path),
            &body,
        )
        .await
    }

    pub async fn get_tree(
        &self,
        owner: &str,
        repo: &str,
        tree_sha: &str,
        recursive: bool,
    ) -> Result<Tree> {
        let path = if recursive {
            format!(
                "/repos/{}/{}/git/trees/{}?recursive=1",
                owner, repo, tree_sha
            )
        } else {
            format!("/repos/{}/{}/git/trees/{}", owner, repo, tree_sha)
        };
        self.request(reqwest::Method::GET, &path).await
    }

    pub async fn create_tree(
        &self,
        owner: &str,
        repo: &str,
        base_tree: Option<&str>,
        entries: Vec<CreateTreeEntry>,
    ) -> Result<Tree> {
        let mut body = json!({ "tree": entries });
        if let Some(base) = base_tree {
            body["base_tree"] = json!(base);
        }

        self.request_with_body(
            reqwest::Method::POST,
            &format!("/repos/{}/{}/git/trees", owner, repo),
            &body,
        )
        .await
    }

    pub async fn create_commit(
        &self,
        owner: &str,
        repo: &str,
        message: &str,
        tree_sha: &str,
        parents: Vec<&str>,
    ) -> Result<serde_json::Value> {
        let body = json!({
            "message": message,
            "tree": tree_sha,
            "parents": parents,
        });

        self.request_with_body(
            reqwest::Method::POST,
            &format!("/repos/{}/{}/git/commits", owner, repo),
            &body,
        )
        .await
    }

    pub async fn create_blob(
        &self,
        owner: &str,
        repo: &str,
        content: &str,
        encoding: &str,
    ) -> Result<Blob> {
        let body = json!({
            "content": content,
            "encoding": encoding,
        });

        self.request_with_body(
            reqwest::Method::POST,
            &format!("/repos/{}/{}/git/blobs", owner, repo),
            &body,
        )
        .await
    }

    // User operations

    pub async fn get_authenticated_user(&self) -> Result<User> {
        self.request(reqwest::Method::GET, "/user").await
    }

    // Release operations

    pub async fn list_releases(&self, owner: &str, repo: &str) -> Result<Vec<Release>> {
        self.request(
            reqwest::Method::GET,
            &format!("/repos/{}/{}/releases", owner, repo),
        )
        .await
    }

    pub async fn get_latest_release(&self, owner: &str, repo: &str) -> Result<Release> {
        self.request(
            reqwest::Method::GET,
            &format!("/repos/{}/{}/releases/latest", owner, repo),
        )
        .await
    }

    pub async fn get_release_by_tag(&self, owner: &str, repo: &str, tag: &str) -> Result<Release> {
        self.request(
            reqwest::Method::GET,
            &format!("/repos/{}/{}/releases/tags/{}", owner, repo, tag),
        )
        .await
    }

    pub async fn list_release_assets(
        &self,
        owner: &str,
        repo: &str,
        release_id: u64,
    ) -> Result<Vec<ReleaseAsset>> {
        self.request(
            reqwest::Method::GET,
            &format!("/repos/{}/{}/releases/{}/assets", owner, repo, release_id),
        )
        .await
    }

    pub async fn download_asset(&self, url: &str) -> Result<bytes::Bytes> {
        debug!("GET {} (binary)", url);

        let mut headers = header::HeaderMap::new();
        self.auth.apply(&mut headers).await?;
        headers.insert(header::ACCEPT, "application/octet-stream".parse().unwrap());

        let response = self.http.get(url).headers(headers).send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(GitHubError::Api {
                status: status.as_u16(),
                message: body,
            });
        }

        Ok(response.bytes().await?)
    }
}
