//! Adapter to use plugin services as LanguageAnalyzer trait.

use lib_indexer_lang_abi::{
    ExtractRequest, LocationAbi, ParsedReferenceAbi, ParsedSymbolAbi, ReferenceKindAbi,
    SymbolKindAbi, VisibilityAbi, METHOD_EXTRACT_REFERENCES, METHOD_EXTRACT_SYMBOLS,
};
use lib_plugin_abi::ServiceHandle;
use tracing;
use tree_sitter::Tree;

use super::treesitter::analyzers::LanguageAnalyzer;
use crate::types::{
    Language, Location, ParsedReference, ParsedSymbol, ReferenceKind, SymbolKind, Visibility,
};

/// Adapter that bridges plugin services to the LanguageAnalyzer trait.
pub struct PluginAnalyzerAdapter {
    service: ServiceHandle,
    #[allow(dead_code)]
    language: Language,
}

impl PluginAnalyzerAdapter {
    /// Create a new plugin analyzer adapter.
    pub fn new(service: ServiceHandle, language: Language) -> Self {
        Self { service, language }
    }
}

impl LanguageAnalyzer for PluginAnalyzerAdapter {
    fn extract_symbols(&self, source: &str, tree: &Tree) -> Vec<ParsedSymbol> {
        let request = ExtractRequest {
            source: source.to_string(),
            tree_sexp: tree.root_node().to_sexp(),
        };

        let args = match serde_json::to_string(&request) {
            Ok(a) => a,
            Err(e) => {
                tracing::error!("Failed to serialize extract_symbols request: {}", e);
                return vec![];
            }
        };

        let response = match unsafe { self.service.invoke(METHOD_EXTRACT_SYMBOLS, &args) } {
            Ok(r) => r,
            Err(e) => {
                tracing::error!("Plugin extract_symbols failed: {}", e);
                return vec![];
            }
        };

        let abi_symbols: Vec<ParsedSymbolAbi> = match serde_json::from_str(&response) {
            Ok(s) => s,
            Err(e) => {
                tracing::error!("Failed to parse extract_symbols response: {}", e);
                return vec![];
            }
        };

        abi_symbols.into_iter().map(convert_symbol).collect()
    }

    fn extract_references(&self, source: &str, tree: &Tree) -> Vec<ParsedReference> {
        let request = ExtractRequest {
            source: source.to_string(),
            tree_sexp: tree.root_node().to_sexp(),
        };

        let args = match serde_json::to_string(&request) {
            Ok(a) => a,
            Err(e) => {
                tracing::error!("Failed to serialize extract_references request: {}", e);
                return vec![];
            }
        };

        let response = match unsafe { self.service.invoke(METHOD_EXTRACT_REFERENCES, &args) } {
            Ok(r) => r,
            Err(e) => {
                tracing::error!("Plugin extract_references failed: {}", e);
                return vec![];
            }
        };

        let abi_refs: Vec<ParsedReferenceAbi> = match serde_json::from_str(&response) {
            Ok(r) => r,
            Err(e) => {
                tracing::error!("Failed to parse extract_references response: {}", e);
                return vec![];
            }
        };

        abi_refs.into_iter().map(convert_reference).collect()
    }
}

/// Convert ABI symbol to internal ParsedSymbol.
fn convert_symbol(abi: ParsedSymbolAbi) -> ParsedSymbol {
    ParsedSymbol {
        name: abi.name.to_string(),
        kind: convert_symbol_kind(abi.kind),
        location: convert_location(abi.location),
        signature: abi.signature.into_option().map(|s| s.to_string()),
        doc_comment: abi.doc_comment.into_option().map(|s| s.to_string()),
        visibility: convert_visibility(abi.visibility),
        children: abi.children.into_iter().map(convert_symbol).collect(),
    }
}

/// Convert ABI reference to internal ParsedReference.
fn convert_reference(abi: ParsedReferenceAbi) -> ParsedReference {
    ParsedReference {
        name: abi.name.to_string(),
        kind: convert_reference_kind(abi.kind),
        location: convert_location(abi.location),
        containing_symbol_index: abi
            .containing_symbol_index
            .into_option()
            .map(|i| i as usize),
    }
}

/// Convert ABI location to internal Location.
fn convert_location(abi: LocationAbi) -> Location {
    Location {
        start_line: abi.start_line,
        start_col: abi.start_col,
        end_line: abi.end_line,
        end_col: abi.end_col,
        start_byte: abi.start_byte,
        end_byte: abi.end_byte,
    }
}

/// Convert ABI symbol kind to internal SymbolKind.
fn convert_symbol_kind(abi: SymbolKindAbi) -> SymbolKind {
    match abi {
        SymbolKindAbi::Function => SymbolKind::Function,
        SymbolKindAbi::Method => SymbolKind::Method,
        SymbolKindAbi::Class => SymbolKind::Class,
        SymbolKindAbi::Struct => SymbolKind::Struct,
        SymbolKindAbi::Enum => SymbolKind::Enum,
        SymbolKindAbi::Interface => SymbolKind::Interface,
        SymbolKindAbi::Trait => SymbolKind::Trait,
        SymbolKindAbi::Module => SymbolKind::Module,
        SymbolKindAbi::Constant => SymbolKind::Constant,
        SymbolKindAbi::Variable => SymbolKind::Variable,
        SymbolKindAbi::Type => SymbolKind::Type,
        SymbolKindAbi::Property => SymbolKind::Property,
        SymbolKindAbi::Field => SymbolKind::Field,
        SymbolKindAbi::Constructor => SymbolKind::Constructor,
        SymbolKindAbi::Destructor => SymbolKind::Destructor,
        SymbolKindAbi::Operator => SymbolKind::Operator,
        SymbolKindAbi::Macro => SymbolKind::Macro,
        SymbolKindAbi::Namespace => SymbolKind::Namespace,
        SymbolKindAbi::Package => SymbolKind::Package,
        SymbolKindAbi::Unknown => SymbolKind::Unknown,
    }
}

/// Convert ABI visibility to internal Visibility.
fn convert_visibility(abi: VisibilityAbi) -> Visibility {
    match abi {
        VisibilityAbi::Public => Visibility::Public,
        VisibilityAbi::PublicCrate => Visibility::PublicCrate,
        VisibilityAbi::PublicSuper => Visibility::PublicSuper,
        VisibilityAbi::Protected => Visibility::Protected,
        VisibilityAbi::Private => Visibility::Private,
        VisibilityAbi::Internal => Visibility::Internal,
        VisibilityAbi::Unknown => Visibility::Unknown,
    }
}

/// Convert ABI reference kind to internal ReferenceKind.
fn convert_reference_kind(abi: ReferenceKindAbi) -> ReferenceKind {
    match abi {
        ReferenceKindAbi::Call => ReferenceKind::Call,
        ReferenceKindAbi::TypeReference => ReferenceKind::TypeReference,
        ReferenceKindAbi::FieldAccess => ReferenceKind::FieldAccess,
        ReferenceKindAbi::Import => ReferenceKind::Import,
        ReferenceKindAbi::Inheritance => ReferenceKind::Inheritance,
        ReferenceKindAbi::MacroInvocation => ReferenceKind::MacroInvocation,
        ReferenceKindAbi::VariableReference => ReferenceKind::VariableReference,
    }
}
