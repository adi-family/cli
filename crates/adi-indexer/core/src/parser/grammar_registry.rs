//! Dynamic tree-sitter grammar loading from language plugins.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use lib_indexer_lang_abi::{GrammarPathResponse, METHOD_GET_GRAMMAR_PATH};
use lib_plugin_host::ServiceRegistry;
use libloading::Library;
use tree_sitter::Language as TsLanguage;

use crate::error::{Error, Result};
use crate::types::Language;

/// Loaded grammar with its dynamic library handle.
struct LoadedGrammar {
    #[allow(dead_code)]
    library: Library, // Keep library loaded
    language: TsLanguage,
}

/// Registry for dynamically loading tree-sitter grammars from plugins.
pub struct GrammarRegistry {
    service_registry: Option<Arc<ServiceRegistry>>,
    loaded: RwLock<HashMap<Language, LoadedGrammar>>,
    plugin_dirs: Vec<PathBuf>,
}

impl GrammarRegistry {
    /// Create a new grammar registry.
    pub fn new(service_registry: Option<Arc<ServiceRegistry>>) -> Self {
        Self {
            service_registry,
            loaded: RwLock::new(HashMap::new()),
            plugin_dirs: Vec::new(),
        }
    }

    /// Add a directory to search for plugin grammars.
    pub fn add_plugin_dir(&mut self, path: PathBuf) {
        self.plugin_dirs.push(path);
    }

    /// Load a grammar for a language.
    /// Returns cached grammar if already loaded.
    pub fn load(&self, lang: Language) -> Result<TsLanguage> {
        // Check cache first
        if let Ok(loaded) = self.loaded.read() {
            if let Some(grammar) = loaded.get(&lang) {
                return Ok(grammar.language.clone());
            }
        }

        // Try to load from plugin service
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

        // Check if plugin service exists
        if let Some(ref registry) = self.service_registry {
            let service_id = lib_indexer_lang_abi::service_id(lang.as_str());
            return registry.has_service(&service_id);
        }

        false
    }

    /// Load grammar from a plugin service.
    fn load_from_plugin(&self, lang: Language) -> Result<Option<TsLanguage>> {
        let registry = match &self.service_registry {
            Some(r) => r,
            None => return Ok(None),
        };

        let service_id = lib_indexer_lang_abi::service_id(lang.as_str());

        let handle = match registry.lookup(&service_id) {
            Some(h) => h,
            None => return Ok(None),
        };

        // Get grammar path from plugin
        let response = unsafe {
            handle
                .invoke(METHOD_GET_GRAMMAR_PATH, "{}")
                .map_err(|e| Error::Plugin(e.message.to_string()))?
        };

        let grammar_response: GrammarPathResponse =
            serde_json::from_str(&response).map_err(|e| Error::Parser(e.to_string()))?;

        // Resolve path relative to plugin directory
        let grammar_path = self.resolve_grammar_path(&grammar_response.path)?;

        // Load the dynamic library
        let ts_lang = self.load_grammar_library(&grammar_path, lang)?;

        // Cache the loaded grammar
        if let Ok(mut loaded) = self.loaded.write() {
            // Re-check in case another thread loaded it
            if !loaded.contains_key(&lang) {
                let library = unsafe {
                    Library::new(&grammar_path)
                        .map_err(|e| Error::Plugin(format!("Failed to load grammar: {}", e)))?
                };
                loaded.insert(
                    lang,
                    LoadedGrammar {
                        library,
                        language: ts_lang.clone(),
                    },
                );
            }
        }

        Ok(Some(ts_lang))
    }

    /// Resolve a grammar path, checking plugin directories.
    fn resolve_grammar_path(&self, path: &str) -> Result<PathBuf> {
        let path = PathBuf::from(path);

        // If absolute, use as-is
        if path.is_absolute() && path.exists() {
            return Ok(path);
        }

        // Search in plugin directories
        for dir in &self.plugin_dirs {
            let full_path = dir.join(&path);
            if full_path.exists() {
                return Ok(full_path);
            }
        }

        Err(Error::Plugin(format!(
            "Grammar not found: {}",
            path.display()
        )))
    }

    /// Load a tree-sitter grammar from a shared library.
    fn load_grammar_library(&self, path: &PathBuf, lang: Language) -> Result<TsLanguage> {
        let library = unsafe {
            Library::new(path)
                .map_err(|e| Error::Plugin(format!("Failed to load grammar library: {}", e)))?
        };

        // Tree-sitter grammars export a function named tree_sitter_<language>
        let symbol_name = format!("tree_sitter_{}", lang.as_str());

        let func: libloading::Symbol<unsafe extern "C" fn() -> TsLanguage> = unsafe {
            library.get(symbol_name.as_bytes()).map_err(|e| {
                Error::Plugin(format!("Failed to find symbol '{}': {}", symbol_name, e))
            })?
        };

        let ts_lang = unsafe { func() };

        Ok(ts_lang)
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
    fn test_no_service_registry() {
        let registry = GrammarRegistry::new(None);
        assert!(!registry.supports(Language::Rust));
        assert!(registry.load(Language::Rust).is_err());
    }
}
