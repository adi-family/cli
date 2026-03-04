use crate::error::{I18nError, Result};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Translation service information discovered from plugin registry
#[derive(Debug, Clone)]
pub struct TranslationServiceInfo {
    /// Service ID (e.g., "adi.i18n.cli.en-US")
    pub service_id: String,
    /// Language code (e.g., "en-US")
    pub language: String,
    /// Language name (e.g., "English (United States)")
    pub language_name: String,
    /// Namespace for translation keys (e.g., "cli")
    pub namespace: String,
    /// Plugin ID that provides this translation (e.g., "adi.cli.en-US")
    pub plugin_id: String,
}

/// Metadata returned from translation service's get_metadata() method
#[derive(Debug, Serialize, Deserialize)]
struct TranslationMetadata {
    plugin_id: String,
    language: String,
    language_name: String,
    namespace: String,
    #[allow(dead_code)]
    version: String,
}

/// Discover translation services from the service registry
///
/// Scans for services matching pattern `adi.i18n.{namespace}.*`
/// and extracts their metadata.
pub fn discover_translation_services(
    service_registry: &Arc<dyn ServiceRegistry>,
    namespace: &str,
) -> Result<Vec<TranslationServiceInfo>> {
    let mut services = Vec::new();
    let prefix = format!("adi.i18n.{}.", namespace);

    // List all services from registry
    let all_services = service_registry
        .list_services()
        .map_err(|e| I18nError::ServiceRegistryError(e.to_string()))?;

    // Filter for translation services matching our namespace
    for service_descriptor in all_services {
        let service_id = service_descriptor.service_id();

        if service_id.starts_with(&prefix) {
            // Lookup the service to get its metadata
            match service_registry.lookup_service(&service_id) {
                Ok(service_handle) => {
                    // Invoke get_metadata() to extract translation info
                    match service_handle.invoke("get_metadata", "{}") {
                        Ok(json_result) => {
                            match serde_json::from_str::<TranslationMetadata>(&json_result) {
                                Ok(metadata) => {
                                    services.push(TranslationServiceInfo {
                                        service_id: service_id.clone(),
                                        language: metadata.language,
                                        language_name: metadata.language_name,
                                        namespace: metadata.namespace,
                                        plugin_id: metadata.plugin_id,
                                    });
                                }
                                Err(e) => {
                                    tracing::warn!(
                                        "Failed to parse metadata from service {}: {}",
                                        service_id,
                                        e
                                    );
                                }
                            }
                        }
                        Err(e) => {
                            tracing::warn!(
                                "Failed to get metadata from service {}: {}",
                                service_id,
                                e
                            );
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to lookup service {}: {}", service_id, e);
                }
            }
        }
    }

    Ok(services)
}

/// Trait for service registry to abstract away implementation details
pub trait ServiceRegistry: Send + Sync {
    /// List all registered services
    fn list_services(&self) -> Result<Vec<ServiceDescriptor>>;
    /// Lookup a service by ID
    fn lookup_service(&self, service_id: &str) -> Result<Box<dyn ServiceHandle>>;
}

/// Minimal service descriptor for discovery
pub struct ServiceDescriptor {
    service_id: String,
}

impl ServiceDescriptor {
    pub fn new(service_id: String) -> Self {
        Self { service_id }
    }

    pub fn service_id(&self) -> String {
        self.service_id.clone()
    }
}

/// Minimal service handle for invoking methods
pub trait ServiceHandle {
    /// Invoke a method on the service
    fn invoke(&self, method: &str, args: &str) -> Result<String>;
}
