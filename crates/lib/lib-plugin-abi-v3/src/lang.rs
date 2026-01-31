//! Language analyzer plugin trait
//!
//! Language analyzers extract symbols and references from source code using
//! tree-sitter grammars. These are used by the indexer for code navigation
//! and understanding.

use crate::{Plugin, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Language analyzer plugin trait
///
/// Language analyzers provide code intelligence for a specific programming language.
/// They use tree-sitter grammars to parse source code and extract:
/// - Symbols (functions, classes, structs, etc.)
/// - References (calls, imports, type references, etc.)
#[async_trait]
pub trait LanguageAnalyzer: Plugin {
    /// Get language metadata
    fn language_info(&self) -> LanguageInfo;

    /// Extract symbols from source code
    ///
    /// # Arguments
    /// * `source` - The source code to analyze
    ///
    /// # Returns
    /// A list of parsed symbols (functions, classes, structs, etc.)
    async fn extract_symbols(&self, source: &str) -> Result<Vec<ParsedSymbol>>;

    /// Extract references from source code
    ///
    /// # Arguments
    /// * `source` - The source code to analyze
    ///
    /// # Returns
    /// A list of parsed references (calls, imports, type references, etc.)
    async fn extract_references(&self, source: &str) -> Result<Vec<ParsedReference>>;

    /// Get the tree-sitter Language object
    ///
    /// This returns a raw pointer to the tree-sitter Language. The caller is
    /// responsible for using it safely with tree-sitter's API.
    ///
    /// # Safety
    /// The returned pointer is valid for the lifetime of the plugin.
    fn tree_sitter_language(&self) -> *const ();
}

/// Language metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageInfo {
    /// Language identifier (e.g., "rust", "python", "typescript")
    pub language: String,

    /// Human-readable language name (e.g., "Rust", "Python", "TypeScript")
    pub display_name: String,

    /// File extensions (e.g., ["rs"], ["py", "pyi"], ["ts", "tsx"])
    pub extensions: Vec<String>,

    /// Plugin version
    pub version: String,
}

impl LanguageInfo {
    /// Create new language info
    pub fn new(language: impl Into<String>, display_name: impl Into<String>) -> Self {
        Self {
            language: language.into(),
            display_name: display_name.into(),
            extensions: vec![],
            version: String::new(),
        }
    }

    /// Add file extensions
    pub fn with_extensions(mut self, extensions: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.extensions = extensions.into_iter().map(Into::into).collect();
        self
    }

    /// Set version
    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.version = version.into();
        self
    }
}

/// Source code location
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Location {
    /// Start line (0-indexed)
    pub start_line: u32,

    /// Start column (0-indexed)
    pub start_col: u32,

    /// End line (0-indexed)
    pub end_line: u32,

    /// End column (0-indexed)
    pub end_col: u32,

    /// Start byte offset
    pub start_byte: u32,

    /// End byte offset
    pub end_byte: u32,
}

impl Location {
    /// Create a new location
    pub fn new(
        start_line: u32,
        start_col: u32,
        end_line: u32,
        end_col: u32,
        start_byte: u32,
        end_byte: u32,
    ) -> Self {
        Self {
            start_line,
            start_col,
            end_line,
            end_col,
            start_byte,
            end_byte,
        }
    }

    /// Create from tree-sitter Node
    pub fn from_range(start: (u32, u32), end: (u32, u32), start_byte: u32, end_byte: u32) -> Self {
        Self {
            start_line: start.0,
            start_col: start.1,
            end_line: end.0,
            end_col: end.1,
            start_byte,
            end_byte,
        }
    }
}

/// Symbol kind
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SymbolKind {
    Function,
    Method,
    Class,
    Struct,
    Enum,
    Interface,
    Trait,
    Module,
    Constant,
    Variable,
    Type,
    Property,
    Field,
    Constructor,
    Destructor,
    Operator,
    Macro,
    Namespace,
    Package,
    Unknown,
}

impl SymbolKind {
    /// Get string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Function => "function",
            Self::Method => "method",
            Self::Class => "class",
            Self::Struct => "struct",
            Self::Enum => "enum",
            Self::Interface => "interface",
            Self::Trait => "trait",
            Self::Module => "module",
            Self::Constant => "constant",
            Self::Variable => "variable",
            Self::Type => "type",
            Self::Property => "property",
            Self::Field => "field",
            Self::Constructor => "constructor",
            Self::Destructor => "destructor",
            Self::Operator => "operator",
            Self::Macro => "macro",
            Self::Namespace => "namespace",
            Self::Package => "package",
            Self::Unknown => "unknown",
        }
    }

    /// Parse from string
    pub fn parse(s: &str) -> Self {
        match s {
            "function" => Self::Function,
            "method" => Self::Method,
            "class" => Self::Class,
            "struct" => Self::Struct,
            "enum" => Self::Enum,
            "interface" => Self::Interface,
            "trait" => Self::Trait,
            "module" => Self::Module,
            "constant" => Self::Constant,
            "variable" => Self::Variable,
            "type" => Self::Type,
            "property" => Self::Property,
            "field" => Self::Field,
            "constructor" => Self::Constructor,
            "destructor" => Self::Destructor,
            "operator" => Self::Operator,
            "macro" => Self::Macro,
            "namespace" => Self::Namespace,
            "package" => Self::Package,
            _ => Self::Unknown,
        }
    }
}

/// Visibility modifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Visibility {
    Public,
    PublicCrate,
    PublicSuper,
    Protected,
    Private,
    Internal,
    Unknown,
}

