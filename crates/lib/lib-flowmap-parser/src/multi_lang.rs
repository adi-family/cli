use lib_flowmap_core::FlowMapOutput;
use std::path::Path;

use crate::{BlockExtractor, JavaBlockExtractor, PythonBlockExtractor, RustBlockExtractor, Result};

/// Supported programming languages
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    TypeScript,
    JavaScript,
    TSX,
    JSX,
    Python,
    Java,
    Rust,
}

impl Language {
    /// Detect language from file extension
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_lowercase().as_str() {
            "ts" => Some(Self::TypeScript),
            "js" | "mjs" | "cjs" => Some(Self::JavaScript),
            "tsx" => Some(Self::TSX),
            "jsx" => Some(Self::JSX),
            "py" | "pyw" => Some(Self::Python),
            "java" => Some(Self::Java),
            "rs" => Some(Self::Rust),
            _ => None,
        }
    }

    /// Detect language from file path
    pub fn from_path(path: &str) -> Option<Self> {
        Path::new(path)
            .extension()
            .and_then(|ext| ext.to_str())
            .and_then(Self::from_extension)
    }

    /// Get the language name
    pub fn name(&self) -> &'static str {
        match self {
            Self::TypeScript => "typescript",
            Self::JavaScript => "javascript",
            Self::TSX => "tsx",
            Self::JSX => "jsx",
            Self::Python => "python",
            Self::Java => "java",
            Self::Rust => "rust",
        }
    }
}

/// Multi-language code parser that automatically selects the appropriate extractor
pub struct MultiLangParser {
    ts_extractor: Option<BlockExtractor>,
    py_extractor: Option<PythonBlockExtractor>,
    java_extractor: Option<JavaBlockExtractor>,
    rust_extractor: Option<RustBlockExtractor>,
}

impl MultiLangParser {
    /// Create a new multi-language parser with all extractors initialized
    pub fn new() -> Result<Self> {
        Ok(Self {
            ts_extractor: Some(BlockExtractor::new()?),
            py_extractor: Some(PythonBlockExtractor::new()?),
            java_extractor: Some(JavaBlockExtractor::new()?),
            rust_extractor: Some(RustBlockExtractor::new()?),
        })
    }

    /// Create a parser with only specific languages enabled
    pub fn with_languages(languages: &[Language]) -> Result<Self> {
        let mut parser = Self {
            ts_extractor: None,
            py_extractor: None,
            java_extractor: None,
            rust_extractor: None,
        };

        for lang in languages {
            match lang {
                Language::TypeScript | Language::JavaScript | Language::TSX | Language::JSX => {
                    if parser.ts_extractor.is_none() {
                        parser.ts_extractor = Some(BlockExtractor::new()?);
                    }
                }
                Language::Python => {
                    if parser.py_extractor.is_none() {
                        parser.py_extractor = Some(PythonBlockExtractor::new()?);
                    }
                }
                Language::Java => {
                    if parser.java_extractor.is_none() {
                        parser.java_extractor = Some(JavaBlockExtractor::new()?);
                    }
                }
                Language::Rust => {
                    if parser.rust_extractor.is_none() {
                        parser.rust_extractor = Some(RustBlockExtractor::new()?);
                    }
                }
            }
        }

        Ok(parser)
    }

    /// Parse source code with automatic language detection from file path
    pub fn parse(&mut self, source: &str, file_path: &str) -> Result<FlowMapOutput> {
        let language = Language::from_path(file_path)
            .ok_or_else(|| crate::ParseError::UnsupportedLanguage {
                path: file_path.to_string(),
            })?;

        self.parse_with_language(source, file_path, language)
    }

