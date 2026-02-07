//! Adapter to use v3 plugin LanguageAnalyzer as internal LanguageAnalyzer trait.

use lib_plugin_abi_v3::lang::LanguageAnalyzer as V3LanguageAnalyzer;
use std::sync::Arc;
use tree_sitter::Tree;

use super::treesitter::analyzers::LanguageAnalyzer;
use crate::types::{
    Language, Location, ParsedReference, ParsedSymbol, ReferenceKind, SymbolKind, Visibility,
};

/// Adapter that bridges v3 plugin LanguageAnalyzer to the internal LanguageAnalyzer trait.
pub struct PluginAnalyzerAdapter {
    plugin: Arc<dyn V3LanguageAnalyzer>,
    #[allow(dead_code)]
    language: Language,
}

impl PluginAnalyzerAdapter {
    /// Create a new plugin analyzer adapter.
    pub fn new(plugin: Arc<dyn V3LanguageAnalyzer>, language: Language) -> Self {
        Self { plugin, language }
    }
}

impl LanguageAnalyzer for PluginAnalyzerAdapter {
    fn extract_symbols(&self, source: &str, _tree: &Tree) -> Vec<ParsedSymbol> {
        // Use tokio runtime to call async method
        let plugin = self.plugin.clone();
        let source = source.to_string();
        
        let result = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                plugin.extract_symbols(&source).await
            })
        });

        match result {
            Ok(symbols) => symbols.into_iter().map(convert_v3_symbol).collect(),
            Err(e) => {
                tracing::error!("Plugin extract_symbols failed: {}", e);
                vec![]
            }
        }
    }

    fn extract_references(&self, source: &str, _tree: &Tree) -> Vec<ParsedReference> {
        // Use tokio runtime to call async method
        let plugin = self.plugin.clone();
        let source = source.to_string();
        
        let result = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                plugin.extract_references(&source).await
            })
        });

        match result {
            Ok(refs) => refs.into_iter().map(convert_v3_reference).collect(),
            Err(e) => {
                tracing::error!("Plugin extract_references failed: {}", e);
                vec![]
            }
        }
    }
}

/// Convert v3 ABI symbol to internal ParsedSymbol.
fn convert_v3_symbol(v3: lib_plugin_abi_v3::lang::ParsedSymbol) -> ParsedSymbol {
    ParsedSymbol {
        name: v3.name,
        kind: convert_v3_symbol_kind(v3.kind),
        location: convert_v3_location(v3.location),
        signature: v3.signature,
        doc_comment: v3.doc_comment,
        visibility: convert_v3_visibility(v3.visibility),
        children: v3.children.into_iter().map(convert_v3_symbol).collect(),
    }
}

/// Convert v3 ABI reference to internal ParsedReference.
fn convert_v3_reference(v3: lib_plugin_abi_v3::lang::ParsedReference) -> ParsedReference {
    ParsedReference {
        name: v3.name,
        kind: convert_v3_reference_kind(v3.kind),
        location: convert_v3_location(v3.location),
        containing_symbol_index: v3.containing_symbol_index.map(|i| i as usize),
    }
}

/// Convert v3 ABI location to internal Location.
fn convert_v3_location(v3: lib_plugin_abi_v3::lang::Location) -> Location {
    Location {
        start_line: v3.start_line,
        start_col: v3.start_col,
        end_line: v3.end_line,
        end_col: v3.end_col,
        start_byte: v3.start_byte,
        end_byte: v3.end_byte,
    }
}

/// Convert v3 ABI symbol kind to internal SymbolKind.
fn convert_v3_symbol_kind(v3: lib_plugin_abi_v3::lang::SymbolKind) -> SymbolKind {
    match v3 {
        lib_plugin_abi_v3::lang::SymbolKind::Function => SymbolKind::Function,
        lib_plugin_abi_v3::lang::SymbolKind::Method => SymbolKind::Method,
        lib_plugin_abi_v3::lang::SymbolKind::Class => SymbolKind::Class,
        lib_plugin_abi_v3::lang::SymbolKind::Struct => SymbolKind::Struct,
        lib_plugin_abi_v3::lang::SymbolKind::Enum => SymbolKind::Enum,
        lib_plugin_abi_v3::lang::SymbolKind::Interface => SymbolKind::Interface,
        lib_plugin_abi_v3::lang::SymbolKind::Trait => SymbolKind::Trait,
        lib_plugin_abi_v3::lang::SymbolKind::Module => SymbolKind::Module,
        lib_plugin_abi_v3::lang::SymbolKind::Constant => SymbolKind::Constant,
        lib_plugin_abi_v3::lang::SymbolKind::Variable => SymbolKind::Variable,
        lib_plugin_abi_v3::lang::SymbolKind::Type => SymbolKind::Type,
        lib_plugin_abi_v3::lang::SymbolKind::Property => SymbolKind::Property,
        lib_plugin_abi_v3::lang::SymbolKind::Field => SymbolKind::Field,
        lib_plugin_abi_v3::lang::SymbolKind::Constructor => SymbolKind::Constructor,
        lib_plugin_abi_v3::lang::SymbolKind::Destructor => SymbolKind::Destructor,
        lib_plugin_abi_v3::lang::SymbolKind::Operator => SymbolKind::Operator,
        lib_plugin_abi_v3::lang::SymbolKind::Macro => SymbolKind::Macro,
        lib_plugin_abi_v3::lang::SymbolKind::Namespace => SymbolKind::Namespace,
        lib_plugin_abi_v3::lang::SymbolKind::Package => SymbolKind::Package,
        lib_plugin_abi_v3::lang::SymbolKind::Unknown => SymbolKind::Unknown,
    }
}

/// Convert v3 ABI visibility to internal Visibility.
fn convert_v3_visibility(v3: lib_plugin_abi_v3::lang::Visibility) -> Visibility {
    match v3 {
        lib_plugin_abi_v3::lang::Visibility::Public => Visibility::Public,
        lib_plugin_abi_v3::lang::Visibility::PublicCrate => Visibility::PublicCrate,
        lib_plugin_abi_v3::lang::Visibility::PublicSuper => Visibility::PublicSuper,
        lib_plugin_abi_v3::lang::Visibility::Protected => Visibility::Protected,
        lib_plugin_abi_v3::lang::Visibility::Private => Visibility::Private,
        lib_plugin_abi_v3::lang::Visibility::Internal => Visibility::Internal,
        lib_plugin_abi_v3::lang::Visibility::Unknown => Visibility::Unknown,
    }
}

/// Convert v3 ABI reference kind to internal ReferenceKind.
fn convert_v3_reference_kind(v3: lib_plugin_abi_v3::lang::ReferenceKind) -> ReferenceKind {
    match v3 {
        lib_plugin_abi_v3::lang::ReferenceKind::Call => ReferenceKind::Call,
        lib_plugin_abi_v3::lang::ReferenceKind::TypeReference => ReferenceKind::TypeReference,
        lib_plugin_abi_v3::lang::ReferenceKind::FieldAccess => ReferenceKind::FieldAccess,
        lib_plugin_abi_v3::lang::ReferenceKind::Import => ReferenceKind::Import,
        lib_plugin_abi_v3::lang::ReferenceKind::Inheritance => ReferenceKind::Inheritance,
        lib_plugin_abi_v3::lang::ReferenceKind::MacroInvocation => ReferenceKind::MacroInvocation,
        lib_plugin_abi_v3::lang::ReferenceKind::VariableReference => ReferenceKind::VariableReference,
    }
}
