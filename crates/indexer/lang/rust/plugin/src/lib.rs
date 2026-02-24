//! Rust Language Support Plugin
//!
//! Provides Rust language parsing and analysis for the ADI indexer.

mod analyzer;

use lib_plugin_abi_v3::{
    async_trait, lang::{LanguageAnalyzer, LanguageInfo, ParsedReference, ParsedSymbol},
    Plugin, PluginContext, PluginMetadata, PluginType, Result as PluginResult,
    SERVICE_LANGUAGE_ANALYZER,
};

pub struct RustAnalyzer;

#[async_trait]
impl Plugin for RustAnalyzer {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            id: "adi.lang.rust".to_string(),
            name: "Rust Language Support".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            plugin_type: PluginType::Extension,
            author: Some("ADI Team".to_string()),
            description: Some("Rust language parsing and analysis for ADI indexer".to_string()),
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
impl LanguageAnalyzer for RustAnalyzer {
    fn language_info(&self) -> LanguageInfo {
        LanguageInfo::new("rust", "Rust")
            .with_extensions(["rs"])
            .with_version(env!("CARGO_PKG_VERSION"))
    }

    async fn extract_symbols(&self, source: &str) -> PluginResult<Vec<ParsedSymbol>> {
        Ok(analyzer::extract_symbols(source))
    }

    async fn extract_references(&self, source: &str) -> PluginResult<Vec<ParsedReference>> {
        Ok(analyzer::extract_references(source))
    }

    fn tree_sitter_language(&self) -> *const () {
        // Return the tree-sitter language pointer for external use
        let lang: tree_sitter::Language = tree_sitter_rust::LANGUAGE.into();
        // Safety: The language is static and valid for the lifetime of the plugin
        std::ptr::addr_of!(lang) as *const ()
    }
}

#[no_mangle]
pub fn plugin_create() -> Box<dyn Plugin> {
    Box::new(RustAnalyzer)
}
