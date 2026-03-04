use serde::{Deserialize, Serialize};

/// Project in GitLab.
#[derive(Debug, Clone, Deserialize)]
pub struct Project {
    pub id: u64,
    pub name: String,
    pub name_with_namespace: String,
    pub path: String,
    pub path_with_namespace: String,
    pub description: Option<String>,
    pub default_branch: Option<String>,
    pub web_url: String,
    pub ssh_url_to_repo: String,
    pub http_url_to_repo: String,
    pub visibility: String,
    pub created_at: String,
    pub last_activity_at: String,
}

/// Merge request.
#[derive(Debug, Clone, Deserialize)]
pub struct MergeRequest {
    pub id: u64,
    pub iid: u64,
    pub title: String,
    pub description: Option<String>,
    pub state: String,
    pub source_branch: String,
    pub target_branch: String,
    pub author: User,
    pub assignee: Option<User>,
    pub web_url: String,
    pub created_at: String,
    pub updated_at: String,
    pub merged_at: Option<String>,
    pub closed_at: Option<String>,
    pub draft: bool,
    pub merge_status: String,
}

/// Issue in GitLab.
#[derive(Debug, Clone, Deserialize)]
pub struct Issue {
    pub id: u64,
    pub iid: u64,
    pub title: String,
    pub description: Option<String>,
    pub state: String,
    pub author: User,
    pub assignee: Option<User>,
    pub labels: Vec<String>,
    pub web_url: String,
    pub created_at: String,
    pub updated_at: String,
    pub closed_at: Option<String>,
}

/// User in GitLab.
#[derive(Debug, Clone, Deserialize)]
pub struct User {
    pub id: u64,
    pub username: String,
    pub name: String,
    pub avatar_url: Option<String>,
    pub web_url: String,
}

/// Pipeline.
#[derive(Debug, Clone, Deserialize)]
pub struct Pipeline {
    pub id: u64,
    pub iid: u64,
    #[serde(rename = "ref")]
    pub git_ref: String,
    pub sha: String,
    pub status: String,
    pub source: String,
    pub web_url: String,
    pub created_at: String,
    pub updated_at: String,
}

/// Pipeline job.
#[derive(Debug, Clone, Deserialize)]
pub struct Job {
    pub id: u64,
    pub name: String,
    pub stage: String,
    pub status: String,
    pub web_url: String,
    pub created_at: String,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
}

/// Branch.
#[derive(Debug, Clone, Deserialize)]
pub struct Branch {
    pub name: String,
    pub commit: Commit,
    pub protected: bool,
    pub default: bool,
    pub web_url: String,
}

/// Commit.
#[derive(Debug, Clone, Deserialize)]
pub struct Commit {
    pub id: String,
    pub short_id: String,
    pub title: String,
    pub message: String,
    pub author_name: String,
    pub author_email: String,
    pub authored_date: String,
    pub committed_date: String,
    pub web_url: Option<String>,
}

/// File content.
#[derive(Debug, Clone, Deserialize)]
pub struct FileContent {
    pub file_name: String,
    pub file_path: String,
    pub size: u64,
    pub encoding: String,
    pub content: String,
    pub content_sha256: String,
    #[serde(rename = "ref")]
    pub git_ref: String,
    pub blob_id: String,
    pub commit_id: String,
    pub last_commit_id: String,
}

/// Input for creating a merge request.
#[derive(Debug, Clone, Default, Serialize)]
pub struct CreateMergeRequestInput {
    pub source_branch: String,
    pub target_branch: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assignee_id: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remove_source_branch: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub squash: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub labels: Option<String>,
}

impl CreateMergeRequestInput {
    pub fn new(
        source_branch: impl Into<String>,
        target_branch: impl Into<String>,
        title: impl Into<String>,
    ) -> Self {
        Self {
            source_branch: source_branch.into(),
            target_branch: target_branch.into(),
            title: title.into(),
            ..Default::default()
        }
    }

    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    pub fn assignee(mut self, id: u64) -> Self {
        self.assignee_id = Some(id);
        self
    }

    pub fn remove_source_branch(mut self, remove: bool) -> Self {
        self.remove_source_branch = Some(remove);
        self
    }
}

/// Input for creating an issue.
#[derive(Debug, Clone, Default, Serialize)]
pub struct CreateIssueInput {
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assignee_id: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub labels: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub milestone_id: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidential: Option<bool>,
}

impl CreateIssueInput {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            ..Default::default()
        }
    }

    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    pub fn assignee(mut self, id: u64) -> Self {
        self.assignee_id = Some(id);
        self
    }

    pub fn labels(mut self, labels: impl Into<String>) -> Self {
        self.labels = Some(labels.into());
        self
    }
}

/// Merge request state filter.
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum MergeRequestState {
    Opened,
    Closed,
    Merged,
    All,
}

/// Issue state filter.
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum IssueState {
    Opened,
    Closed,
    All,
}