impl Visibility {
    /// Get string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Public => "public",
            Self::PublicCrate => "public_crate",
            Self::PublicSuper => "public_super",
            Self::Protected => "protected",
            Self::Private => "private",
            Self::Internal => "internal",
            Self::Unknown => "unknown",
        }
    }

    /// Parse from string
    pub fn parse(s: &str) -> Self {
        match s {
            "public" | "pub" => Self::Public,
            "public_crate" | "pub(crate)" => Self::PublicCrate,
            "public_super" | "pub(super)" => Self::PublicSuper,
            "protected" => Self::Protected,
            "private" => Self::Private,
            "internal" => Self::Internal,
            _ => Self::Unknown,
        }
    }
}

/// Reference kind
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReferenceKind {
    /// Function/method call
    Call,
    /// Type reference
    TypeReference,
    /// Field access
    FieldAccess,
    /// Import/use statement
    Import,
    /// Class/trait inheritance
    Inheritance,
    /// Macro invocation
    MacroInvocation,
    /// Variable reference
    VariableReference,
}

impl ReferenceKind {
    /// Get string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Call => "call",
            Self::TypeReference => "type",
            Self::FieldAccess => "field",
            Self::Import => "import",
            Self::Inheritance => "inheritance",
            Self::MacroInvocation => "macro",
            Self::VariableReference => "variable",
        }
    }

    /// Parse from string
    pub fn parse(s: &str) -> Self {
        match s {
            "call" => Self::Call,
            "type" | "type_reference" => Self::TypeReference,
            "field" | "field_access" => Self::FieldAccess,
            "import" | "use" => Self::Import,
            "inheritance" | "extends" | "implements" => Self::Inheritance,
            "macro" | "macro_invocation" => Self::MacroInvocation,
            "variable" | "variable_reference" => Self::VariableReference,
            _ => Self::Call,
        }
    }
}

/// Parsed symbol from source code
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedSymbol {
    /// Symbol name
    pub name: String,

    /// Symbol kind
    pub kind: SymbolKind,

    /// Source location
    pub location: Location,

    /// Optional function/method signature
    pub signature: Option<String>,

    /// Optional documentation comment
    pub doc_comment: Option<String>,

    /// Visibility modifier
    pub visibility: Visibility,

    /// Nested symbols (e.g., methods inside a class)
    #[serde(default)]
    pub children: Vec<ParsedSymbol>,
}

impl ParsedSymbol {
    /// Create a new parsed symbol
    pub fn new(name: impl Into<String>, kind: SymbolKind, location: Location) -> Self {
        Self {
            name: name.into(),
            kind,
            location,
            signature: None,
            doc_comment: None,
            visibility: Visibility::Unknown,
            children: vec![],
        }
    }

    /// Add signature
    pub fn with_signature(mut self, signature: impl Into<String>) -> Self {
        self.signature = Some(signature.into());
        self
    }

    /// Add documentation comment
    pub fn with_doc_comment(mut self, doc: impl Into<String>) -> Self {
        self.doc_comment = Some(doc.into());
        self
    }

    /// Set visibility
    pub fn with_visibility(mut self, visibility: Visibility) -> Self {
        self.visibility = visibility;
        self
    }

    /// Add children
    pub fn with_children(mut self, children: Vec<ParsedSymbol>) -> Self {
        self.children = children;
        self
    }
}

/// Parsed reference from source code
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedReference {
    /// Referenced name
    pub name: String,

    /// Reference kind
    pub kind: ReferenceKind,

    /// Source location
    pub location: Location,

    /// Index of the containing symbol (if any)
    pub containing_symbol_index: Option<u32>,
}

impl ParsedReference {
    /// Create a new parsed reference
    pub fn new(name: impl Into<String>, kind: ReferenceKind, location: Location) -> Self {
        Self {
            name: name.into(),
            kind,
            location,
            containing_symbol_index: None,
        }
    }

    /// Set containing symbol index
    pub fn with_containing_symbol(mut self, index: u32) -> Self {
        self.containing_symbol_index = Some(index);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parsed_symbol_builder() {
        let loc = Location::new(1, 0, 5, 0, 0, 100);
        let symbol = ParsedSymbol::new("my_function", SymbolKind::Function, loc)
            .with_signature("fn my_function() -> i32")
            .with_visibility(Visibility::Public);

        assert_eq!(symbol.name, "my_function");
        assert_eq!(symbol.kind, SymbolKind::Function);
        assert_eq!(symbol.visibility, Visibility::Public);
    }

    #[test]
    fn test_language_info_builder() {
        let info = LanguageInfo::new("rust", "Rust")
            .with_extensions(["rs"])
            .with_version("0.1.0");

        assert_eq!(info.language, "rust");
        assert_eq!(info.display_name, "Rust");
        assert_eq!(info.extensions, vec!["rs"]);
    }

    #[test]
    fn test_symbol_kind_parsing() {
        assert_eq!(SymbolKind::parse("function"), SymbolKind::Function);
        assert_eq!(SymbolKind::parse("class"), SymbolKind::Class);
        assert_eq!(SymbolKind::parse("invalid"), SymbolKind::Unknown);
    }

    #[test]
    fn test_visibility_parsing() {
        assert_eq!(Visibility::parse("pub"), Visibility::Public);
        assert_eq!(Visibility::parse("pub(crate)"), Visibility::PublicCrate);
        assert_eq!(Visibility::parse("private"), Visibility::Private);
    }
}
