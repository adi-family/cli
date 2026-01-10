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
    /// Set the Jira instance URL (e.g., "https://your-domain.atlassian.net").
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

    async fn put<B: serde::Serialize>(&self, path: &str, body: &B) -> Result<()> {
        self.request_no_response(reqwest::Method::PUT, path, Some(body))
            .await
    }

    async fn request<T: DeserializeOwned>(
        &self,
        method: reqwest::Method,
        path: &str,
        body: Option<&impl serde::Serialize>,
    ) -> Result<T> {
        let url = format!("{}/rest/api/3{}", self.base_url, path);
        debug!("Jira API request: {} {}", method, url);

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

    async fn request_no_response(
        &self,
        method: reqwest::Method,
        path: &str,
        body: Option<&impl serde::Serialize>,
    ) -> Result<()> {
        let url = format!("{}/rest/api/3{}", self.base_url, path);
        debug!("Jira API request: {} {}", method, url);

        let mut headers = HeaderMap::new();
        self.auth.apply(&mut headers).await?;
        headers.insert("Content-Type", "application/json".parse().unwrap());

        let mut request = self.http.request(method, &url).headers(headers);

        if let Some(body) = body {
            request = request.json(body);
        }

        let response = request.send().await?;
        let status = response.status();

        if status.is_success() {
            Ok(())
        } else {
            let status_code = status.as_u16();
            let body = response.text().await.unwrap_or_default();
            warn!("Jira API error ({}): {}", status_code, body);

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

    async fn handle_response<T: DeserializeOwned>(&self, response: reqwest::Response) -> Result<T> {
        let status = response.status();

        if status.is_success() {
            let body = response.text().await?;
            serde_json::from_str(&body).map_err(Error::from)
        } else {
            let status_code = status.as_u16();
            let body = response.text().await.unwrap_or_default();
            warn!("Jira API error ({}): {}", status_code, body);

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

    /// Get an issue by key (e.g., "PROJ-123").
    pub async fn get_issue(&self, key: &str) -> Result<Issue> {
        self.get(&format!("/issue/{}", key)).await
    }

    /// Create an issue.
    pub async fn create_issue(&self, input: CreateIssueInput) -> Result<CreatedIssue> {
        self.post("/issue", &input).await
    }

    /// Update an issue.
    pub async fn update_issue(&self, key: &str, input: UpdateIssueInput) -> Result<()> {
        self.put(&format!("/issue/{}", key), &input).await
    }

    /// Search issues using JQL.
    pub async fn search_issues(&self, jql: &str, max_results: Option<u32>) -> Result<SearchResult> {
        let max = max_results.unwrap_or(50);
        let encoded_jql = urlencoding::encode(jql);
        self.get(&format!("/search?jql={}&maxResults={}", encoded_jql, max))
            .await
    }

    /// List all projects.
    pub async fn list_projects(&self) -> Result<Vec<Project>> {
        self.get("/project").await
    }

    /// Get a project by key.
    pub async fn get_project(&self, key: &str) -> Result<Project> {
        self.get(&format!("/project/{}", key)).await
    }

    /// Add a comment to an issue.
    pub async fn add_comment(&self, issue_key: &str, input: AddCommentInput) -> Result<Comment> {
        self.post(&format!("/issue/{}/comment", issue_key), &input)
            .await
    }

    /// Get available transitions for an issue.
    pub async fn get_transitions(&self, issue_key: &str) -> Result<Vec<Transition>> {
        #[derive(serde::Deserialize)]
        struct Response {
            transitions: Vec<Transition>,
        }

        let resp: Response = self.get(&format!("/issue/{}/transitions", issue_key)).await?;
        Ok(resp.transitions)
    }

    /// Transition an issue to a new status.
    pub async fn transition_issue(&self, issue_key: &str, transition_id: &str) -> Result<()> {
        let input = TransitionInput::new(transition_id);
        self.request_no_response(
            reqwest::Method::POST,
            &format!("/issue/{}/transitions", issue_key),
            Some(&input),
        )
        .await
    }

    /// Get the authenticated user.
    pub async fn myself(&self) -> Result<User> {
        self.get("/myself").await
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
            .base_url("https://example.atlassian.net")
            .build();
        assert!(result.is_ok());
    }
}
