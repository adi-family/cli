use crate::discovery::{discover_translation_services, ServiceRegistry};
use crate::error::{I18nError, Result};
use crate::fallback::{get_attribute, get_message};
use fluent_bundle::{FluentBundle, FluentResource};
use std::collections::HashMap;
use std::sync::Arc;
use unic_langid::LanguageIdentifier;

/// Main i18n manager for translation lookups
pub struct I18n {
    /// FluentBundle instances per language
    bundles: HashMap<String, FluentBundle<FluentResource>>,
    /// Currently active language
    current_language: String,
    /// Fallback language (typically "en-US")
    fallback_language: String,
    /// Service registry for discovering translation plugins
    service_registry: Arc<dyn ServiceRegistry>,
    /// Namespace for translation keys (e.g., "cli", "tasks")
    namespace: Option<String>,
}

impl std::fmt::Debug for I18n {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("I18n")
            .field("current_language", &self.current_language)
            .field("fallback_language", &self.fallback_language)
            .field("namespace", &self.namespace)
            .field(
                "bundles",
                &format!("<{} languages>", self.available_languages().len()),
            )
            .finish()
    }
}

// SAFETY: I18n is protected by a Mutex at the global level, so it's safe to send between threads.
// FluentBundle contains RefCell which is !Send, but the Mutex ensures exclusive access.
unsafe impl Send for I18n {}

impl I18n {
    /// Create a new I18n instance
    ///
    /// # Arguments
    /// * `service_registry` - Service registry from plugin host
    pub fn new(service_registry: Arc<dyn ServiceRegistry>) -> Self {
        Self {
            bundles: HashMap::new(),
            current_language: "en-US".to_string(),
            fallback_language: "en-US".to_string(),
            service_registry,
            namespace: None,
        }
    }

    /// Set the namespace for translation discovery
    ///
    /// This limits discovery to services matching `adi.i18n.{namespace}.*`
    pub fn with_namespace(mut self, namespace: impl Into<String>) -> Self {
        self.namespace = Some(namespace.into());
        self
    }

    /// Discover and load all available translation plugins
    ///
    /// Scans the service registry for translation services matching the namespace
    /// and loads their Fluent messages into bundles.
    pub fn discover_translations(&mut self) -> Result<()> {
        let namespace = self.namespace.as_deref().unwrap_or("cli");

        let services = discover_translation_services(&self.service_registry, namespace)?;

        tracing::info!("Discovered {} translation services", services.len());

        for service in services {
            match self.load_translation_service(&service.service_id, &service.language) {
                Ok(_) => {
                    tracing::debug!(
                        "Loaded translation: {} ({})",
                        service.language_name,
                        service.language
                    );
                }
                Err(e) => {
                    tracing::warn!("Failed to load translation for {}: {}", service.language, e);
                }
            }
        }

        Ok(())
    }

    /// Load a specific translation service by ID
    fn load_translation_service(&mut self, service_id: &str, language: &str) -> Result<()> {
        // Lookup service
        let service = self
            .service_registry
            .lookup_service(service_id)
            .map_err(|e: I18nError| I18nError::ServiceRegistryError(e.to_string()))?;

        // Invoke get_messages() to get .ftl content
        let ftl_content = service
            .invoke("get_messages", "{}")
            .map_err(|e: I18nError| I18nError::ServiceInvokeError(e.to_string()))?;

        // Parse Fluent resource
        let resource = FluentResource::try_new(ftl_content).map_err(|e| {
            I18nError::FluentParseError(format!("Failed to parse .ftl for {}: {:?}", language, e))
        })?;

        // Create language identifier
        let lang_id: LanguageIdentifier = language
            .parse()
            .map_err(|_| I18nError::InvalidLanguageCode(language.to_string()))?;

        // Create FluentBundle
        let mut bundle = FluentBundle::new(vec![lang_id]);
        bundle
            .add_resource(resource)
            .map_err(|e| I18nError::FluentParseError(format!("Failed to add resource: {:?}", e)))?;

        // Store bundle
        self.bundles.insert(language.to_string(), bundle);

        Ok(())
    }

    /// Load embedded FTL content directly without a plugin
    ///
    /// This is useful for embedding fallback translations in the binary.
    ///
    /// # Arguments
    /// * `language` - Language code (e.g., "en-US")
    /// * `ftl_content` - Raw Fluent (.ftl) content
    pub fn load_embedded(&mut self, language: &str, ftl_content: &str) -> Result<()> {
        // Parse Fluent resource
        let resource = FluentResource::try_new(ftl_content.to_string()).map_err(|e| {
            I18nError::FluentParseError(format!(
                "Failed to parse embedded .ftl for {}: {:?}",
                language, e
            ))
        })?;

        // Create language identifier
        let lang_id: LanguageIdentifier = language
            .parse()
            .map_err(|_| I18nError::InvalidLanguageCode(language.to_string()))?;

        // Create FluentBundle
        let mut bundle = FluentBundle::new(vec![lang_id]);
        bundle
            .add_resource(resource)
            .map_err(|e| I18nError::FluentParseError(format!("Failed to add resource: {:?}", e)))?;

        // Store bundle
        self.bundles.insert(language.to_string(), bundle);

        Ok(())
    }

