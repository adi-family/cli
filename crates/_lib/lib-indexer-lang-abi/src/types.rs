//! FFI-safe types for language analyzer plugins.

use abi_stable::{
    std_types::{ROption, RString, RVec},
    StableAbi,
};
use serde::{Deserialize, Serialize};

/// FFI-safe source location.
#[repr(C)]
#[derive(StableAbi, Clone, Debug, Serialize, Deserialize)]
pub struct LocationAbi {
    pub start_line: u32,
    pub start_col: u32,
    pub end_line: u32,
    pub end_col: u32,
    pub start_byte: u32,
    pub end_byte: u32,
}

impl LocationAbi {
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
}

/// FFI-safe symbol kind.
#[repr(u8)]
#[derive(StableAbi, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SymbolKindAbi {
    Function = 0,
    Method = 1,
    Class = 2,
    Struct = 3,
    Enum = 4,
    Interface = 5,
    Trait = 6,
    Module = 7,
    Constant = 8,
    Variable = 9,
    Type = 10,
    Property = 11,
    Field = 12,
    Constructor = 13,
    Destructor = 14,
    Operator = 15,
    Macro = 16,
    Namespace = 17,
    Package = 18,
    Unknown = 255,
}

impl SymbolKindAbi {
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

/// FFI-safe visibility modifier.
#[repr(u8)]
#[derive(StableAbi, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum VisibilityAbi {
    Public = 0,
    PublicCrate = 1,
    PublicSuper = 2,
    Protected = 3,
    Private = 4,
    Internal = 5,
    Unknown = 255,
}

impl VisibilityAbi {
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

    pub fn parse(s: &str) -> Self {
        match s {
            "public" => Self::Public,
            "public_crate" => Self::PublicCrate,
            "public_super" => Self::PublicSuper,
            "protected" => Self::Protected,
            "private" => Self::Private,
            "internal" => Self::Internal,
            _ => Self::Unknown,
        }
    }
}

/// FFI-safe reference kind.
#[repr(u8)]
#[derive(StableAbi, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReferenceKindAbi {
    Call = 0,
    TypeReference = 1,
    FieldAccess = 2,
    Import = 3,
    Inheritance = 4,
    MacroInvocation = 5,
    VariableReference = 6,
}

impl ReferenceKindAbi {
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

    pub fn parse(s: &str) -> Self {
        match s {
            "call" => Self::Call,
            "type" => Self::TypeReference,
            "field" => Self::FieldAccess,
            "import" => Self::Import,
            "inheritance" => Self::Inheritance,
            "macro" => Self::MacroInvocation,
            "variable" => Self::VariableReference,
            _ => Self::Call,
        }
    }
}

/// FFI-safe parsed symbol.
#[repr(C)]
#[derive(StableAbi, Clone, Debug, Serialize, Deserialize)]
pub struct ParsedSymbolAbi {
    pub name: RString,
    pub kind: SymbolKindAbi,
    pub location: LocationAbi,
    pub signature: ROption<RString>,
    pub doc_comment: ROption<RString>,
    pub visibility: VisibilityAbi,
    pub children: RVec<ParsedSymbolAbi>,
}

impl ParsedSymbolAbi {
    pub fn new(name: impl Into<RString>, kind: SymbolKindAbi, location: LocationAbi) -> Self {
        Self {
            name: name.into(),
            kind,
            location,
            signature: ROption::RNone,
            doc_comment: ROption::RNone,
            visibility: VisibilityAbi::Unknown,
            children: RVec::new(),
        }
    }

    pub fn with_signature(mut self, signature: impl Into<RString>) -> Self {
        self.signature = ROption::RSome(signature.into());
        self
    }

    pub fn with_doc_comment(mut self, doc: impl Into<RString>) -> Self {
        self.doc_comment = ROption::RSome(doc.into());
        self
    }

    pub fn with_visibility(mut self, visibility: VisibilityAbi) -> Self {
        self.visibility = visibility;
        self
    }

    pub fn with_children(mut self, children: impl Into<RVec<ParsedSymbolAbi>>) -> Self {
        self.children = children.into();
        self
    }
}

/// FFI-safe parsed reference.
#[repr(C)]
#[derive(StableAbi, Clone, Debug, Serialize, Deserialize)]
pub struct ParsedReferenceAbi {
    pub name: RString,
    pub kind: ReferenceKindAbi,
    pub location: LocationAbi,
    pub containing_symbol_index: ROption<u32>,
}

impl ParsedReferenceAbi {
    pub fn new(name: impl Into<RString>, kind: ReferenceKindAbi, location: LocationAbi) -> Self {
        Self {
            name: name.into(),
            kind,
            location,
            containing_symbol_index: ROption::RNone,
        }
    }

    pub fn with_containing_symbol(mut self, index: u32) -> Self {
        self.containing_symbol_index = ROption::RSome(index);
        self
    }
}

/// FFI-safe language info returned by plugins.
#[repr(C)]
#[derive(StableAbi, Clone, Debug, Serialize, Deserialize)]
pub struct LanguageInfoAbi {
    /// Language identifier (e.g., "rust", "python")
    pub language: RString,
    /// File extensions (e.g., ["rs"], ["py", "pyi"])
    pub extensions: RVec<RString>,
    /// Plugin version
    pub version: RString,
    /// Human-readable language name
    pub display_name: RString,
}

impl LanguageInfoAbi {
    pub fn new(language: impl Into<RString>, version: impl Into<RString>) -> Self {
        Self {
            language: language.into(),
            extensions: RVec::new(),
            version: version.into(),
            display_name: RString::new(),
        }
    }

    pub fn with_extensions(
        mut self,
        extensions: impl IntoIterator<Item = impl Into<RString>>,
    ) -> Self {
        self.extensions = extensions.into_iter().map(Into::into).collect();
        self
    }

    pub fn with_display_name(mut self, name: impl Into<RString>) -> Self {
        self.display_name = name.into();
        self
    }
}

/// Request for extract_symbols and extract_references methods.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractRequest {
    pub source: String,
    pub tree_sexp: String,
}

/// Response for get_grammar_path method.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrammarPathResponse {
    pub path: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parsed_symbol_builder() {
        let loc = LocationAbi::new(1, 0, 5, 0, 0, 100);
        let symbol = ParsedSymbolAbi::new("my_function", SymbolKindAbi::Function, loc)
            .with_signature("fn my_function() -> i32")
            .with_visibility(VisibilityAbi::Public);

        assert_eq!(symbol.name.as_str(), "my_function");
        assert_eq!(symbol.kind, SymbolKindAbi::Function);
        assert_eq!(symbol.visibility, VisibilityAbi::Public);
    }

    #[test]
    fn test_language_info_builder() {
        let info = LanguageInfoAbi::new("rust", "0.1.0")
            .with_extensions(["rs"])
            .with_display_name("Rust");

        assert_eq!(info.language.as_str(), "rust");
        assert_eq!(info.extensions.len(), 1);
        assert_eq!(info.display_name.as_str(), "Rust");
    }

    #[test]
    fn test_serialization() {
        let loc = LocationAbi::new(1, 0, 5, 0, 0, 100);
        let symbol = ParsedSymbolAbi::new("test", SymbolKindAbi::Function, loc);

        let json = serde_json::to_string(&symbol).unwrap();
        let parsed: ParsedSymbolAbi = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.name.as_str(), "test");
    }
}
