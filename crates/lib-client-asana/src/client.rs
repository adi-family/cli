use reqwest::header::HeaderMap;
use serde::{de::DeserializeOwned, Deserialize};
use std::sync::Arc;
use tracing::{debug, warn};

use crate::auth::AuthStrategy;
use crate::error::{Error, Result};
use crate::types::*;

const DEFAULT_BASE_URL: &str = "https://app.asana.com/api/1.0";

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
        debug!("Asana API request: {} {}", method, url);

        let mut headers = HeaderMap::new();
        self.auth.apply(&mut headers).await?;
        headers.insert("Content-Type", "application/json".parse().unwrap());

        let mut request = self.http.request(method, &url).headers(headers);

        if let Some(body) = body {
            request = request.json(&serde_json::json!({ "data": body }));
        }

        let response = request.send().await?;
        self.handle_response(response).await
    }

    async fn handle_response<T: DeserializeOwned>(&self, response: reqwest::Response) -> Result<T> {
        let status = response.status();

        if status.is_success() {
            let body = response.text().await?;
            let resp: AsanaResponse<T> = serde_json::from_str(&body)?;
            Ok(resp.data)
        } else {
            let status_code = status.as_u16();
            let body = response.text().await.unwrap_or_default();
            warn!("Asana API error ({}): {}", status_code, body);

            match status_code {
                401 => Err(Error::Unauthorized),
                403 => Err(Error::Forbidden(body)),
                404 => Err(Error::NotFound(body)),
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

    /// Get a task by GID.
    pub async fn get_task(&self, gid: &str) -> Result<Task> {
        self.get(&format!("/tasks/{}", gid)).await
    }

    /// Create a task.
    pub async fn create_task(&self, input: CreateTaskInput) -> Result<Task> {
        self.post("/tasks", &input).await
    }

    /// Update a task.
    pub async fn update_task(&self, gid: &str, input: UpdateTaskInput) -> Result<Task> {
        self.put(&format!("/tasks/{}", gid), &input).await
    }

    /// Complete a task.
    pub async fn complete_task(&self, gid: &str) -> Result<Task> {
        self.update_task(gid, UpdateTaskInput::new().completed(true))
            .await
    }

    /// List tasks in a project.
    pub async fn list_tasks(&self, project_gid: &str) -> Result<Vec<Task>> {
        #[derive(Deserialize)]
        struct Response {
            data: Vec<Task>,
        }

        let url = format!("{}/projects/{}/tasks?opt_fields=name,notes,completed,due_on,assignee,created_at,modified_at,permalink_url", self.base_url, project_gid);

        let mut headers = HeaderMap::new();
        self.auth.apply(&mut headers).await?;

        let response = self.http.get(&url).headers(headers).send().await?;

        if response.status().is_success() {
            let body = response.text().await?;
            let resp: Response = serde_json::from_str(&body)?;
            Ok(resp.data)
        } else {
            let status = response.status().as_u16();
            let body = response.text().await.unwrap_or_default();
            Err(Error::Api { status, message: body })
        }
    }

    /// List projects in a workspace.
    pub async fn list_projects(&self, workspace_gid: &str) -> Result<Vec<Project>> {
        #[derive(Deserialize)]
        struct Response {
            data: Vec<Project>,
        }

        let url = format!("{}/workspaces/{}/projects", self.base_url, workspace_gid);

        let mut headers = HeaderMap::new();
        self.auth.apply(&mut headers).await?;

        let response = self.http.get(&url).headers(headers).send().await?;

        if response.status().is_success() {
            let body = response.text().await?;
            let resp: Response = serde_json::from_str(&body)?;
            Ok(resp.data)
        } else {
            let status = response.status().as_u16();
            let body = response.text().await.unwrap_or_default();
            Err(Error::Api { status, message: body })
        }
    }

    /// List workspaces.
    pub async fn list_workspaces(&self) -> Result<Vec<Workspace>> {
        #[derive(Deserialize)]
        struct Response {
            data: Vec<Workspace>,
        }

        let url = format!("{}/workspaces", self.base_url);

        let mut headers = HeaderMap::new();
        self.auth.apply(&mut headers).await?;

        let response = self.http.get(&url).headers(headers).send().await?;

        if response.status().is_success() {
            let body = response.text().await?;
            let resp: Response = serde_json::from_str(&body)?;
            Ok(resp.data)
        } else {
            let status = response.status().as_u16();
            let body = response.text().await.unwrap_or_default();
            Err(Error::Api { status, message: body })
        }
    }

    /// Add task to a project.
    pub async fn add_task_to_project(&self, task_gid: &str, input: AddToProjectInput) -> Result<()> {
        let url = format!("{}/tasks/{}/addProject", self.base_url, task_gid);

        let mut headers = HeaderMap::new();
        self.auth.apply(&mut headers).await?;
        headers.insert("Content-Type", "application/json".parse().unwrap());

        let response = self
            .http
            .post(&url)
            .headers(headers)
            .json(&serde_json::json!({ "data": input }))
            .send()
            .await?;

        if response.status().is_success() {
            Ok(())
        } else {
            let status = response.status().as_u16();
            let body = response.text().await.unwrap_or_default();
            Err(Error::Api { status, message: body })
        }
    }

    /// Get the authenticated user.
    pub async fn me(&self) -> Result<User> {
        self.get("/users/me").await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::BearerAuth;

    #[test]
    fn test_builder() {
        let client = Client::builder()
            .auth(BearerAuth::new("test-token"))
            .build();
        assert_eq!(client.base_url, DEFAULT_BASE_URL);
    }
}
