//! Korean translation plugin for ADI Workflow (v3)
//!
//! Provides Korean (ko-KR) translations via Fluent message format.

use lib_plugin_abi_v3::*;
use serde::{Deserialize, Serialize};

// Embedded Fluent messages at compile time
const MESSAGES_FTL: &str = include_str!("../messages.ftl");

/// Translation metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
struct TranslationMetadata {
    plugin_id: String,
    language: String,
    language_name: String,
    namespace: String,
    version: String,
}

/// Korean translation plugin
pub struct KoreanTranslationPlugin {
    metadata: TranslationMetadata,
}

impl KoreanTranslationPlugin {
    pub fn new() -> Self {
        Self {
            metadata: TranslationMetadata {
                plugin_id: "adi.workflow".to_string(),
                language: "ko-KR".to_string(),
                language_name: "한국어".to_string(),
                namespace: "workflow".to_string(),
                version: "3.0.0".to_string(),
            },
        }
    }

    /// Get Fluent messages (.ftl file content)
    pub fn get_messages(&self) -> &'static str {
        MESSAGES_FTL
    }

    /// Get translation metadata as JSON
    pub fn get_metadata_json(&self) -> Result<String> {
        Ok(serde_json::to_string(&self.metadata)?)
    }
}

#[async_trait]
impl Plugin for KoreanTranslationPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            id: "adi.workflow.ko-KR".to_string(),
            name: "ADI Workflow - 한국어".to_string(),
            version: "3.0.0".to_string(),
            plugin_type: PluginType::Extension,
            author: Some("ADI Team".to_string()),
            description: Some("ADI Workflow 한국어 번역".to_string()),
            category: None,
        }
    }

    async fn init(&mut self, _ctx: &PluginContext) -> Result<()> {
        // No initialization needed for static translations
        Ok(())
    }

    async fn shutdown(&self) -> Result<()> {
        // No cleanup needed
        Ok(())
    }
}

impl Default for KoreanTranslationPlugin {
    fn default() -> Self {
        Self::new()
    }
}

// Plugin entry point
#[no_mangle]
pub fn plugin_create() -> Box<dyn Plugin> {
    Box::new(KoreanTranslationPlugin::new())
}
