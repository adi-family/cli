//! Coolify API client.

use crate::error::{CoolifyError, Result};
use crate::types::{Deployment, DeploymentStatus, Service, ServiceStatus};
use reqwest::Client;
use url::Url;

/// Async Coolify API client.
#[derive(Debug, Clone)]
pub struct CoolifyClient {
    base_url: Url,
    api_key: String,
    client: Client,
}

impl CoolifyClient {
    /// Create a new Coolify client.
    pub fn new(base_url: &str, api_key: &str) -> Result<Self> {
        let base_url = Url::parse(base_url)?;
        let client = Client::new();

        Ok(Self {
            base_url,
            api_key: api_key.to_string(),
            client,
        })
    }

    /// Make an authenticated GET request.
    async fn get(&self, endpoint: &str) -> Result<serde_json::Value> {
        let url = self.base_url.join(&format!("/api/v1{}", endpoint))?;

        let response = self
            .client
            .get(url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .send()
            .await?;

        if response.status().is_client_error() {
            return Err(CoolifyError::AuthFailed(
                "Invalid API key or unauthorized".to_string(),
            ));
        }

        let text = response.text().await?;
        let json: serde_json::Value = serde_json::from_str(&text)?;

        Ok(json)
    }

    /// Get application status.
    pub async fn get_application_status(&self, uuid: &str) -> Result<ServiceStatus> {
        let result = self.get(&format!("/applications/{}", uuid)).await?;

        let status_str = result
            .get("status")
            .and_then(|s| s.as_str())
            .unwrap_or("unknown");

        Ok(ServiceStatus::from_api_status(status_str))
    }

    /// Get service status with full info.
    pub async fn get_service_status(&self, service: &Service) -> Result<Service> {
        let status = self.get_application_status(&service.uuid).await?;

        Ok(Service {
            id: service.id.clone(),
            name: service.name.clone(),
            uuid: service.uuid.clone(),
            status: Some(status),
        })
    }

    /// Deploy an application.
    pub async fn deploy(&self, uuid: &str, force: bool) -> Result<Deployment> {
        let force_param = if force { "&force=true" } else { "" };
        let endpoint = format!("/deploy?uuid={}{}", uuid, force_param);

        let result = self.get(&endpoint).await?;

        // Extract deployment UUID from response
        let deployment_uuid = result
            .get("deployments")
            .and_then(|d| d.as_array())
            .and_then(|arr| arr.first())
            .and_then(|d| d.get("deployment_uuid"))
            .and_then(|u| u.as_str())
            .ok_or_else(|| CoolifyError::Api {
                message: result
                    .get("message")
                    .or_else(|| result.get("error"))
                    .and_then(|e| e.as_str())
                    .unwrap_or("Unknown error")
                    .to_string(),
                code: None,
            })?;

        Ok(Deployment {
            uuid: deployment_uuid.to_string(),
            status: DeploymentStatus::Queued,
            commit: None,
            created_at: None,
            logs: None,
        })
    }

    /// Get recent deployments for an application.
    pub async fn get_deployments(&self, uuid: &str, take: u32) -> Result<Vec<Deployment>> {
        let endpoint = format!("/applications/{}/deployments?take={}", uuid, take);
        let result = self.get(&endpoint).await?;

        let deployments = result
            .as_array()
            .ok_or_else(|| CoolifyError::Api {
                message: "Invalid response format".to_string(),
                code: None,
            })?
            .iter()
            .map(|d| {
                let status_str = d
                    .get("status")
                    .and_then(|s| s.as_str())
                    .unwrap_or("unknown");
                let commit = d
                    .get("commit")
                    .and_then(|c| c.as_str())
                    .map(|c| c.chars().take(7).collect());
                let created_at = d
                    .get("created_at")
                    .and_then(|c| c.as_str())
                    .map(|s| s.to_string());
                let uuid = d
                    .get("deployment_uuid")
                    .and_then(|u| u.as_str())
                    .unwrap_or("unknown")
                    .to_string();

                Deployment {
                    uuid,
                    status: DeploymentStatus::from_api_status(status_str),
                    commit,
                    created_at,
                    logs: None,
                }
            })
            .collect();

        Ok(deployments)
    }

    /// Get deployment logs.
    pub async fn get_deployment_logs(&self, deployment_uuid: &str) -> Result<String> {
        let endpoint = format!("/deployments/{}", deployment_uuid);
        let result = self.get(&endpoint).await?;

        let logs = result
            .get("logs")
            .and_then(|l| l.as_str())
            .unwrap_or("No logs available")
            .to_string();

        Ok(logs)
    }
}
