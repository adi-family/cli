//! Registry for looking up language analyzers from plugins.

use std::sync::Arc;

use lib_plugin_host::PluginManagerV3;

use super::plugin_adapter::PluginAnalyzerAdapter;
use super::treesitter::analyzers::{generic::GenericAnalyzer, LanguageAnalyzer};
use crate::types::Language;

/// Registry for language analyzers.
/// Looks up analyzers from plugin manager, falling back to GenericAnalyzer.
pub struct AnalyzerRegistry {
    plugin_manager: Option<Arc<PluginManagerV3>>,
}

impl AnalyzerRegistry {
    /// Create a new analyzer registry.
    pub fn new(plugin_manager: Option<Arc<PluginManagerV3>>) -> Self {
        Self { plugin_manager }
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

    /// Try to get an analyzer from a plugin.
    fn try_plugin_analyzer(&self, lang: Language) -> Option<Box<dyn LanguageAnalyzer>> {
        let manager = self.plugin_manager.as_ref()?;
        let plugin = manager.get_language_analyzer(lang.as_str())?;
        Some(Box::new(PluginAnalyzerAdapter::new(plugin, lang)))
    }

    /// Check if a plugin analyzer is available for a language.
    pub fn has_plugin_analyzer(&self, lang: Language) -> bool {
        if let Some(ref manager) = self.plugin_manager {
            return manager.has_language_analyzer(lang.as_str());
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
