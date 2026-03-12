use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Error code slugs for API responses
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    // Docker errors
    DockerConnection,
    DockerApi,
    SocketSetupFailed,
    ContainerCreateFailed,
    ContainerStartFailed,
    ImagePullFailed,
    ImageNotFound,
    RegistryAuth,

    // Worker/execution errors
    WorkerInvalidResponse,
    ExecutionFailed,

    // Output errors
    OutputHandlerFailed,
    GitPushFailed,
    GitCloneFailed,
    GitAuthFailed,
    WebhookFailed,

    // Job errors
    JobNotFound,
    JobAlreadyExists,

    // Validation errors
    InvalidPackageType,
    InvalidImageFormat,
    InvalidUrl,
    InvalidUuid,
    InvalidConfig,
    MissingField,
    InvalidJson,

    // HTTP errors
    HttpRequestFailed,
    HttpTimeout,

    // Internal errors
    Internal,
}

/// Structured error for API responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiError {
    pub code: ErrorCode,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
}

impl ApiError {
    pub fn new(code: ErrorCode) -> Self {
        Self {
            code,
            details: None,
        }
    }

    pub fn with_details(code: ErrorCode, details: impl Into<String>) -> Self {
        Self {
            code,
            details: Some(details.into()),
        }
    }
}

#[derive(Debug, Error)]
pub enum ExecutorError {
    #[error("docker_connection")]
    DockerConnection(#[source] bollard::errors::Error),

    #[error("docker_api")]
    DockerApi(#[source] bollard::errors::Error),

    #[error("socket_setup_failed")]
    SocketSetupFailed(String),

    #[error("container_create_failed")]
    ContainerCreateFailed(String),

    #[error("container_start_failed")]
    ContainerStartFailed(String),

    #[error("image_pull_failed")]
    ImagePullFailed(String),

    #[error("image_not_found")]
    ImageNotFound(String),

    #[error("registry_auth")]
    RegistryAuth(String),

    #[error("worker_invalid_response")]
    WorkerInvalidResponse(String),

    #[error("output_handler_failed")]
    OutputHandlerFailed(String),

    #[error("git_push_failed")]
    GitPushFailed(String),

    #[error("git_clone_failed")]
    GitCloneFailed(String),

    #[error("git_auth_failed")]
    GitAuthFailed(String),

    #[error("webhook_failed")]
    WebhookFailed(String),

    #[error("job_not_found")]
    JobNotFound(uuid::Uuid),

    #[error("invalid_config")]
    InvalidConfig(String),

    #[error("http_request_failed")]
    HttpRequest(#[source] reqwest::Error),

    #[error("http_timeout")]
    HttpTimeout(String),

    #[error("internal")]
    Internal(String),
}

impl ExecutorError {
    pub fn code(&self) -> ErrorCode {
        match self {
            Self::DockerConnection(_) => ErrorCode::DockerConnection,
            Self::DockerApi(_) => ErrorCode::DockerApi,
            Self::SocketSetupFailed(_) => ErrorCode::SocketSetupFailed,
            Self::ContainerCreateFailed(_) => ErrorCode::ContainerCreateFailed,
            Self::ContainerStartFailed(_) => ErrorCode::ContainerStartFailed,
            Self::ImagePullFailed(_) => ErrorCode::ImagePullFailed,
            Self::ImageNotFound(_) => ErrorCode::ImageNotFound,
            Self::RegistryAuth(_) => ErrorCode::RegistryAuth,
            Self::WorkerInvalidResponse(_) => ErrorCode::WorkerInvalidResponse,
            Self::OutputHandlerFailed(_) => ErrorCode::OutputHandlerFailed,
            Self::GitPushFailed(_) => ErrorCode::GitPushFailed,
            Self::GitCloneFailed(_) => ErrorCode::GitCloneFailed,
            Self::GitAuthFailed(_) => ErrorCode::GitAuthFailed,
            Self::WebhookFailed(_) => ErrorCode::WebhookFailed,
            Self::JobNotFound(_) => ErrorCode::JobNotFound,
            Self::InvalidConfig(_) => ErrorCode::InvalidConfig,
            Self::HttpRequest(_) => ErrorCode::HttpRequestFailed,
            Self::HttpTimeout(_) => ErrorCode::HttpTimeout,
            Self::Internal(_) => ErrorCode::Internal,
        }
    }

    pub fn details(&self) -> Option<String> {
        match self {
            Self::DockerConnection(e) => Some(e.to_string()),
            Self::DockerApi(e) => Some(e.to_string()),
            Self::SocketSetupFailed(s) => Some(s.clone()),
            Self::ContainerCreateFailed(s) => Some(s.clone()),
            Self::ContainerStartFailed(s) => Some(s.clone()),
            Self::ImagePullFailed(s) => Some(s.clone()),
            Self::ImageNotFound(s) => Some(s.clone()),
            Self::RegistryAuth(s) => Some(s.clone()),
            Self::WorkerInvalidResponse(s) => Some(s.clone()),
            Self::OutputHandlerFailed(s) => Some(s.clone()),
            Self::GitPushFailed(s) => Some(s.clone()),
            Self::GitCloneFailed(s) => Some(s.clone()),
            Self::GitAuthFailed(s) => Some(s.clone()),
            Self::WebhookFailed(s) => Some(s.clone()),
            Self::JobNotFound(id) => Some(id.to_string()),
            Self::InvalidConfig(s) => Some(s.clone()),
            Self::HttpRequest(e) => Some(e.to_string()),
            Self::HttpTimeout(s) => Some(s.clone()),
            Self::Internal(s) => Some(s.clone()),
        }
    }

    pub fn to_api_error(&self) -> ApiError {
        ApiError {
            code: self.code(),
            details: self.details(),
        }
    }
}

impl From<bollard::errors::Error> for ExecutorError {
    fn from(e: bollard::errors::Error) -> Self {
        Self::DockerApi(e)
    }
}

impl From<reqwest::Error> for ExecutorError {
    fn from(e: reqwest::Error) -> Self {
        if e.is_timeout() {
            Self::HttpTimeout(e.to_string())
        } else {
            Self::HttpRequest(e)
        }
    }
}

pub type Result<T> = std::result::Result<T, ExecutorError>;
