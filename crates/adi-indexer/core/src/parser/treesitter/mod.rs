// Copyright (c) 2024-2025 Ihor
// SPDX-License-Identifier: BSL-1.1
// See LICENSE file for details

pub mod analyzers;

use std::sync::Arc;

use lib_plugin_host::ServiceRegistry;
use tree_sitter::Parser as TsParser;

use crate::error::{Error, Result};
use crate::parser::{AnalyzerRegistry, GrammarRegistry, Parser};
use crate::types::*;
use analyzers::{generic::GenericAnalyzer, LanguageAnalyzer};

/// Tree-sitter based parser with plugin support.
///
/// Requires language plugins to be installed for parsing.
/// Falls back to GenericAnalyzer for symbol extraction when no plugin analyzer is available.
pub struct TreeSitterParser {
    grammar_registry: GrammarRegistry,
    analyzer_registry: AnalyzerRegistry,
}

impl TreeSitterParser {
    /// Create a new parser with plugin support.
    /// Requires a service registry with language plugins registered.
    pub fn new(service_registry: Arc<ServiceRegistry>) -> Self {
        Self {
            grammar_registry: GrammarRegistry::new(Some(service_registry.clone())),
            analyzer_registry: AnalyzerRegistry::new(Some(service_registry)),
        }
    }

    /// Create a parser without plugin support (will fail to parse any language).
    /// Useful for testing or when plugins are not available.
    pub fn without_plugins() -> Self {
        Self {
            grammar_registry: GrammarRegistry::new(None),
            analyzer_registry: AnalyzerRegistry::new(None),
        }
    }

    /// Get an analyzer for the language.
    /// Returns plugin analyzer if available, otherwise GenericAnalyzer.
    fn get_analyzer(&self, language: Language) -> Box<dyn LanguageAnalyzer> {
        if self.analyzer_registry.has_plugin_analyzer(language) {
            self.analyzer_registry.get_analyzer(language)
        } else {
            Box::new(GenericAnalyzer::new(language))
        }
    }
}

impl Parser for TreeSitterParser {
    fn parse(&self, source: &str, language: Language) -> Result<ParsedFile> {
        // Load grammar from plugin
        let ts_lang = self.grammar_registry.load(language).map_err(|e| {
            Error::UnsupportedLanguage(format!(
                "{}: {} (install adi-lang-{} plugin)",
                language.as_str(),
                e,
                language.as_str()
            ))
        })?;

        let mut parser = TsParser::new();
        parser
            .set_language(&ts_lang)
            .map_err(|e| Error::Parser(format!("Failed to set language: {}", e)))?;

        let tree = parser
            .parse(source, None)
            .ok_or_else(|| Error::Parser("Failed to parse source".to_string()))?;

        let analyzer = self.get_analyzer(language);
        let symbols = analyzer.extract_symbols(source, &tree);
        let references = analyzer.extract_references(source, &tree);

        Ok(ParsedFile {
            language,
            symbols,
            references,
        })
    }

    fn supports(&self, language: Language) -> bool {
        self.grammar_registry.supports(language)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_without_plugins() {
        let parser = TreeSitterParser::without_plugins();

        // Without plugins, no language is supported
        assert!(!parser.supports(Language::Rust));
        assert!(!parser.supports(Language::Python));
        assert!(!parser.supports(Language::Unknown));
    }

    #[test]
    fn test_parse_fails_without_plugins() {
        let parser = TreeSitterParser::without_plugins();
        let source = "fn main() {}";

        let result = parser.parse(source, Language::Rust);
        assert!(result.is_err());
    }
}
