use reqwest::header::HeaderMap;
use serde::de::DeserializeOwned;
use std::sync::Arc;
use tracing::{debug, warn};

use crate::auth::AuthStrategy;
use crate::error::{Error, Result};
use crate::graphql::{GraphQLRequest, GraphQLResponse};
use crate::types::*;

const DEFAULT_BASE_URL: &str = "https://api.linear.app/graphql";

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

    /// Execute a GraphQL query.
    pub async fn query<T: DeserializeOwned>(&self, request: GraphQLRequest) -> Result<T> {
        debug!("Linear GraphQL query");

        let mut headers = HeaderMap::new();
        self.auth.apply(&mut headers).await?;
        headers.insert("Content-Type", "application/json".parse().unwrap());

        let response = self
            .http
            .post(&self.base_url)
            .headers(headers)
            .json(&request)
            .send()
            .await?;

        let status = response.status();

        if status.is_success() {
            let result: GraphQLResponse<T> = response.json().await?;

            if let Some(errors) = result.errors {
                let messages: Vec<String> = errors.into_iter().map(|e| e.message).collect();
                return Err(Error::GraphQL(messages.join("; ")));
            }

            result
                .data
                .ok_or_else(|| Error::GraphQL("No data returned".to_string()))
        } else {
            let status_code = status.as_u16();
            let body = response.text().await.unwrap_or_default();
            warn!("Linear API error ({}): {}", status_code, body);

            match status_code {
                401 => Err(Error::Unauthorized),
                429 => Err(Error::RateLimited { retry_after: 60 }),
                _ => Err(Error::Api {
                    status: status_code,
                    message: body,
                }),
            }
        }
    }

    /// Get an issue by ID.
    pub async fn get_issue(&self, id: &str) -> Result<Issue> {
        let query = r#"
            query GetIssue($id: String!) {
                issue(id: $id) {
                    id identifier title description priority url
                    createdAt updatedAt completedAt
                    state { id name color type }
                    assignee { id name email displayName avatarUrl }
                    project { id name description state url createdAt updatedAt }
                    team { id name key description }
                    labels { nodes { id name color } }
                }
            }
        "#;

        #[derive(serde::Deserialize)]
        struct Response {
            issue: Issue,
        }

        let request = GraphQLRequest::new(query).with_variables(serde_json::json!({ "id": id }));

        let response: Response = self.query(request).await?;
        Ok(response.issue)
    }

    /// Create an issue.
    pub async fn create_issue(&self, input: IssueCreateInput) -> Result<Issue> {
        let query = r#"
            mutation CreateIssue($input: IssueCreateInput!) {
                issueCreate(input: $input) {
                    success
                    issue {
                        id identifier title description priority url
                        createdAt updatedAt completedAt
                        state { id name color type }
                        assignee { id name email displayName avatarUrl }
                        team { id name key description }
                    }
                }
            }
        "#;

        #[derive(serde::Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Response {
            issue_create: IssuePayload,
        }

        let request =
            GraphQLRequest::new(query).with_variables(serde_json::json!({ "input": input }));

        let response: Response = self.query(request).await?;
        response
            .issue_create
            .issue
            .ok_or_else(|| Error::GraphQL("Issue creation failed".to_string()))
    }

    /// Update an issue.
    pub async fn update_issue(&self, id: &str, input: IssueUpdateInput) -> Result<Issue> {
        let query = r#"
            mutation UpdateIssue($id: String!, $input: IssueUpdateInput!) {
                issueUpdate(id: $id, input: $input) {
                    success
                    issue {
                        id identifier title description priority url
                        createdAt updatedAt completedAt
                        state { id name color type }
                        assignee { id name email displayName avatarUrl }
                        team { id name key description }
                    }
                }
            }
        "#;

        #[derive(serde::Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Response {
            issue_update: IssuePayload,
        }

        let request = GraphQLRequest::new(query)
            .with_variables(serde_json::json!({ "id": id, "input": input }));

        let response: Response = self.query(request).await?;
        response
            .issue_update
            .issue
            .ok_or_else(|| Error::GraphQL("Issue update failed".to_string()))
    }

    /// List issues with optional filter.
    pub async fn list_issues(
        &self,
        filter: Option<IssueFilter>,
        first: Option<i32>,
        after: Option<&str>,
    ) -> Result<IssueConnection> {
        let query = r#"
            query ListIssues($filter: IssueFilter, $first: Int, $after: String) {
                issues(filter: $filter, first: $first, after: $after) {
                    nodes {
                        id identifier title description priority url
                        createdAt updatedAt completedAt
                        state { id name color type }
                        assignee { id name email displayName avatarUrl }
                        team { id name key description }
                    }
                    pageInfo {
                        hasNextPage hasPreviousPage startCursor endCursor
                    }
                }
            }
        "#;

        #[derive(serde::Deserialize)]
        struct Response {
            issues: IssueConnection,
        }

        let variables = serde_json::json!({
            "filter": filter,
            "first": first.unwrap_or(50),
            "after": after,
        });

        let request = GraphQLRequest::new(query).with_variables(variables);
        let response: Response = self.query(request).await?;
        Ok(response.issues)
    }

    /// List all teams.
    pub async fn list_teams(&self) -> Result<Vec<Team>> {
        let query = r#"
            query ListTeams {
                teams {
                    nodes { id name key description }
                }
            }
        "#;

        #[derive(serde::Deserialize)]
        struct TeamsConnection {
            nodes: Vec<Team>,
        }

        #[derive(serde::Deserialize)]
        struct Response {
            teams: TeamsConnection,
        }

        let request = GraphQLRequest::new(query);
        let response: Response = self.query(request).await?;
        Ok(response.teams.nodes)
    }

    /// List all projects.
    pub async fn list_projects(&self) -> Result<Vec<Project>> {
        let query = r#"
            query ListProjects {
                projects {
                    nodes { id name description state url createdAt updatedAt }
                }
            }
        "#;

        #[derive(serde::Deserialize)]
        struct ProjectsConnection {
            nodes: Vec<Project>,
        }

        #[derive(serde::Deserialize)]
        struct Response {
            projects: ProjectsConnection,
        }

        let request = GraphQLRequest::new(query);
        let response: Response = self.query(request).await?;
        Ok(response.projects.nodes)
    }

    /// Get the authenticated user.
    pub async fn viewer(&self) -> Result<User> {
        let query = r#"
            query Viewer {
                viewer { id name email displayName avatarUrl }
            }
        "#;

        #[derive(serde::Deserialize)]
        struct Response {
            viewer: User,
        }

        let request = GraphQLRequest::new(query);
        let response: Response = self.query(request).await?;
        Ok(response.viewer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::ApiKeyAuth;

    #[test]
    fn test_builder() {
        let client = Client::builder()
            .auth(ApiKeyAuth::new("lin_api_key"))
            .build();
        assert_eq!(client.base_url, DEFAULT_BASE_URL);
    }
}
