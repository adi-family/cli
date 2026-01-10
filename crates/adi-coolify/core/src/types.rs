//! Core types for Coolify operations.

use serde::{Deserialize, Serialize};

/// Service status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServiceStatus {
    /// Service is running and healthy
    Running,
    /// Service is running but unhealthy
    Unhealthy,
    /// Service is stopped
    Stopped,
    /// Service status is unknown
    Unknown,
    /// Service exited with error
    Exited,
}

impl ServiceStatus {
    /// Parse status from Coolify API string.
    pub fn from_api_status(status: &str) -> Self {
        match status {
            "running:healthy" | "running" => Self::Running,
            "running:unhealthy" | "running:unknown" => Self::Unhealthy,
            s if s.starts_with("exited") => Self::Exited,
            "stopped" => Self::Stopped,
            _ => Self::Unknown,
        }
    }

    /// Get status icon for display.
    pub fn icon(&self) -> &'static str {
        match self {
            Self::Running => "●",
            Self::Unhealthy => "◐",
            Self::Stopped | Self::Exited => "✗",
            Self::Unknown => "?",
        }
    }

    /// Get human-readable label.
    pub fn label(&self) -> &'static str {
        match self {
            Self::Running => "healthy",
            Self::Unhealthy => "unhealthy",
            Self::Stopped => "stopped",
            Self::Exited => "exited",
            Self::Unknown => "unknown",
        }
    }
}

/// Service definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Service {
    /// Internal service ID (short name)
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// Coolify UUID
    pub uuid: String,
    /// Current status
    #[serde(default)]
    pub status: Option<ServiceStatus>,
}

/// Deployment status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeploymentStatus {
    /// Deployment is queued
    Queued,
    /// Deployment is in progress
    InProgress,
    /// Deployment finished successfully
    Success,
    /// Deployment failed
    Failed,
    /// Deployment was cancelled
    Cancelled,
    /// Unknown status
    Unknown,
}

impl DeploymentStatus {
    /// Parse status from Coolify API string.
    pub fn from_api_status(status: &str) -> Self {
        match status {
            "queued" => Self::Queued,
            "in_progress" | "building" => Self::InProgress,
            "finished" | "success" => Self::Success,
            "failed" | "error" => Self::Failed,
            "cancelled" => Self::Cancelled,
            _ => Self::Unknown,
        }
    }

    /// Get status icon for display.
    pub fn icon(&self) -> &'static str {
        match self {
            Self::Queued => "○",
            Self::InProgress => "◐",
            Self::Success => "●",
            Self::Failed | Self::Cancelled => "✗",
            Self::Unknown => "?",
        }
    }

    /// Check if deployment is terminal (no longer running).
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::Success | Self::Failed | Self::Cancelled | Self::Unknown
        )
    }
}

/// Deployment information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Deployment {
    /// Deployment UUID
    pub uuid: String,
    /// Current status
    pub status: DeploymentStatus,
    /// Git commit (short hash)
    pub commit: Option<String>,
    /// Creation timestamp
    pub created_at: Option<String>,
    /// Deployment logs
    pub logs: Option<String>,
}
