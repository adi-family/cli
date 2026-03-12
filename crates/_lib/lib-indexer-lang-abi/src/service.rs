//! Service constants and helpers for language analyzer plugins.

/// Service ID prefix for language analyzers.
/// Full service ID format: `adi.indexer.lang.<language>`
pub const SERVICE_PREFIX: &str = "adi.indexer.lang.";

/// Method name: Get path to tree-sitter grammar shared library.
/// Args: `{}`
/// Returns: `{"path": "grammar/rust.so"}`
pub const METHOD_GET_GRAMMAR_PATH: &str = "get_grammar_path";

/// Method name: Extract symbols from source code.
/// Args: `{"source": "...", "tree_sexp": "..."}`
/// Returns: `[ParsedSymbolAbi, ...]`
pub const METHOD_EXTRACT_SYMBOLS: &str = "extract_symbols";

/// Method name: Extract references from source code.
/// Args: `{"source": "...", "tree_sexp": "..."}`
/// Returns: `[ParsedReferenceAbi, ...]`
pub const METHOD_EXTRACT_REFERENCES: &str = "extract_references";

/// Method name: Get language plugin info.
/// Args: `{}`
/// Returns: `LanguageInfoAbi`
pub const METHOD_GET_INFO: &str = "get_info";

/// Build a service ID for a language.
pub fn service_id(language: &str) -> String {
    format!("{}{}", SERVICE_PREFIX, language)
}

/// Extract language name from service ID.
pub fn language_from_service_id(service_id: &str) -> Option<&str> {
    service_id.strip_prefix(SERVICE_PREFIX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_id() {
        assert_eq!(service_id("rust"), "adi.indexer.lang.rust");
        assert_eq!(service_id("python"), "adi.indexer.lang.python");
    }

    #[test]
    fn test_language_from_service_id() {
        assert_eq!(
            language_from_service_id("adi.indexer.lang.rust"),
            Some("rust")
        );
        assert_eq!(
            language_from_service_id("adi.indexer.lang.python"),
            Some("python")
        );
        assert_eq!(language_from_service_id("adi.tasks.cli"), None);
    }
}
