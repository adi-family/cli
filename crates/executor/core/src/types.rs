use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::ApiError;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Package {
    /// Public GitHub Container Registry (ghcr.io)
    #[serde(rename = "github/public")]
    GithubPublic { image: String },

    /// Private GitHub Container Registry with token auth
    #[serde(rename = "github/private")]
    GithubPrivate {
        image: String,
        user: String,
        token: String,
    },

    /// Public Docker Hub
    #[serde(rename = "dockerhub/public")]
    DockerhubPublic { image: String },

    /// Private Docker Hub with credentials
    #[serde(rename = "dockerhub/private")]
    DockerhubPrivate {
        image: String,
        user: String,
        password: String,
    },

    /// Generic private registry with full URL and credentials
    #[serde(rename = "registry/private")]
    RegistryPrivate {
        url: String,
        user: String,
        password: String,
    },

    /// Generic public registry with full URL
    #[serde(rename = "registry/public")]
    RegistryPublic { url: String },
}

impl Package {
    /// Get the full image URL for Docker
    pub fn image_url(&self) -> String {
        match self {
            Self::GithubPublic { image } => format!("ghcr.io/{}", image),
            Self::GithubPrivate { image, .. } => format!("ghcr.io/{}", image),
            Self::DockerhubPublic { image } => image.clone(),
            Self::DockerhubPrivate { image, .. } => image.clone(),
            Self::RegistryPrivate { url, .. } => url.clone(),
            Self::RegistryPublic { url } => url.clone(),
        }
    }

    /// Get credentials if required
    pub fn credentials(&self) -> Option<(String, String)> {
        match self {
            Self::GithubPublic { .. } => None,
            Self::GithubPrivate { user, token, .. } => Some((user.clone(), token.clone())),
            Self::DockerhubPublic { .. } => None,
            Self::DockerhubPrivate { user, password, .. } => Some((user.clone(), password.clone())),
            Self::RegistryPrivate { user, password, .. } => Some((user.clone(), password.clone())),
            Self::RegistryPublic { .. } => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WorkerRequest {
    Message { message: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum OutputConfig {
    GithubBranch {
        repo: String,
        branch: String,
        token: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        commit_message: Option<String>,
    },
    Webhook {
        url: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        headers: Option<std::collections::HashMap<String, String>>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    Queued,
    Pulling,
    Running,
    ProcessingOutput,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub id: Uuid,
    pub status: JobStatus,
    pub package: Package,
    pub request: WorkerRequest,
    pub output: OutputConfig,
    pub created_at: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub started_at: Option<DateTime<Utc>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<DateTime<Utc>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<ApiError>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub container_id: Option<String>,
}

impl Job {
    pub fn new(package: Package, request: WorkerRequest, output: OutputConfig) -> Self {
        Self {
            id: Uuid::new_v4(),
            status: JobStatus::Queued,
            package,
            request,
            output,
            created_at: Utc::now(),
            started_at: None,
            completed_at: None,
            error: None,
            result: None,
            container_id: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifyResult {
    pub valid: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<ApiError>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerResponse {
    pub success: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<WorkerError>,
    #[serde(default)]
    pub files: Vec<OutputFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerError {
    pub code: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputFile {
    pub path: String,
    pub content: String,
    #[serde(default)]
    pub binary: bool,
}