    /// Parse source code with explicit language specification
    pub fn parse_with_language(
        &mut self,
        source: &str,
        file_path: &str,
        language: Language,
    ) -> Result<FlowMapOutput> {
        match language {
            Language::TypeScript | Language::JavaScript | Language::TSX | Language::JSX => {
                let extractor = self.ts_extractor.as_mut()
                    .ok_or_else(|| crate::ParseError::UnsupportedLanguage {
                        path: format!("{} (extractor not initialized)", language.name()),
                    })?;
                extractor.parse_file(source, file_path)
            }
            Language::Python => {
                let extractor = self.py_extractor.as_mut()
                    .ok_or_else(|| crate::ParseError::UnsupportedLanguage {
                        path: format!("{} (extractor not initialized)", language.name()),
                    })?;
                extractor.parse_file(source, file_path)
            }
            Language::Java => {
                let extractor = self.java_extractor.as_mut()
                    .ok_or_else(|| crate::ParseError::UnsupportedLanguage {
                        path: format!("{} (extractor not initialized)", language.name()),
                    })?;
                extractor.parse_file(source, file_path)
            }
            Language::Rust => {
                let extractor = self.rust_extractor.as_mut()
                    .ok_or_else(|| crate::ParseError::UnsupportedLanguage {
                        path: format!("{} (extractor not initialized)", language.name()),
                    })?;
                extractor.parse_file(source, file_path)
            }
        }
    }

    /// Check if a file extension is supported
    pub fn is_supported(file_path: &str) -> bool {
        Language::from_path(file_path).is_some()
    }

    /// Get list of supported file extensions
    pub fn supported_extensions() -> &'static [&'static str] {
        &["ts", "tsx", "js", "jsx", "mjs", "cjs", "py", "pyw", "java", "rs"]
    }
}

impl Default for MultiLangParser {
    fn default() -> Self {
        Self::new().expect("Failed to create MultiLangParser")
    }
}

/// Parse a single file (convenience function)
pub fn parse_file(source: &str, file_path: &str) -> Result<FlowMapOutput> {
    let mut parser = MultiLangParser::new()?;
    parser.parse(source, file_path)
}

/// Parse multiple files and merge results
pub fn parse_files(files: &[(String, String)]) -> Result<FlowMapOutput> {
    let mut parser = MultiLangParser::new()?;
    let mut output = FlowMapOutput::new();

    for (path, source) in files {
        if MultiLangParser::is_supported(path) {
            let file_output = parser.parse(source, path)?;
            output.merge(file_output);
        }
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_language_detection() {
        assert_eq!(Language::from_path("test.ts"), Some(Language::TypeScript));
        assert_eq!(Language::from_path("test.tsx"), Some(Language::TSX));
        assert_eq!(Language::from_path("test.js"), Some(Language::JavaScript));
        assert_eq!(Language::from_path("test.py"), Some(Language::Python));
        assert_eq!(Language::from_path("Test.java"), Some(Language::Java));
        assert_eq!(Language::from_path("test.rs"), Some(Language::Rust));
        assert_eq!(Language::from_path("test.go"), None);
    }

    #[test]
    fn test_multi_lang_parser() {
        let mut parser = MultiLangParser::new().unwrap();

        // TypeScript
        let ts_output = parser.parse(
            "function add(a: number, b: number): number { return a + b; }",
            "test.ts"
        ).unwrap();
        assert!(ts_output.block_count() > 0);

        // Python
        let py_output = parser.parse(
            "def add(a, b):\n    return a + b",
            "test.py"
        ).unwrap();
        assert!(py_output.block_count() > 0);

        // Java
        let java_output = parser.parse(
            "public class Test { public int add(int a, int b) { return a + b; } }",
            "Test.java"
        ).unwrap();
        assert!(java_output.block_count() > 0);

        // Rust
        let rust_output = parser.parse(
            "fn add(a: i32, b: i32) -> i32 { a + b }",
            "test.rs"
        ).unwrap();
        assert!(rust_output.block_count() > 0);
    }

    #[test]
    fn test_unsupported_language() {
        let mut parser = MultiLangParser::new().unwrap();
        let result = parser.parse("package main", "test.go");
        assert!(result.is_err());
    }
}
