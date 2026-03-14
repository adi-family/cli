// Copyright (c) 2024-2025 Ihor
// SPDX-License-Identifier: BSL-1.1
// See LICENSE file for details

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SymbolId(pub i64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FileId(pub i64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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

    pub fn is_public(&self) -> bool {
        matches!(self, Self::Public)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Location {
    pub start_line: u32,
    pub start_col: u32,
    pub end_line: u32,
    pub end_col: u32,
    pub start_byte: u32,
    pub end_byte: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Symbol {
    pub id: SymbolId,
    pub name: String,
    pub kind: SymbolKind,
    pub file_id: FileId,
    pub file_path: PathBuf,
    pub location: Location,
    pub parent_id: Option<SymbolId>,
    pub signature: Option<String>,
    pub description: Option<String>,
    pub doc_comment: Option<String>,
    pub visibility: Visibility,
    pub is_entry_point: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct File {
    pub id: FileId,
    pub path: PathBuf,
    pub language: Language,
    pub hash: String,
    pub size: u64,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    pub file: File,
    pub symbols: Vec<Symbol>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub symbol: Symbol,
    pub score: f32,
    pub context: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tree {
    pub files: Vec<FileNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileNode {
    pub path: PathBuf,
    pub language: Language,
    pub symbols: Vec<SymbolNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolNode {
    pub id: SymbolId,
    pub name: String,
    pub kind: SymbolKind,
    pub children: Vec<SymbolNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Status {
    pub indexed_files: u64,
    pub indexed_symbols: u64,
    pub embedding_dimensions: u32,
    pub embedding_model: String,
    pub last_indexed: Option<String>,
    pub storage_size_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexProgress {
    pub files_processed: u64,
    pub files_total: u64,
    pub symbols_indexed: u64,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Language {
    Rust,
    Python,
    JavaScript,
    TypeScript,
    Java,
    Go,
    C,
    Cpp,
    CSharp,
    Ruby,
    Php,
    Kotlin,
    Scala,
    Swift,
    Bash,
    Lua,
    Sql,
    Json,
    Yaml,
    Toml,
    Xml,
    Html,
    Css,
    Markdown,
    Dockerfile,
    Hcl,
    GraphQL,
    Haskell,
    OCaml,
    Elixir,
    Erlang,
    Zig,
    Unknown,
}

impl Language {
    pub fn from_extension(ext: &str) -> Self {
        match ext.to_lowercase().as_str() {
            "rs" => Self::Rust,
            "py" | "pyi" | "pyw" => Self::Python,
            "js" | "mjs" | "cjs" => Self::JavaScript,
            "ts" | "mts" | "cts" => Self::TypeScript,
            "tsx" => Self::TypeScript,
            "jsx" => Self::JavaScript,
            "java" => Self::Java,
            "go" => Self::Go,
            "c" | "h" => Self::C,
            "cpp" | "cc" | "cxx" | "hpp" | "hxx" | "hh" => Self::Cpp,
            "cs" => Self::CSharp,
            "rb" | "rake" | "gemspec" => Self::Ruby,
            "php" => Self::Php,
            "kt" | "kts" => Self::Kotlin,
            "scala" | "sc" => Self::Scala,
            "swift" => Self::Swift,
            "sh" | "bash" | "zsh" => Self::Bash,
            "lua" => Self::Lua,
            "sql" => Self::Sql,
            "json" => Self::Json,
            "yaml" | "yml" => Self::Yaml,
            "toml" => Self::Toml,
            "xml" | "xsd" | "xsl" => Self::Xml,
            "html" | "htm" => Self::Html,
            "css" | "scss" | "sass" | "less" => Self::Css,
            "md" | "markdown" => Self::Markdown,
            "dockerfile" => Self::Dockerfile,
            "tf" | "hcl" => Self::Hcl,
            "graphql" | "gql" => Self::GraphQL,
            "hs" | "lhs" => Self::Haskell,
            "ml" | "mli" => Self::OCaml,
            "ex" | "exs" => Self::Elixir,
            "erl" | "hrl" => Self::Erlang,
            "zig" => Self::Zig,
            _ => Self::Unknown,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Rust => "rust",
            Self::Python => "python",
            Self::JavaScript => "javascript",
            Self::TypeScript => "typescript",
            Self::Java => "java",
            Self::Go => "go",
            Self::C => "c",
            Self::Cpp => "cpp",
            Self::CSharp => "csharp",
            Self::Ruby => "ruby",
            Self::Php => "php",
            Self::Kotlin => "kotlin",
            Self::Scala => "scala",
            Self::Swift => "swift",
            Self::Bash => "bash",
            Self::Lua => "lua",
            Self::Sql => "sql",
            Self::Json => "json",
            Self::Yaml => "yaml",
            Self::Toml => "toml",
            Self::Xml => "xml",
            Self::Html => "html",
            Self::Css => "css",
            Self::Markdown => "markdown",
            Self::Dockerfile => "dockerfile",
            Self::Hcl => "hcl",
            Self::GraphQL => "graphql",
            Self::Haskell => "haskell",
            Self::OCaml => "ocaml",
            Self::Elixir => "elixir",
            Self::Erlang => "erlang",
            Self::Zig => "zig",
            Self::Unknown => "unknown",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "rust" => Self::Rust,
            "python" => Self::Python,
            "javascript" => Self::JavaScript,
            "typescript" => Self::TypeScript,
            "java" => Self::Java,
            "go" => Self::Go,
            "c" => Self::C,
            "cpp" => Self::Cpp,
            "csharp" => Self::CSharp,
            "ruby" => Self::Ruby,
            "php" => Self::Php,
            "kotlin" => Self::Kotlin,
            "scala" => Self::Scala,
            "swift" => Self::Swift,
            "bash" => Self::Bash,
            "lua" => Self::Lua,
            "sql" => Self::Sql,
            "json" => Self::Json,
            "yaml" => Self::Yaml,
            "toml" => Self::Toml,
            "xml" => Self::Xml,
            "html" => Self::Html,
            "css" => Self::Css,
            "markdown" => Self::Markdown,
            "dockerfile" => Self::Dockerfile,
            "hcl" => Self::Hcl,
            "graphql" => Self::GraphQL,
            "haskell" => Self::Haskell,
            "ocaml" => Self::OCaml,
            "elixir" => Self::Elixir,
            "erlang" => Self::Erlang,
            "zig" => Self::Zig,
            _ => Self::Unknown,
        }
    }
}

/// Kind of reference between symbols
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReferenceKind {
    /// Function or method call
    Call,
    /// Type used in signature, variable declaration, or generic
    TypeReference,
    /// Struct/object field access
    FieldAccess,
    /// Import/use statement
    Import,
    /// Trait implementation or class inheritance
    Inheritance,
    /// Macro invocation
    MacroInvocation,
    /// Variable/constant/static reference
    VariableReference,
}

impl ReferenceKind {
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

/// Unresolved reference found during parsing
#[derive(Debug, Clone)]
pub struct ParsedReference {
    /// Name of the referenced symbol (may be qualified like "foo::bar")
    pub name: String,
    /// Kind of reference
    pub kind: ReferenceKind,
    /// Location where the reference occurs
    pub location: Location,
    /// The containing symbol's index in the parsed symbols list (for resolution)
    pub containing_symbol_index: Option<usize>,
}

/// Resolved reference stored in database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reference {
    /// Symbol that contains the reference (caller)
    pub from_symbol_id: SymbolId,
    /// Symbol being referenced (callee)
    pub to_symbol_id: SymbolId,
    /// Kind of reference
    pub kind: ReferenceKind,
    /// Location where the reference occurs
    pub location: Location,
}

/// Symbol usage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolUsage {
    pub symbol: Symbol,
    /// Number of times this symbol is referenced
    pub reference_count: u64,
    /// Symbols that reference this one (callers)
    pub callers: Vec<Symbol>,
    /// Symbols that this one references (callees)
    pub callees: Vec<Symbol>,
}

#[derive(Debug, Clone)]
pub struct ParsedSymbol {
    pub name: String,
    pub kind: SymbolKind,
    pub location: Location,
    pub signature: Option<String>,
    pub doc_comment: Option<String>,
    pub children: Vec<ParsedSymbol>,
    pub visibility: Visibility,
}

#[derive(Debug, Clone)]
pub struct ParsedFile {
    pub language: Language,
    pub symbols: Vec<ParsedSymbol>,
    /// Unresolved references found in this file
    pub references: Vec<ParsedReference>,
}
