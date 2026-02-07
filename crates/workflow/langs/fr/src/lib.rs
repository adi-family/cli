//! French translation plugin for ADI Workflow (v3)
//!
//! Provides French (fr-FR) translations via Fluent message format.

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

/// French translation plugin
pub struct FrenchTranslationPlugin {
    metadata: TranslationMetadata,
}

impl FrenchTranslationPlugin {
    pub fn new() -> Self {
        Self {
            metadata: TranslationMetadata {
                plugin_id: "adi.workflow".to_string(),
                language: "fr-FR".to_string(),
                language_name: "Français".to_string(),
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
impl Plugin for FrenchTranslationPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            id: "adi.workflow.fr-FR".to_string(),
            name: "ADI Workflow - Français".to_string(),
            version: "3.0.0".to_string(),
            plugin_type: PluginType::Extension,
            author: Some("ADI Team".to_string()),
            description: Some("Traductions françaises pour ADI Workflow".to_string()),
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

impl Default for FrenchTranslationPlugin {
    fn default() -> Self {
        Self::new()
    }
}

// Plugin entry point
#[no_mangle]
pub fn plugin_create() -> Box<dyn Plugin> {
    Box::new(FrenchTranslationPlugin::new())
}