    /// Set the active language
    ///
    /// The language must be loaded via `discover_translations()` first.
    pub fn set_language(&mut self, language: &str) -> Result<()> {
        // Check if the language is available
        if !self.bundles.contains_key(language) && language != self.fallback_language {
            return Err(I18nError::LanguageNotFound(language.to_string()));
        }

        self.current_language = language.to_string();
        Ok(())
    }

    /// Get current language code
    pub fn current_language(&self) -> &str {
        &self.current_language
    }

    /// Get a translated message by key
    ///
    /// Implements fallback chain: current_lang → fallback_lang → key
    pub fn get(&self, key: &str) -> String {
        // Check if key contains attribute (e.g., "key.prefix")
        if let Some(dot_pos) = key.rfind('.') {
            let base_key = &key[..dot_pos];
            let attr = &key[dot_pos + 1..];

            if let Some(value) = get_attribute(
                &self.bundles,
                &self.current_language,
                &self.fallback_language,
                base_key,
                attr,
            ) {
                return value;
            }
        }

        // Regular message lookup
        get_message(
            &self.bundles,
            &self.current_language,
            &self.fallback_language,
            key,
            None,
        )
    }

    /// Get a translated message with arguments
    ///
    /// # Arguments
    /// * `key` - Translation key (e.g., "hello")
    /// * `args` - HashMap of arguments for Fluent placeholders
    pub fn get_with_args(
        &self,
        key: &str,
        args: HashMap<String, fluent_bundle::FluentValue<'static>>,
    ) -> String {
        get_message(
            &self.bundles,
            &self.current_language,
            &self.fallback_language,
            key,
            Some(&args),
        )
    }

    /// Get list of available languages
    pub fn available_languages(&self) -> Vec<String> {
        self.bundles.keys().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::discovery::{ServiceDescriptor, ServiceHandle};

    struct MockServiceRegistry {
        services: Vec<String>,
        messages: HashMap<String, String>,
        metadata: HashMap<String, String>,
    }

    impl ServiceRegistry for MockServiceRegistry {
        fn list_services(&self) -> Result<Vec<ServiceDescriptor>> {
            Ok(self
                .services
                .iter()
                .map(|id| ServiceDescriptor::new(id.clone()))
                .collect())
        }

        fn lookup_service(&self, service_id: &str) -> Result<Box<dyn ServiceHandle>> {
            if self.services.contains(&service_id.to_string()) {
                Ok(Box::new(MockServiceHandle {
                    service_id: service_id.to_string(),
                    messages: self.messages.clone(),
                    metadata: self.metadata.clone(),
                }))
            } else {
                Err(I18nError::ServiceRegistryError(format!(
                    "Service not found: {}",
                    service_id
                )))
            }
        }
    }

    struct MockServiceHandle {
        service_id: String,
        messages: HashMap<String, String>,
        metadata: HashMap<String, String>,
    }

    impl ServiceHandle for MockServiceHandle {
        fn invoke(&self, method: &str, _args: &str) -> Result<String> {
            match method {
                "get_messages" => self
                    .messages
                    .get(&self.service_id)
                    .cloned()
                    .ok_or_else(|| I18nError::ServiceInvokeError("No messages".to_string())),
                "get_metadata" => self
                    .metadata
                    .get(&self.service_id)
                    .cloned()
                    .ok_or_else(|| I18nError::ServiceInvokeError("No metadata".to_string())),
                _ => Err(I18nError::ServiceInvokeError(format!(
                    "Unknown method: {}",
                    method
                ))),
            }
        }
    }

    #[test]
    fn test_i18n_basic() {
        let mut messages = HashMap::new();
        messages.insert(
            "adi.i18n.cli.en-US".to_string(),
            "hello = Hello, { $name }!".to_string(),
        );

        let mut metadata = HashMap::new();
        metadata.insert(
            "adi.i18n.cli.en-US".to_string(),
            r#"{"plugin_id":"adi.cli","language":"en-US","language_name":"English","namespace":"cli","version":"1.0.0"}"#.to_string(),
        );

        let registry = Arc::new(MockServiceRegistry {
            services: vec!["adi.i18n.cli.en-US".to_string()],
            messages,
            metadata,
        });

        let mut i18n = I18n::new(registry);
        i18n.discover_translations().unwrap();
        i18n.set_language("en-US").unwrap();

        let mut args = HashMap::new();
        args.insert(
            "name".to_string(),
            fluent_bundle::FluentValue::from("World"),
        );
        let result = i18n.get_with_args("hello", args);
        assert!(result.contains("Hello"));
        assert!(result.contains("World"));
    }
}
