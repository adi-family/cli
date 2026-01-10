//! Registry for looking up language analyzers from plugins.

use std::sync::Arc;

use lib_plugin_host::ServiceRegistry;

use super::plugin_adapter::PluginAnalyzerAdapter;
use super::treesitter::analyzers::{generic::GenericAnalyzer, LanguageAnalyzer};
use crate::types::Language;

/// Registry for language analyzers.
/// Looks up analyzers from plugin services, falling back to GenericAnalyzer.
pub struct AnalyzerRegistry {
    service_registry: Option<Arc<ServiceRegistry>>,
}

impl AnalyzerRegistry {
    /// Create a new analyzer registry.
    pub fn new(service_registry: Option<Arc<ServiceRegistry>>) -> Self {
        Self { service_registry }
    }

    /// Get an analyzer for a language.
    /// Tries plugin service first, falls back to GenericAnalyzer.
    pub fn get_analyzer(&self, lang: Language) -> Box<dyn LanguageAnalyzer> {
        // Try plugin service
        if let Some(analyzer) = self.try_plugin_analyzer(lang) {
            return analyzer;
        }

        // Fall back to generic analyzer
        Box::new(GenericAnalyzer::new(lang))
    }

    /// Try to get an analyzer from a plugin service.
    fn try_plugin_analyzer(&self, lang: Language) -> Option<Box<dyn LanguageAnalyzer>> {
        let registry = self.service_registry.as_ref()?;

        let service_id = lib_indexer_lang_abi::service_id(lang.as_str());
        let handle = registry.lookup(&service_id)?;

        Some(Box::new(PluginAnalyzerAdapter::new(handle, lang)))
    }

    /// Check if a plugin analyzer is available for a language.
    pub fn has_plugin_analyzer(&self, lang: Language) -> bool {
        if let Some(ref registry) = self.service_registry {
            let service_id = lib_indexer_lang_abi::service_id(lang.as_str());
            return registry.has_service(&service_id);
        }
        false
    }
}

impl Default for AnalyzerRegistry {
    fn default() -> Self {
        Self::new(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fallback_to_generic() {
        let registry = AnalyzerRegistry::new(None);

        // Should always return an analyzer (generic fallback)
        let analyzer = registry.get_analyzer(Language::Rust);

        // GenericAnalyzer should be returned
        assert!(!registry.has_plugin_analyzer(Language::Rust));

        // Should not panic
        drop(analyzer);
    }
}
