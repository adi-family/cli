//! Service Exposure System
//!
//! Enables cross-source dependencies through expose/uses declarations.
//! Services can expose variables and ports to other sources, and
//! consuming services can declare dependencies on exposed services.

use crate::hive_config::{ExposeConfig, RuntimeContext, UsesConfig};
use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};

/// Compute HMAC-SHA256 of a secret using the expose name as key material
fn hmac_hash(secret: &str, expose_name: &str) -> Result<String> {
    // Note: Uses expose_name as the key and secret as data (for access control hashing)
    crate::crypto::hmac_sign(secret, expose_name)
}

fn hmac_verify(secret: &str, expose_name: &str, expected_hash: &str) -> Result<bool> {
    Ok(hmac_hash(secret, expose_name)? == expected_hash)
}

#[derive(Debug, Clone)]
pub struct ExposedService {
    pub name: String,
    pub source_name: String,
    pub service_name: String,
    /// Bcrypt hash of secret (if any)
    pub secret_hash: Option<String>,
    pub vars: HashMap<String, String>,
    pub ports: HashMap<String, u16>,
    pub healthy: bool,
}

pub struct ExposureManager {
    exposed: Arc<RwLock<HashMap<String, ExposedService>>>,
}

impl Default for ExposureManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ExposureManager {
    pub fn new() -> Self {
        Self {
            exposed: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn register_exposed(
        &self,
        source_name: &str,
        service_name: &str,
        config: &ExposeConfig,
        ports: &HashMap<String, u16>,
    ) -> Result<()> {
        // Hash the secret if provided (HMAC-SHA256 keyed on expose name)
        let secret_hash = config.secret.as_ref()
            .map(|secret| hmac_hash(secret, &config.name))
            .transpose()?;

        // Create runtime context for variable interpolation
        let mut runtime_ctx = RuntimeContext::new();
        runtime_ctx.set_ports(ports.clone());

        // Resolve variables with runtime port interpolation
        let mut resolved_vars = HashMap::new();
        for (key, value) in &config.vars {
            let resolved = runtime_ctx.interpolate(value)?;
            resolved_vars.insert(key.clone(), resolved);
        }

        let exposed = ExposedService {
            name: config.name.clone(),
            source_name: source_name.to_string(),
            service_name: service_name.to_string(),
            secret_hash,
            vars: resolved_vars,
            ports: ports.clone(),
            healthy: false,
        };

        let mut services = self.exposed.write().await;
        
        // Check for conflicts
        if let Some(existing) = services.get(&config.name) {
            if existing.source_name != source_name || existing.service_name != service_name {
                return Err(anyhow!(
                    "Expose name '{}' is already used by {}:{}",
                    config.name,
                    existing.source_name,
                    existing.service_name
                ));
            }
        }

        info!(
            "Registered exposed service: {} ({}:{})",
            config.name, source_name, service_name
        );
        services.insert(config.name.clone(), exposed);

        Ok(())
    }

    pub async fn unregister_exposed(&self, expose_name: &str) {
        let mut services = self.exposed.write().await;
        if services.remove(expose_name).is_some() {
            info!("Unregistered exposed service: {}", expose_name);
        }
    }

    pub async fn update_health(&self, expose_name: &str, healthy: bool) {
        let mut services = self.exposed.write().await;
        if let Some(service) = services.get_mut(expose_name) {
            service.healthy = healthy;
            debug!("Updated health for {}: {}", expose_name, healthy);
        }
    }

    pub async fn verify_secret(&self, expose_name: &str, secret: Option<&str>) -> Result<bool> {
        let services = self.exposed.read().await;

        let exposed = services.get(expose_name)
            .ok_or_else(|| anyhow!("Unknown exposed service: {}", expose_name))?;

        match (&exposed.secret_hash, secret) {
            (None, _) => Ok(true), // No secret required
            (Some(_), None) => Err(anyhow!(
                "Exposed service '{}' requires a secret",
                expose_name
            )),
            (Some(hash), Some(s)) => {
                if hmac_verify(s, expose_name, hash)? {
                    Ok(true)
                } else {
                    Err(anyhow!(
                        "Invalid secret for exposed service '{}'",
                        expose_name
                    ))
                }
            }
        }
    }

    pub async fn resolve_uses(
        &self,
        uses_configs: &[UsesConfig],
    ) -> Result<(HashMap<String, String>, HashMap<String, HashMap<String, u16>>)> {
        let services = self.exposed.read().await;
        
        let mut env_vars = HashMap::new();
        let mut uses_ports = HashMap::new();

        for uses in uses_configs {
            let exposed = services.get(&uses.name)
                .ok_or_else(|| anyhow!("Unknown exposed service: {}", uses.name))?;

            // Verify secret
            if let Some(secret_hash) = &exposed.secret_hash {
                let secret = uses.secret.as_ref()
                    .ok_or_else(|| anyhow!(
                        "Exposed service '{}' requires a secret",
                        uses.name
                    ))?;

                if !hmac_verify(secret, &uses.name, secret_hash)? {
                    return Err(anyhow!(
                        "Invalid secret for exposed service '{}'",
                        uses.name
                    ));
                }
            }

            // Check if exposed service is healthy
            if !exposed.healthy {
                return Err(anyhow!(
                    "Exposed service '{}' is not healthy",
                    uses.name
                ));
            }

            // Apply variable remapping
            for (orig_key, value) in &exposed.vars {
                let key = uses.vars.get(orig_key)
                    .cloned()
                    .unwrap_or_else(|| orig_key.clone());
                env_vars.insert(key, value.clone());
            }

            // Store ports for {{uses.alias.port.name}} interpolation
            let alias = uses.alias.as_ref()
                .unwrap_or(&uses.name);
            uses_ports.insert(alias.clone(), exposed.ports.clone());
        }

        Ok((env_vars, uses_ports))
    }

    pub async fn wait_for_exposed(&self, expose_name: &str, timeout_secs: u64) -> Result<()> {
        let start = std::time::Instant::now();
        let timeout = std::time::Duration::from_secs(timeout_secs);

        loop {
            {
                let services = self.exposed.read().await;
                if let Some(exposed) = services.get(expose_name) {
                    if exposed.healthy {
                        return Ok(());
                    }
                } else {
                    return Err(anyhow!("Unknown exposed service: {}", expose_name));
                }
            }

            if start.elapsed() > timeout {
                return Err(anyhow!(
                    "Timeout waiting for exposed service '{}' to become healthy",
                    expose_name
                ));
            }

            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }
    }

    pub async fn list_exposed(&self) -> Vec<ExposedService> {
        let services = self.exposed.read().await;
        services.values().cloned().collect()
    }

    pub async fn get_exposed(&self, name: &str) -> Option<ExposedService> {
        let services = self.exposed.read().await;
        services.get(name).cloned()
    }

    pub async fn get_consumers(&self, _expose_name: &str) -> Vec<String> {
        // This would require tracking which services use which exposed services
        // For now, return empty - can be implemented with additional tracking
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_register_exposed() {
        let manager = ExposureManager::new();
        
        let config = ExposeConfig {
            name: "shared-db".to_string(),
            secret: None,
            vars: {
                let mut v = HashMap::new();
                v.insert("DATABASE_URL".to_string(), "postgres://localhost:{{runtime.port.db}}/mydb".to_string());
                v
            },
        };

        let mut ports = HashMap::new();
        ports.insert("db".to_string(), 5432);

        manager.register_exposed("default", "postgres", &config, &ports)
            .await
            .unwrap();

        let exposed = manager.get_exposed("shared-db").await.unwrap();
        assert_eq!(exposed.name, "shared-db");
        assert_eq!(exposed.vars.get("DATABASE_URL"), Some(&"postgres://localhost:5432/mydb".to_string()));
    }

    #[tokio::test]
    async fn test_secret_verification() {
        let manager = ExposureManager::new();
        
        let config = ExposeConfig {
            name: "protected-db".to_string(),
            secret: Some("my-secret-123".to_string()),
            vars: HashMap::new(),
        };

        manager.register_exposed("default", "postgres", &config, &HashMap::new())
            .await
            .unwrap();

        // Correct secret should work
        assert!(manager.verify_secret("protected-db", Some("my-secret-123")).await.unwrap());
        
        // Wrong secret should fail
        assert!(manager.verify_secret("protected-db", Some("wrong-secret")).await.is_err());
        
        // No secret should fail
        assert!(manager.verify_secret("protected-db", None).await.is_err());
    }

    #[tokio::test]
    async fn test_resolve_uses() {
        let manager = ExposureManager::new();
        
        // Register exposed service
        let expose_config = ExposeConfig {
            name: "shared-db".to_string(),
            secret: None,
            vars: {
                let mut v = HashMap::new();
                v.insert("DATABASE_URL".to_string(), "postgres://localhost:5432/mydb".to_string());
                v.insert("DB_HOST".to_string(), "localhost".to_string());
                v
            },
        };

        let mut ports = HashMap::new();
        ports.insert("db".to_string(), 5432);

        manager.register_exposed("default", "postgres", &expose_config, &ports)
            .await
            .unwrap();

        // Mark as healthy
        manager.update_health("shared-db", true).await;

        // Resolve uses
        let uses_config = vec![
            UsesConfig {
                name: "shared-db".to_string(),
                secret: None,
                alias: Some("pg".to_string()),
                vars: {
                    let mut v = HashMap::new();
                    v.insert("DATABASE_URL".to_string(), "MY_DB_URL".to_string());
                    v
                },
            }
        ];

        let (vars, uses_ports) = manager.resolve_uses(&uses_config).await.unwrap();

        // Check remapped variable
        assert_eq!(vars.get("MY_DB_URL"), Some(&"postgres://localhost:5432/mydb".to_string()));
        // Check non-remapped variable
        assert_eq!(vars.get("DB_HOST"), Some(&"localhost".to_string()));
        // Check ports are available under alias
        assert_eq!(uses_ports.get("pg").unwrap().get("db"), Some(&5432));
    }
}
