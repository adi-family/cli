//! Dynamic tree-sitter grammar loading from language plugins.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use lib_plugin_host::PluginManagerV3;
use tree_sitter::Language as TsLanguage;

use crate::error::{Error, Result};
use crate::types::Language;

/// Registry for dynamically loading tree-sitter grammars from plugins.
pub struct GrammarRegistry {
    plugin_manager: Option<Arc<PluginManagerV3>>,
    loaded: RwLock<HashMap<Language, TsLanguage>>,
}

impl GrammarRegistry {
    /// Create a new grammar registry.
    pub fn new(plugin_manager: Option<Arc<PluginManagerV3>>) -> Self {
        Self {
            plugin_manager,
            loaded: RwLock::new(HashMap::new()),
        }
    }

    /// Load a grammar for a language.
    /// Returns cached grammar if already loaded.
    pub fn load(&self, lang: Language) -> Result<TsLanguage> {
        // Check cache first
        if let Ok(loaded) = self.loaded.read() {
            if let Some(grammar) = loaded.get(&lang) {
                return Ok(grammar.clone());
            }
        }

        // Try to load from plugin
        if let Some(ts_lang) = self.load_from_plugin(lang)? {
            return Ok(ts_lang);
        }

        Err(Error::UnsupportedLanguage(format!(
            "No grammar plugin found for {}",
            lang.as_str()
        )))
    }

    /// Check if a grammar is available for a language.
    pub fn supports(&self, lang: Language) -> bool {
        // Check cache
        if let Ok(loaded) = self.loaded.read() {
            if loaded.contains_key(&lang) {
                return true;
            }
        }

        // Check if plugin exists
        if let Some(ref manager) = self.plugin_manager {
            return manager.has_language_analyzer(lang.as_str());
        }

        false
    }

    /// Load grammar from a plugin.
    fn load_from_plugin(&self, lang: Language) -> Result<Option<TsLanguage>> {
        let manager = match &self.plugin_manager {
            Some(m) => m,
            None => return Ok(None),
        };

        let plugin = match manager.get_language_analyzer(lang.as_str()) {
            Some(p) => p,
            None => return Ok(None),
        };

        // Get tree-sitter language from plugin
        let ts_lang_ptr = plugin.tree_sitter_language();
        if ts_lang_ptr.is_null() {
            return Err(Error::Plugin(format!(
                "Plugin returned null tree-sitter language for {}",
                lang.as_str()
            )));
        }

        // SAFETY: The plugin is responsible for returning a valid tree-sitter Language pointer.
        // The pointer remains valid for the lifetime of the plugin.
        let ts_lang = unsafe { (*(ts_lang_ptr as *const TsLanguage)).clone() };

        // Cache the loaded grammar
        if let Ok(mut loaded) = self.loaded.write() {
            // Re-check in case another thread loaded it
            if !loaded.contains_key(&lang) {
                loaded.insert(lang, ts_lang.clone());
            }
        }

        Ok(Some(ts_lang))
    }

    /// Get the number of loaded grammars.
    pub fn loaded_count(&self) -> usize {
        self.loaded.read().map(|l| l.len()).unwrap_or(0)
    }
}

impl Default for GrammarRegistry {
    fn default() -> Self {
        Self::new(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_plugin_manager() {
        let registry = GrammarRegistry::new(None);
        assert!(!registry.supports(Language::Rust));
        assert!(registry.load(Language::Rust).is_err());
    }
}
