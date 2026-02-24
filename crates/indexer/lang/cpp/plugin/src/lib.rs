//! C/C++ Language Support Plugin

mod analyzer;

use lib_plugin_abi_v3::{
    async_trait,
    lang::{
        LanguageAnalyzer, LanguageInfo, Location, ParsedReference, ParsedSymbol, ReferenceKind,
        SymbolKind, Visibility,
    },
    Plugin, PluginContext, PluginMetadata, PluginType, Result as PluginResult,
    SERVICE_LANGUAGE_ANALYZER,
};

/// C++ Language Analyzer Plugin
pub struct CppAnalyzer;

#[async_trait]
impl Plugin for CppAnalyzer {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            id: "adi.lang.cpp".to_string(),
            name: "C++ Language Support".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            plugin_type: PluginType::Extension,
            author: Some("ADI Team".to_string()),
            description: Some("C++ language parsing and analysis".to_string()),
            category: None,
        }
    }

    async fn init(&mut self, _ctx: &PluginContext) -> PluginResult<()> {
        Ok(())
    }

    async fn shutdown(&self) -> PluginResult<()> {
        Ok(())
    }

    fn provides(&self) -> Vec<&'static str> {
        vec![SERVICE_LANGUAGE_ANALYZER]
    }
}

#[async_trait]
impl LanguageAnalyzer for CppAnalyzer {
    fn language_info(&self) -> LanguageInfo {
        LanguageInfo::new("cpp", "C++")
            .with_extensions(["cpp", "cc", "cxx", "hpp", "hh", "hxx"])
            .with_version(env!("CARGO_PKG_VERSION"))
    }

    async fn extract_symbols(&self, source: &str) -> PluginResult<Vec<ParsedSymbol>> {
        Ok(analyzer::extract_symbols(source, true)
            .into_iter()
            .map(convert_symbol)
            .collect())
    }

    async fn extract_references(&self, source: &str) -> PluginResult<Vec<ParsedReference>> {
        Ok(analyzer::extract_references(source, true)
            .into_iter()
            .map(convert_reference)
            .collect())
    }

    fn tree_sitter_language(&self) -> *const () {
        &tree_sitter_cpp::LANGUAGE as *const _ as *const ()
    }
}

/// C Language Analyzer Plugin
pub struct CAnalyzer;

#[async_trait]
impl Plugin for CAnalyzer {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            id: "adi.lang.c".to_string(),
            name: "C Language Support".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            plugin_type: PluginType::Extension,
            author: Some("ADI Team".to_string()),
            description: Some("C language parsing and analysis".to_string()),
            category: None,
        }
    }

    async fn init(&mut self, _ctx: &PluginContext) -> PluginResult<()> {
        Ok(())
    }

    async fn shutdown(&self) -> PluginResult<()> {
        Ok(())
    }

    fn provides(&self) -> Vec<&'static str> {
        vec![SERVICE_LANGUAGE_ANALYZER]
    }
}

#[async_trait]
impl LanguageAnalyzer for CAnalyzer {
    fn language_info(&self) -> LanguageInfo {
        LanguageInfo::new("c", "C")
            .with_extensions(["c", "h"])
            .with_version(env!("CARGO_PKG_VERSION"))
    }

    async fn extract_symbols(&self, source: &str) -> PluginResult<Vec<ParsedSymbol>> {
        Ok(analyzer::extract_symbols(source, false)
            .into_iter()
            .map(convert_symbol)
            .collect())
    }

    async fn extract_references(&self, source: &str) -> PluginResult<Vec<ParsedReference>> {
        Ok(analyzer::extract_references(source, false)
            .into_iter()
            .map(convert_reference)
            .collect())
    }

    fn tree_sitter_language(&self) -> *const () {
        &tree_sitter_c::LANGUAGE as *const _ as *const ()
    }
}

// Helper functions to convert from internal types to v3 ABI types

fn convert_symbol(sym: analyzer::InternalSymbol) -> ParsedSymbol {
    let location = Location::new(
        sym.location.start_line,
        sym.location.start_col,
        sym.location.end_line,
        sym.location.end_col,
        sym.location.start_byte,
        sym.location.end_byte,
    );

    let mut result = ParsedSymbol::new(sym.name, convert_symbol_kind(sym.kind), location)
        .with_visibility(Visibility::Unknown);

    if let Some(sig) = sym.signature {
        result = result.with_signature(sig);
    }

    if !sym.children.is_empty() {
        result = result.with_children(sym.children.into_iter().map(convert_symbol).collect());
    }

    result
}

fn convert_reference(r: analyzer::InternalReference) -> ParsedReference {
    let location = Location::new(
        r.location.start_line,
        r.location.start_col,
        r.location.end_line,
        r.location.end_col,
        r.location.start_byte,
        r.location.end_byte,
    );

    ParsedReference::new(r.name, convert_reference_kind(r.kind), location)
}

fn convert_symbol_kind(kind: analyzer::InternalSymbolKind) -> SymbolKind {
    match kind {
        analyzer::InternalSymbolKind::Function => SymbolKind::Function,
        analyzer::InternalSymbolKind::Method => SymbolKind::Method,
        analyzer::InternalSymbolKind::Class => SymbolKind::Class,
        analyzer::InternalSymbolKind::Struct => SymbolKind::Struct,
        analyzer::InternalSymbolKind::Enum => SymbolKind::Enum,
        analyzer::InternalSymbolKind::Namespace => SymbolKind::Namespace,
        analyzer::InternalSymbolKind::Field => SymbolKind::Field,
    }
}

fn convert_reference_kind(kind: analyzer::InternalReferenceKind) -> ReferenceKind {
    match kind {
        analyzer::InternalReferenceKind::Call => ReferenceKind::Call,
        analyzer::InternalReferenceKind::Import => ReferenceKind::Import,
        analyzer::InternalReferenceKind::TypeReference => ReferenceKind::TypeReference,
        analyzer::InternalReferenceKind::FieldAccess => ReferenceKind::FieldAccess,
        analyzer::InternalReferenceKind::Inheritance => ReferenceKind::Inheritance,
    }
}

#[no_mangle]
pub fn plugin_create() -> Box<dyn Plugin> {
    Box::new(CppAnalyzer)
}

/// Additional entry point for C analyzer
#[no_mangle]
pub fn plugin_create_c() -> Box<dyn Plugin> {
    Box::new(CAnalyzer)
}
