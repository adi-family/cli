//! Protocol Code Generator
//!
//! Generates Rust code from TypeSpec protocol definitions.
//! Reads `@channel`, `@request`, `@relay`, `@event`, `@stream`, `@command`,
//! `@serverPush`, and `@scatter` decorators to produce:
//! - A serde-tagged enum with all message variants
//! - Supporting model/enum types
//! - Per-channel async handler traits

use std::fmt::Write;
use std::fs;
use std::path::Path;

use convert_case::{Case, Casing};

use crate::ast::{Decorator, OperationParam, Property, TypeRef, TypeSpecFile};

use super::rust::type_to_rust;
use super::{build_model_map, build_scalar_map, CodegenError, ModelMap, ScalarMap};

/// Configuration for protocol code generation.
#[derive(Debug, Clone)]
pub struct RustProtocolConfig {
    /// Serde tag field name (e.g., "type")
    pub tag: String,
    /// Serde rename strategy (e.g., "snake_case")
    pub rename: String,
    /// Name of the generated enum (e.g., "SignalingMessage")
    pub enum_name: String,
}

/// Message kind derived from operation decorators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MessageKind {
    /// Client → server RPC with typed response
    Request,
    /// Client → cocoon (relayed) RPC with typed response
    Command,
    /// Peer ↔ peer forwarding, no response
    Relay,
    /// One-way notification
    Event,
    /// High-frequency continuous events
    Stream,
    /// Server-initiated message
    ServerPush,
    /// Fan-out query to all devices
    Scatter,
}

/// A resolved enum variant ready for code generation.
#[allow(dead_code)]
pub(crate) struct EnumVariant {
    /// PascalCase variant name (e.g., "WebRtcStartSession")
    pub name: String,
    /// Resolved fields (language-agnostic — TypeRef stored, converted at render time)
    pub fields: Vec<VariantField>,
    /// Original message kind
    pub kind: MessageKind,
    /// Channel name this variant belongs to
    pub channel: String,
}

pub(crate) struct VariantField {
    /// snake_case field name
    pub name: String,
    /// Original type reference (resolved to target language at render time)
    pub type_ref: TypeRef,
    /// Whether the field is optional
    pub optional: bool,
}

/// A resolved handler method for a channel trait.
pub(crate) struct HandlerMethod {
    /// snake_case method name
    name: String,
    /// The request variant name (for typed param)
    request_variant: String,
    /// The response variant name (for typed return), if any
    response_variant: Option<String>,
    /// Message kind
    kind: MessageKind,
}

/// Entry point for protocol code generation.
pub fn generate(
    file: &TypeSpecFile,
    output_dir: &Path,
    _package_name: &str,
    config: &RustProtocolConfig,
) -> Result<Vec<String>, CodegenError> {
    let scalars = build_scalar_map(file);
    let models = build_model_map(file);

    let src_dir = output_dir.join("src");
    fs::create_dir_all(&src_dir)?;

    let mut generated = Vec::new();

    // Collect all enum variants and handler methods from channel interfaces
    let (variants, channel_handlers) = collect_protocol_data(file, &models);

    // Generate messages.rs
    let messages = generate_messages(&variants, config, &scalars)?;
    let messages_path = src_dir.join("messages.rs");
    fs::write(&messages_path, &messages)?;
    generated.push(messages_path.display().to_string());

    // Generate types.rs (models + enums)
    let types = generate_types(file, &scalars, &models)?;
    let types_path = src_dir.join("types.rs");
    fs::write(&types_path, &types)?;
    generated.push(types_path.display().to_string());

    // Generate handlers.rs
    let handlers = generate_handlers(&channel_handlers)?;
    let handlers_path = src_dir.join("handlers.rs");
    fs::write(&handlers_path, &handlers)?;
    generated.push(handlers_path.display().to_string());

    // Generate lib.rs
    let lib = generate_lib()?;
    let lib_path = src_dir.join("lib.rs");
    fs::write(&lib_path, &lib)?;
    generated.push(lib_path.display().to_string());

    Ok(generated)
}

