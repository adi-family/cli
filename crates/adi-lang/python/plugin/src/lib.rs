//! Python Language Support Plugin

mod analyzer;

use lib_plugin_abi_v3::{
    async_trait,
    lang::{LanguageAnalyzer, LanguageInfo, ParsedReference, ParsedSymbol},
    Plugin, PluginContext, PluginMetadata, PluginType, Result as PluginResult,
    SERVICE_LANGUAGE_ANALYZER,
};

pub struct PythonAnalyzer;

#[async_trait]
impl Plugin for PythonAnalyzer {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            id: "adi.lang.python".to_string(),
            name: "Python Language Support".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            plugin_type: PluginType::Extension,
            author: Some("ADI Team".to_string()),
            description: Some("Python language parsing and analysis for ADI indexer".to_string()),
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
impl LanguageAnalyzer for PythonAnalyzer {
    fn language_info(&self) -> LanguageInfo {
        LanguageInfo::new("python", "Python")
            .with_extensions(["py", "pyi", "pyw"])
            .with_version(env!("CARGO_PKG_VERSION"))
    }

    async fn extract_symbols(&self, source: &str) -> PluginResult<Vec<ParsedSymbol>> {
        Ok(analyzer::extract_symbols(source))
    }

    async fn extract_references(&self, source: &str) -> PluginResult<Vec<ParsedReference>> {
        Ok(analyzer::extract_references(source))
    }

    fn tree_sitter_language(&self) -> *const () {
        &tree_sitter_python::LANGUAGE as *const _ as *const ()
    }
}

#[no_mangle]
pub fn plugin_create() -> Box<dyn Plugin> {
    Box::new(PythonAnalyzer)
}
