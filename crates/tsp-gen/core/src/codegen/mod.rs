//! Code Generators
//!
//! Generate Python, TypeScript, Rust code, and OpenAPI specs from TypeSpec AST.

pub mod openapi;
pub mod protocol;
pub mod python;
pub mod rust;
pub mod ts_adi;
pub mod ts_eventbus;
pub mod ts_protocol;
pub mod typescript;

use crate::ast::{Model, Property, TypeRef, TypeSpecFile};
use std::collections::HashMap;
use std::path::Path;
use thiserror::Error;

/// Map of scalar name -> base type it extends
pub type ScalarMap = HashMap<String, String>;

/// Map of model name -> Model
pub type ModelMap<'a> = HashMap<&'a str, &'a Model>;

/// Build a map of custom scalars from parsed TypeSpec file
pub fn build_scalar_map(file: &TypeSpecFile) -> ScalarMap {
    file.scalars()
        .filter_map(|s| {
            s.extends
                .as_ref()
                .map(|base| (s.name.clone(), base.clone()))
        })
        .collect()
}

/// Build a map of model names to models for spread resolution
pub fn build_model_map(file: &TypeSpecFile) -> ModelMap<'_> {
    file.models().map(|m| (m.name.as_str(), m)).collect()
}

/// Resolve spread references and return all properties including spread ones
pub fn resolve_properties<'a>(model: &'a Model, models: &'a ModelMap<'a>) -> Vec<&'a Property> {
    let mut properties = Vec::new();

    // First add properties from spread references
    for spread_ref in &model.spread_refs {
        if let Some(name) = get_type_name(spread_ref) {
            if let Some(spread_model) = models.get(name.as_str()) {
                // Recursively resolve spread model's properties
                properties.extend(resolve_properties(spread_model, models));
            }
        }
    }

    // Then add this model's own properties (they override spread ones)
    properties.extend(model.properties.iter());

    properties
}

/// Get the type name from a TypeRef
fn get_type_name(type_ref: &TypeRef) -> Option<String> {
    match type_ref {
        TypeRef::Named(name) => Some(name.clone()),
        TypeRef::Qualified(parts) => parts.last().cloned(),
        _ => None,
    }
}

#[derive(Debug, Error)]
pub enum CodegenError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Format error: {0}")]
    Fmt(#[from] std::fmt::Error),

    #[error("Generation error: {0}")]
    Generation(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "cli", derive(clap::ValueEnum))]
pub enum Language {
    #[cfg_attr(feature = "cli", value(name = "python"))]
    Python,
    #[cfg_attr(feature = "cli", value(name = "typescript", alias = "ts"))]
    TypeScript,
    #[cfg_attr(feature = "cli", value(name = "rust", alias = "rs"))]
    Rust,
    #[cfg_attr(feature = "cli", value(name = "openapi", alias = "oas"))]
    OpenApi,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "cli", derive(clap::ValueEnum))]
pub enum Side {
    Client,
    Server,
    Both,
    /// Generate standalone types crate (models + enums only)
    Types,
    /// Generate AdiService implementation for WebRTC transport
    #[cfg_attr(feature = "cli", value(name = "adi", alias = "adi-service"))]
    AdiService,
    /// Generate WebSocket protocol messages, types, and handler traits
    #[cfg_attr(feature = "cli", value(name = "protocol", alias = "proto"))]
    Protocol,
    /// Generate EventBus types and module augmentation from @bus interfaces
    #[cfg_attr(feature = "cli", value(name = "eventbus", alias = "bus"))]
    EventBus,
}

pub struct Generator<'a> {
    file: &'a TypeSpecFile,
    output_dir: &'a Path,
    package_name: &'a str,
    rust_server_config: rust::RustServerConfig,
    rust_adi_config: Option<rust::RustAdiServiceConfig>,
    rust_protocol_config: Option<protocol::RustProtocolConfig>,
    eventbus_config: Option<ts_eventbus::EventBusConfig>,
    types_crate: Option<String>,
}