/// Collect enum variants and handler methods from all channel interfaces.
pub(crate) fn collect_protocol_data(
    file: &TypeSpecFile,
    models: &ModelMap<'_>,
) -> (Vec<EnumVariant>, Vec<(String, Vec<HandlerMethod>)>) {
    let mut all_variants = Vec::new();
    let mut channel_handlers = Vec::new();

    for iface in file.interfaces() {
        let channel_name = match get_channel_name(&iface.decorators) {
            Some(name) => name,
            None => continue, // Skip interfaces without @channel
        };

        let prefix = channel_name.to_case(Case::Pascal);
        let mut methods = Vec::new();

        for op in &iface.operations {
            let kind = get_message_kind(&op.decorators);
            let op_pascal = op.name.to_case(Case::Pascal);
            let variant_name = format!("{}{}", prefix, op_pascal);

            // Resolve operation params to variant fields
            let fields = resolve_variant_fields(&op.params, models);

            all_variants.push(EnumVariant {
                name: variant_name.clone(),
                fields,
                kind,
                channel: channel_name.clone(),
            });

            // For request/command/scatter, also generate a response variant
            let response_variant = if matches!(kind, MessageKind::Request | MessageKind::Command | MessageKind::Scatter) {
                if let Some(ref return_type) = op.return_type {
                    let resp_name = derive_response_name(&prefix, &op_pascal);
                    let resp_fields = resolve_response_fields(return_type, models);

                    if !resp_fields.is_empty() {
                        all_variants.push(EnumVariant {
                            name: resp_name.clone(),
                            fields: resp_fields,
                            kind,
                            channel: channel_name.clone(),
                        });
                        Some(resp_name)
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            };

            methods.push(HandlerMethod {
                name: op.name.to_case(Case::Snake),
                request_variant: variant_name,
                response_variant,
                kind,
            });
        }

        channel_handlers.push((channel_name, methods));
    }

    (all_variants, channel_handlers)
}

/// Generate the messages.rs file containing the main protocol enum.
fn generate_messages(
    variants: &[EnumVariant],
    config: &RustProtocolConfig,
    scalars: &ScalarMap,
) -> Result<String, CodegenError> {
    let mut out = String::new();

    writeln!(out, "//! Auto-generated protocol messages from TypeSpec.")?;
    writeln!(out, "//! DO NOT EDIT.")?;
    writeln!(out)?;
    writeln!(out, "use serde::{{Deserialize, Serialize}};")?;
    writeln!(out, "use super::types::*;")?;
    writeln!(out)?;

    // Main enum
    writeln!(out, "#[derive(Debug, Clone, Serialize, Deserialize)]")?;
    writeln!(
        out,
        "#[serde(tag = \"{}\", rename_all = \"{}\")]",
        config.tag, config.rename
    )?;
    writeln!(out, "pub enum {} {{", config.enum_name)?;

    let mut current_channel = String::new();
    for variant in variants {
        // Add channel separator comment
        if variant.channel != current_channel {
            if !current_channel.is_empty() {
                writeln!(out)?;
            }
            writeln!(out, "    // ── {} ──", variant.channel)?;
            current_channel.clone_from(&variant.channel);
        }

        if variant.fields.is_empty() {
            writeln!(out, "    {},", variant.name)?;
        } else {
            writeln!(out, "    {} {{", variant.name)?;
            for field in &variant.fields {
                if field.optional {
                    writeln!(
                        out,
                        "        #[serde(skip_serializing_if = \"Option::is_none\")]"
                    )?;
                }
                let rust_type = type_to_rust(&field.type_ref, field.optional, scalars);
                writeln!(out, "        {}: {},", field.name, rust_type)?;
            }
            writeln!(out, "    }},")?;
        }
    }

    writeln!(out, "}}")?;

    Ok(out)
}

/// Generate the types.rs file containing supporting models and enums.
fn generate_types(
    file: &TypeSpecFile,
    scalars: &ScalarMap,
    models: &ModelMap<'_>,
) -> Result<String, CodegenError> {
    let mut out = String::new();

    writeln!(out, "//! Auto-generated protocol types from TypeSpec.")?;
    writeln!(out, "//! DO NOT EDIT.")?;
    writeln!(out)?;
    writeln!(out, "#![allow(unused_imports)]")?;
    writeln!(out)?;
    writeln!(out, "use serde::{{Deserialize, Serialize}};")?;
    writeln!(out)?;

    // Generate models
    for model in file.models() {
        let mut all_props: Vec<&Property> = Vec::new();

        // Resolve spread refs
        for spread_ref in &model.spread_refs {
            if let Some(name) = type_ref_name(spread_ref) {
                if let Some(spread_model) = models.get(name.as_str()) {
                    all_props.extend(spread_model.properties.iter());
                }
            }
        }
        all_props.extend(model.properties.iter());

        writeln!(out, "#[derive(Debug, Clone, Serialize, Deserialize)]")?;
        writeln!(out, "#[serde(rename_all = \"snake_case\")]")?;
        writeln!(out, "pub struct {} {{", model.name)?;

        for prop in &all_props {
            let rust_type = type_to_rust(&prop.type_ref, prop.optional, scalars);
            let field_name = prop.name.to_case(Case::Snake);

            if prop.optional {
                writeln!(
                    out,
                    "    #[serde(skip_serializing_if = \"Option::is_none\")]"
                )?;
            }
            writeln!(out, "    pub {}: {},", field_name, rust_type)?;
        }

        writeln!(out, "}}")?;
        writeln!(out)?;
    }

    // Generate enums
    for e in file.enums() {
        writeln!(out, "#[derive(Debug, Clone, Serialize, Deserialize)]")?;
        writeln!(out, "pub enum {} {{", e.name)?;

        for member in &e.members {
            let variant_name = member.name.to_case(Case::Pascal);
            if let Some(ref val) = member.value {
                if let crate::ast::Value::String(s) = val {
                    writeln!(out, "    #[serde(rename = \"{}\")]", s)?;
                }
            }
            writeln!(out, "    {},", variant_name)?;
        }

        writeln!(out, "}}")?;
        writeln!(out)?;
    }

    Ok(out)
}

/// Generate the handlers.rs file containing per-channel async handler traits.
fn generate_handlers(
    channel_handlers: &[(String, Vec<HandlerMethod>)],
) -> Result<String, CodegenError> {
    let mut out = String::new();

    writeln!(out, "//! Auto-generated protocol handler traits from TypeSpec.")?;
    writeln!(out, "//! DO NOT EDIT.")?;
    writeln!(out)?;
    writeln!(out, "use async_trait::async_trait;")?;
    writeln!(out, "use super::messages::*;")?;
    writeln!(out)?;

    // Error type
    writeln!(out, "#[derive(Debug, Clone)]")?;
    writeln!(out, "pub struct ProtocolError {{")?;
    writeln!(out, "    pub code: String,")?;
    writeln!(out, "    pub message: String,")?;
    writeln!(out, "}}")?;
    writeln!(out)?;
    writeln!(out, "impl ProtocolError {{")?;
    writeln!(
        out,
        "    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {{"
    )?;
    writeln!(
        out,
        "        Self {{ code: code.into(), message: message.into() }}"
    )?;
    writeln!(out, "    }}")?;
    writeln!(out, "}}")?;
    writeln!(out)?;

    for (channel_name, methods) in channel_handlers {
        let trait_name = format!("{}Handler", channel_name.to_case(Case::Pascal));

        writeln!(out, "#[async_trait]")?;
        writeln!(out, "pub trait {}: Send + Sync {{", trait_name)?;

        for method in methods {
            let method_prefix = match method.kind {
                MessageKind::Request | MessageKind::Command | MessageKind::Scatter => "handle",
                _ => "on",
            };
            let fn_name = format!("{}_{}", method_prefix, method.name);

            match &method.response_variant {
                Some(resp) => {
                    writeln!(
                        out,
                        "    async fn {}(&self, msg: {}) -> Result<{}, ProtocolError>;",
                        fn_name, method.request_variant, resp,
                    )?;
                }
                None if matches!(method.kind, MessageKind::Request | MessageKind::Command) => {
                    writeln!(
                        out,
                        "    async fn {}(&self, msg: {}) -> Result<(), ProtocolError>;",
                        fn_name, method.request_variant,
                    )?;
                }
                None => {
                    writeln!(
                        out,
                        "    async fn {}(&self, msg: {});",
                        fn_name, method.request_variant,
                    )?;
                }
            }
        }

        writeln!(out, "}}")?;
        writeln!(out)?;
    }

    Ok(out)
}

/// Generate lib.rs that re-exports all modules.
fn generate_lib() -> Result<String, CodegenError> {
    let mut out = String::new();
    writeln!(out, "//! Auto-generated protocol library from TypeSpec.")?;
    writeln!(out, "//! DO NOT EDIT.")?;
    writeln!(out)?;
    writeln!(out, "pub mod handlers;")?;
    writeln!(out, "pub mod messages;")?;
    writeln!(out, "pub mod types;")?;
    writeln!(out)?;
    writeln!(out, "pub use handlers::*;")?;
    writeln!(out, "pub use messages::*;")?;
    writeln!(out, "pub use types::*;")?;
    Ok(out)
}

// ── Helpers ──────────────────────────────────────────

/// Extract @channel("name") value from interface decorators.
pub(crate) fn get_channel_name(decorators: &[Decorator]) -> Option<String> {
    decorators
        .iter()
        .find(|d| d.name == "channel")
        .and_then(|d| d.get_string_arg(0).map(|s| s.to_string()))
}

/// Determine message kind from operation decorators.
pub(crate) fn get_message_kind(decorators: &[Decorator]) -> MessageKind {
    for d in decorators {
        match d.name.as_str() {
            "request" => return MessageKind::Request,
            "command" => return MessageKind::Command,
            "relay" => return MessageKind::Relay,
            "event" => return MessageKind::Event,
            "stream" => return MessageKind::Stream,
            "serverPush" => return MessageKind::ServerPush,
            "scatter" => return MessageKind::Scatter,
            _ => {}
        }
    }
    // Default to event if no recognized decorator
    MessageKind::Event
}

/// Derive a response variant name from channel prefix and operation name.
/// e.g., ("WebRtc", "StartSession") → "WebRtcSessionStarted"
///
/// Naming convention: attempts common patterns, falls back to {Op}Result.
fn derive_response_name(channel_prefix: &str, op_pascal: &str) -> String {
    // Common pattern: "Start" + "Session" → "SessionStarted"
    // We use the simple pattern: {Channel}{Op}Response for consistency
    format!("{}{}Response", channel_prefix, op_pascal)
}

/// Resolve operation parameters into enum variant fields.
fn resolve_variant_fields(
    params: &[OperationParam],
    models: &ModelMap<'_>,
) -> Vec<VariantField> {
    let mut fields = Vec::new();

    for param in params {
        if param.spread {
            // Spread: inline the referenced model's fields
            if let Some(name) = type_ref_name(&param.type_ref) {
                if let Some(model) = models.get(name.as_str()) {
                    for prop in &model.properties {
                        fields.push(VariantField {
                            name: prop.name.to_case(Case::Snake),
                            type_ref: prop.type_ref.clone(),
                            optional: prop.optional,
                        });
                    }
                }
            }
            continue;
        }

        fields.push(VariantField {
            name: param.name.to_case(Case::Snake),
            type_ref: param.type_ref.clone(),
            optional: param.optional,
        });
    }

    fields
}

/// Resolve response type (return type) into variant fields.
fn resolve_response_fields(
    return_type: &TypeRef,
    models: &ModelMap<'_>,
) -> Vec<VariantField> {
    match return_type {
        TypeRef::AnonymousModel(props) => props
            .iter()
            .map(|p| VariantField {
                name: p.name.to_case(Case::Snake),
                type_ref: p.type_ref.clone(),
                optional: p.optional,
            })
            .collect(),
        TypeRef::Named(name) => {
            if let Some(model) = models.get(name.as_str()) {
                model
                    .properties
                    .iter()
                    .map(|p| VariantField {
                        name: p.name.to_case(Case::Snake),
                        type_ref: p.type_ref.clone(),
                        optional: p.optional,
                    })
                    .collect()
            } else {
                Vec::new()
            }
        }
        TypeRef::Builtin(b) if b == "void" => Vec::new(),
        _ => Vec::new(),
    }
}

/// Extract type name from a TypeRef.
fn type_ref_name(type_ref: &TypeRef) -> Option<String> {
    match type_ref {
        TypeRef::Named(name) => Some(name.clone()),
        TypeRef::Qualified(parts) => parts.last().cloned(),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse;

    #[test]
    fn test_webrtc_channel_generates_enum() {
        let source = r#"
@channel("web-rtc")
interface WebRtc {
    @request
    startSession(session_id: string, device_id: string): {
        session_id: string;
        device_id: string;
    };

    @relay
    offer(session_id: string, sdp: string): void;

    @relay
    answer(session_id: string, sdp: string): void;

    @event
    sessionEnded(session_id: string, reason?: string): void;

    @event
    error(session_id: string, code: string, message: string): void;
}
"#;

        let file = parse(source).expect("parse failed");
        let models = build_model_map(&file);

        let (variants, channel_handlers) = collect_protocol_data(&file, &models);

        // Should produce 6 variants: StartSession, StartSessionResponse, Offer, Answer, SessionEnded, Error
        assert_eq!(variants.len(), 6, "expected 6 variants, got {}", variants.len());

        assert_eq!(variants[0].name, "WebRtcStartSession");
        assert_eq!(variants[0].fields.len(), 2);
        assert_eq!(variants[0].fields[0].name, "session_id");
        assert_eq!(variants[0].fields[1].name, "device_id");

        assert_eq!(variants[1].name, "WebRtcStartSessionResponse");
        assert_eq!(variants[1].fields.len(), 2);

        assert_eq!(variants[2].name, "WebRtcOffer");
        assert_eq!(variants[3].name, "WebRtcAnswer");
        assert_eq!(variants[4].name, "WebRtcSessionEnded");
        assert_eq!(variants[4].fields[1].name, "reason");
        assert!(variants[4].fields[1].optional);

        assert_eq!(variants[5].name, "WebRtcError");

        // Should have 1 channel with 5 handler methods
        assert_eq!(channel_handlers.len(), 1);
        assert_eq!(channel_handlers[0].0, "web-rtc");
        assert_eq!(channel_handlers[0].1.len(), 5);

        // Request method should have response
        assert_eq!(channel_handlers[0].1[0].name, "start_session");
        assert!(channel_handlers[0].1[0].response_variant.is_some());

        // Relay/event methods should have no response
        assert!(channel_handlers[0].1[1].response_variant.is_none());
    }

    #[test]
    fn test_generates_valid_messages_rs() {
        let source = r#"
@channel("web-rtc")
interface WebRtc {
    @relay
    offer(session_id: string, sdp: string): void;

    @event
    sessionEnded(session_id: string, reason?: string): void;
}
"#;

        let file = parse(source).expect("parse failed");
        let scalars = build_scalar_map(&file);
        let models = build_model_map(&file);
        let (variants, _) = collect_protocol_data(&file, &models);

        let config = RustProtocolConfig {
            tag: "type".to_string(),
            rename: "snake_case".to_string(),
            enum_name: "SignalingMessage".to_string(),
        };

        let output = generate_messages(&variants, &config, &scalars).unwrap();

        assert!(output.contains("pub enum SignalingMessage {"));
        assert!(output.contains("#[serde(tag = \"type\", rename_all = \"snake_case\")]"));
        assert!(output.contains("WebRtcOffer {"));
        assert!(output.contains("session_id: String,"));
        assert!(output.contains("sdp: String,"));
        assert!(output.contains("WebRtcSessionEnded {"));
        assert!(output.contains("reason: Option<String>,"));
        assert!(output.contains("#[serde(skip_serializing_if = \"Option::is_none\")]"));
    }

    #[test]
    fn test_generates_handler_traits() {
        let source = r#"
@channel("web-rtc")
interface WebRtc {
    @request
    startSession(session_id: string, device_id: string): {
        session_id: string;
        device_id: string;
    };

    @relay
    offer(session_id: string, sdp: string): void;

    @event
    sessionEnded(session_id: string, reason?: string): void;
}
"#;

        let file = parse(source).expect("parse failed");
        let models = build_model_map(&file);
        let (_, channel_handlers) = collect_protocol_data(&file, &models);

        let output = generate_handlers(&channel_handlers).unwrap();

        assert!(output.contains("pub trait WebRtcHandler: Send + Sync {"));
        // Request → handle_ prefix with Result return
        assert!(output.contains("async fn handle_start_session(&self, msg: WebRtcStartSession) -> Result<WebRtcStartSessionResponse, ProtocolError>;"));
        // Relay → on_ prefix, no return
        assert!(output.contains("async fn on_offer(&self, msg: WebRtcOffer);"));
        // Event → on_ prefix, no return
        assert!(output.contains("async fn on_session_ended(&self, msg: WebRtcSessionEnded);"));
    }

    #[test]
    fn test_model_types_generation() {
        let source = r#"
model WebRtcSessionInfo {
    session_id: string;
    client_id: string;
    state: string;
    ice_state?: string;
}

@channel("web-rtc")
interface WebRtc {
    @relay
    offer(session_id: string, sdp: string): void;
}
"#;

        let file = parse(source).expect("parse failed");
        let scalars = build_scalar_map(&file);
        let models = build_model_map(&file);

        let output = generate_types(&file, &scalars, &models).unwrap();

        assert!(output.contains("pub struct WebRtcSessionInfo {"));
        assert!(output.contains("pub session_id: String,"));
        assert!(output.contains("pub ice_state: Option<String>,"));
        assert!(output.contains("#[serde(skip_serializing_if = \"Option::is_none\")]"));
    }

    #[test]
    fn test_spread_params_in_operations() {
        let source = r#"
model HiveCapabilities {
    cpu_cores: int32;
    memory_gb: int32;
}

@channel("hive")
interface Hive {
    @request
    register(hive_id: string, signature: string, ...HiveCapabilities): {
        hive_id: string;
    };
}
"#;

        let file = parse(source).expect("parse failed");
        let models = build_model_map(&file);
        let (variants, _) = collect_protocol_data(&file, &models);

        // Request variant should have: hive_id, signature, cpu_cores, memory_gb
        assert_eq!(variants[0].name, "HiveRegister");
        assert_eq!(variants[0].fields.len(), 4);
        assert_eq!(variants[0].fields[0].name, "hive_id");
        assert_eq!(variants[0].fields[1].name, "signature");
        assert_eq!(variants[0].fields[2].name, "cpu_cores");
        assert_eq!(variants[0].fields[3].name, "memory_gb");
    }

    #[test]
    fn test_multiple_channels() {
        let source = r#"
@channel("auth")
interface Auth {
    @request
    authenticate(access_token: string): {
        user_id: string;
    };

    @event
    accessDenied(reason: string): void;
}

@channel("web-rtc")
interface WebRtc {
    @relay
    offer(session_id: string, sdp: string): void;
}
"#;

        let file = parse(source).expect("parse failed");
        let models = build_model_map(&file);
        let (variants, channel_handlers) = collect_protocol_data(&file, &models);

        // Auth: Authenticate, AuthenticateResponse, AccessDenied = 3
        // WebRTC: Offer = 1
        assert_eq!(variants.len(), 4);
        assert_eq!(variants[0].name, "AuthAuthenticate");
        assert_eq!(variants[1].name, "AuthAuthenticateResponse");
        assert_eq!(variants[2].name, "AuthAccessDenied");
        assert_eq!(variants[3].name, "WebRtcOffer");

        assert_eq!(channel_handlers.len(), 2);
        assert_eq!(channel_handlers[0].0, "auth");
        assert_eq!(channel_handlers[1].0, "web-rtc");
    }
}
