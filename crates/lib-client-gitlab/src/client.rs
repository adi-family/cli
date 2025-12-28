use reqwest::header::HeaderMap;
use serde::de::DeserializeOwned;
use std::sync::Arc;
use tracing::{debug, warn};

use crate::auth::AuthStrategy;
use crate::error::{Error, Result};
use crate::types::*;

const DEFAULT_BASE_URL: &str = "https://gitlab.com/api/v4";

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

    async fn request<T: DeserializeOwned>(
        &self,
        method: reqwest::Method,
        path: &str,
        body: Option<&impl serde::Serialize>,
    ) -> Result<T> {
        let url = format!("{}{}", self.base_url, path);
        debug!("GitLab API request: {} {}", method, url);

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
            warn!("GitLab API error ({}): {}", status_code, body);

            match status_code {
                401 => Err(Error::Unauthorized),
                403 => Err(Error::Forbidden(body)),
                404 => Err(Error::NotFound(body)),
                409 => Err(Error::Conflict(body)),
                429 => {
                    let retry_after = 60;
                    Err(Error::RateLimited { retry_after })
                }
                _ => Err(Error::Api {
                    status: status_code,
                    message: body,
                }),
            }
        }
    }

    fn encode_project(&self, project: &str) -> String {
        urlencoding::encode(project).to_string()
    }

    /// Get a project by ID or path.
    pub async fn get_project(&self, project: &str) -> Result<Project> {
        let encoded = self.encode_project(project);
        self.get(&format!("/projects/{}", encoded)).await
    }

    /// List merge requests for a project.
    pub async fn list_merge_requests(
        &self,
        project: &str,
        state: Option<MergeRequestState>,
    ) -> Result<Vec<MergeRequest>> {
        let encoded = self.encode_project(project);
        let state_param = state
            .map(|s| format!("?state={:?}", s).to_lowercase())
            .unwrap_or_default();
        self.get(&format!("/projects/{}/merge_requests{}", encoded, state_param))
            .await
    }

    /// Get a merge request by IID.
    pub async fn get_merge_request(&self, project: &str, mr_iid: u64) -> Result<MergeRequest> {
        let encoded = self.encode_project(project);
        self.get(&format!("/projects/{}/merge_requests/{}", encoded, mr_iid))
            .await
    }

    /// Create a merge request.
    pub async fn create_merge_request(
        &self,
        project: &str,
        input: CreateMergeRequestInput,
    ) -> Result<MergeRequest> {
        let encoded = self.encode_project(project);
        self.post(&format!("/projects/{}/merge_requests", encoded), &input)
            .await
    }

    /// List issues for a project.
    pub async fn list_issues(
        &self,
        project: &str,
        state: Option<IssueState>,
    ) -> Result<Vec<Issue>> {
        let encoded = self.encode_project(project);
        let state_param = state
            .map(|s| format!("?state={:?}", s).to_lowercase())
            .unwrap_or_default();
        self.get(&format!("/projects/{}/issues{}", encoded, state_param))
            .await
    }

    /// Get an issue by IID.
    pub async fn get_issue(&self, project: &str, issue_iid: u64) -> Result<Issue> {
        let encoded = self.encode_project(project);
        self.get(&format!("/projects/{}/issues/{}", encoded, issue_iid))
            .await
    }

    /// Create an issue.
    pub async fn create_issue(&self, project: &str, input: CreateIssueInput) -> Result<Issue> {
        let encoded = self.encode_project(project);
        self.post(&format!("/projects/{}/issues", encoded), &input)
            .await
    }

    /// List pipelines for a project.
    pub async fn list_pipelines(&self, project: &str) -> Result<Vec<Pipeline>> {
        let encoded = self.encode_project(project);
        self.get(&format!("/projects/{}/pipelines", encoded)).await
    }

    /// Get a pipeline by ID.
    pub async fn get_pipeline(&self, project: &str, pipeline_id: u64) -> Result<Pipeline> {
        let encoded = self.encode_project(project);
        self.get(&format!("/projects/{}/pipelines/{}", encoded, pipeline_id))
            .await
    }

    /// List jobs for a pipeline.
    pub async fn list_pipeline_jobs(&self, project: &str, pipeline_id: u64) -> Result<Vec<Job>> {
        let encoded = self.encode_project(project);
        self.get(&format!(
            "/projects/{}/pipelines/{}/jobs",
            encoded, pipeline_id
        ))
        .await
    }

    /// List branches for a project.
    pub async fn list_branches(&self, project: &str) -> Result<Vec<Branch>> {
        let encoded = self.encode_project(project);
        self.get(&format!("/projects/{}/repository/branches", encoded))
            .await
    }

    /// Get a file from the repository.
    pub async fn get_file(&self, project: &str, path: &str, git_ref: &str) -> Result<FileContent> {
        let encoded_project = self.encode_project(project);
        let encoded_path = urlencoding::encode(path);
        self.get(&format!(
            "/projects/{}/repository/files/{}?ref={}",
            encoded_project, encoded_path, git_ref
        ))
        .await
    }

    /// List commits for a project.
    pub async fn list_commits(&self, project: &str, git_ref: Option<&str>) -> Result<Vec<Commit>> {
        let encoded = self.encode_project(project);
        let ref_param = git_ref
            .map(|r| format!("?ref_name={}", r))
            .unwrap_or_default();
        self.get(&format!(
            "/projects/{}/repository/commits{}",
            encoded, ref_param
        ))
        .await
    }

    /// Get the authenticated user.
    pub async fn current_user(&self) -> Result<User> {
        self.get("/user").await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::PrivateTokenAuth;

    #[test]
    fn test_builder() {
        let client = Client::builder()
            .auth(PrivateTokenAuth::new("glpat-xxx"))
            .base_url("https://gitlab.example.com/api/v4")
            .build();
        assert_eq!(client.base_url, "https://gitlab.example.com/api/v4");
    }

    #[test]
    fn test_encode_project() {
        let client = Client::builder()
            .auth(PrivateTokenAuth::new("test"))
            .build();
        assert_eq!(
            client.encode_project("group/project"),
            "group%2Fproject"
        );
    }
}