impl<'a> Generator<'a> {
    pub fn new(file: &'a TypeSpecFile, output_dir: &'a Path, package_name: &'a str) -> Self {
        Self {
            file,
            output_dir,
            package_name,
            rust_server_config: rust::RustServerConfig::default(),
            rust_adi_config: None,
            rust_protocol_config: None,
            eventbus_config: None,
            types_crate: None,
        }
    }

    pub fn with_rust_config(mut self, config: rust::RustServerConfig) -> Self {
        self.rust_server_config = config;
        self
    }

    pub fn with_rust_adi_config(mut self, config: rust::RustAdiServiceConfig) -> Self {
        self.rust_adi_config = Some(config);
        self
    }

    pub fn with_rust_protocol_config(mut self, config: protocol::RustProtocolConfig) -> Self {
        self.rust_protocol_config = Some(config);
        self
    }

    pub fn with_eventbus_config(mut self, config: ts_eventbus::EventBusConfig) -> Self {
        self.eventbus_config = Some(config);
        self
    }

    pub fn with_types_crate(mut self, types_crate: String) -> Self {
        self.types_crate = Some(types_crate);
        self
    }

    pub fn generate(&self, language: Language, side: Side) -> Result<Vec<String>, CodegenError> {
        let mut generated = Vec::new();

        // EventBus generation is TypeScript-only
        if side == Side::EventBus {
            let config = self.eventbus_config.as_ref().ok_or_else(|| {
                CodegenError::Generation(
                    "EventBus generation requires --eventbus-module and --eventbus-interface flags"
                        .to_string(),
                )
            })?;

            return match language {
                Language::TypeScript => {
                    generated.extend(ts_eventbus::generate(
                        self.file,
                        self.output_dir,
                        self.package_name,
                        config,
                    )?);
                    Ok(generated)
                }
                _ => Err(CodegenError::Generation(format!(
                    "EventBus generation not supported for {language:?}"
                ))),
            };
        }

        // Protocol generation is self-contained per language
        if side == Side::Protocol {
            let config = self.rust_protocol_config.as_ref().ok_or_else(|| {
                CodegenError::Generation(
                    "Protocol generation requires --protocol-tag and --protocol-rename flags"
                        .to_string(),
                )
            })?;

            return match language {
                Language::Rust => {
                    generated.extend(protocol::generate(
                        self.file,
                        self.output_dir,
                        self.package_name,
                        config,
                    )?);
                    Ok(generated)
                }
                Language::TypeScript => {
                    generated.extend(ts_protocol::generate(
                        self.file,
                        self.output_dir,
                        self.package_name,
                        config,
                    )?);
                    Ok(generated)
                }
                _ => Err(CodegenError::Generation(format!(
                    "Protocol generation not supported for {language:?}"
                ))),
            };
        }

        // AdiService generation supports both Rust and TypeScript
        if side == Side::AdiService {
            return match language {
                Language::Rust => {
                    generated.extend(rust::generate_with_config(
                        self.file,
                        self.output_dir,
                        self.package_name,
                        side,
                        &self.rust_server_config,
                        self.types_crate.as_deref(),
                        self.rust_adi_config.as_ref(),
                    )?);
                    Ok(generated)
                }
                Language::TypeScript => {
                    generated.extend(ts_adi::generate(
                        self.file,
                        self.output_dir,
                        self.package_name,
                    )?);
                    Ok(generated)
                }
                _ => Err(CodegenError::Generation(format!(
                    "AdiService generation not supported for {language:?}"
                ))),
            };
        }

        match language {
            Language::Python => {
                generated.extend(python::generate(
                    self.file,
                    self.output_dir,
                    self.package_name,
                    side,
                )?);
            }
            Language::TypeScript => {
                generated.extend(typescript::generate(
                    self.file,
                    self.output_dir,
                    self.package_name,
                    side,
                )?);
            }
            Language::Rust => {
                generated.extend(rust::generate_with_config(
                    self.file,
                    self.output_dir,
                    self.package_name,
                    side,
                    &self.rust_server_config,
                    self.types_crate.as_deref(),
                    self.rust_adi_config.as_ref(),
                )?);
            }
            Language::OpenApi => {
                generated.extend(openapi::generate(
                    self.file,
                    self.output_dir,
                    self.package_name,
                )?);
            }
        }

        Ok(generated)
    }
}
