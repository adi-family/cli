//! Rust Code Generator

use crate::ast::*;
use crate::codegen::{
    build_model_map, build_scalar_map, resolve_properties, CodegenError, ModelMap, ScalarMap,
    Side,
};
use convert_case::{Case, Casing};
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::Write;
use std::fs;
use std::path::Path;

/// Axum path parameter syntax
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PathStyle {
    /// `:id` format (axum 0.7 and earlier)
    #[default]
    Colon,
    /// `{id}` format (axum 0.8+)
    Brace,
}

/// How to wrap the handler state type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StateWrapper {
    /// `State<Arc<S>>` — handler trait takes `&self`
    #[default]
    Arc,
    /// `State<S>` where S: Clone — handler trait takes `&self`
    Clone,
}

/// Configuration for Rust server code generation
#[derive(Debug, Clone, Default)]
pub struct RustServerConfig {
    pub path_style: PathStyle,
    pub state_wrapper: StateWrapper,
    /// Optional type extracted from requests and passed as first arg to all trait methods.
    /// Must implement `axum::extract::FromRequestParts<S>` on the consumer side.
    /// Example: "AuthUser" generates `ctx: AuthUser` on trait methods and axum extraction.
    pub request_context: Option<String>,
}

/// Configuration for AdiService code generation
#[derive(Debug, Clone)]
pub struct RustAdiServiceConfig {
    /// Crate name for the types package (e.g., "tasks-types")
    pub types_crate: String,
    /// Service ID for AdiService::service_id() (e.g., "tasks")
    pub service_id: String,
    /// Human-readable service name (e.g., "Task Management")
    pub service_name: String,
    /// Service version
    pub service_version: String,
}

/// Context for tracking inline enums that need to be generated
struct CodegenContext {
    /// Map of enum name -> (variants as string literals)
    inline_enums: RefCell<HashMap<String, Vec<String>>>,
}

impl CodegenContext {
    fn new() -> Self {
        Self {
            inline_enums: RefCell::new(HashMap::new()),
        }
    }

    /// Register an inline enum and return its name
    fn register_inline_enum(
        &self,
        model_name: &str,
        prop_name: &str,
        variants: &[String],
    ) -> String {
        let enum_name = format!("{}{}", model_name, prop_name.to_case(Case::Pascal));
        self.inline_enums
            .borrow_mut()
            .insert(enum_name.clone(), variants.to_vec());
        enum_name
    }

    /// Get all registered inline enums
    fn get_inline_enums(&self) -> HashMap<String, Vec<String>> {
        self.inline_enums.borrow().clone()
    }
}

pub fn generate(
    file: &TypeSpecFile,
    output_dir: &Path,
    package_name: &str,
    side: Side,
) -> Result<Vec<String>, CodegenError> {
    generate_with_config(
        file,
        output_dir,
        package_name,
        side,
        &RustServerConfig::default(),
        None,
        None,
    )
}

pub fn generate_with_config(
    file: &TypeSpecFile,
    output_dir: &Path,
    package_name: &str,
    side: Side,
    server_config: &RustServerConfig,
    types_crate: Option<&str>,
    adi_config: Option<&RustAdiServiceConfig>,
) -> Result<Vec<String>, CodegenError> {
    let scalars = build_scalar_map(file);
    let models = build_model_map(file);

    match side {
        Side::Types => generate_types_crate(file, output_dir, package_name, &scalars, &models),
        Side::AdiService => {
            let config = adi_config.ok_or_else(|| {
                CodegenError::Generation(
                    "AdiService generation requires --types-crate, --service-id, --service-name, --service-version".to_string(),
                )
            })?;
            generate_adi_service_crate(file, output_dir, package_name, &scalars, &models, config)
        }
        // Protocol side is handled by codegen::protocol module, not here
        Side::Protocol => Ok(Vec::new()),
        _ => {
            let mut generated = Vec::new();

            let src_dir = output_dir.join("src");
            fs::create_dir_all(&src_dir)?;

            // Generate Cargo.toml
            let cargo_content = generate_cargo_toml(package_name, side)?;
            let cargo_path = output_dir.join("Cargo.toml");
            fs::write(&cargo_path, cargo_content)?;
            generated.push(cargo_path.display().to_string());

            // Generate lib.rs
            let lib_content = generate_lib(side)?;
            let lib_path = src_dir.join("lib.rs");
            fs::write(&lib_path, lib_content)?;
            generated.push(lib_path.display().to_string());

            // Generate models
            let models_content = generate_models(file, &scalars, &models)?;
            let models_path = src_dir.join("models.rs");
            fs::write(&models_path, models_content)?;
            generated.push(models_path.display().to_string());

            // Generate enums
            let enums_content = generate_enums(file)?;
            let enums_path = src_dir.join("enums.rs");
            fs::write(&enums_path, enums_content)?;
            generated.push(enums_path.display().to_string());

            // Generate client
            if matches!(side, Side::Client | Side::Both) {
                let client_content = generate_client(file, &scalars)?;
                let client_path = src_dir.join("client.rs");
                fs::write(&client_path, client_content)?;
                generated.push(client_path.display().to_string());
            }

            // Generate server
            if matches!(side, Side::Server | Side::Both) {
                let server_content =
                    generate_server(file, &scalars, &models, server_config, types_crate)?;
                let server_path = src_dir.join("server.rs");
                fs::write(&server_path, server_content)?;
                generated.push(server_path.display().to_string());
            }

            Ok(generated)
        }
    }
}

fn generate_cargo_toml(package_name: &str, side: Side) -> Result<String, CodegenError> {
    let mut out = String::new();

    writeln!(out, "[package]")?;
    writeln!(out, r#"name = "{}""#, package_name)?;
    writeln!(out, r#"version = "0.1.0""#)?;
    writeln!(out, r#"edition = "2021""#)?;
    writeln!(out)?;
    writeln!(out, "[dependencies]")?;
    writeln!(
        out,
        r#"serde = {{ version = "1.0", features = ["derive"] }}"#
    )?;
    writeln!(out, r#"serde_json = "1.0""#)?;
    writeln!(
        out,
        r#"chrono = {{ version = "0.4", features = ["serde"] }}"#
    )?;
    writeln!(
        out,
        r#"uuid = {{ version = "1.0", features = ["serde", "v4"] }}"#
    )?;
    writeln!(out, r#"thiserror = "2""#)?;

    if matches!(side, Side::Client | Side::Both) {
        writeln!(
            out,
            r#"reqwest = {{ version = "0.12", features = ["json"] }}"#
        )?;
    }

    if matches!(side, Side::Server | Side::Both) {
        writeln!(out, r#"axum = "0.7""#)?;
        writeln!(out, r#"async-trait = "0.1""#)?;
        writeln!(out, r#"tokio = {{ version = "1", features = ["full"] }}"#)?;
    }

    Ok(out)
}

fn generate_lib(side: Side) -> Result<String, CodegenError> {
    let mut out = String::new();

    writeln!(out, "//! Auto-generated from TypeSpec.")?;
    writeln!(out, "//! DO NOT EDIT.")?;
    writeln!(out)?;
    writeln!(out, "pub mod models;")?;
    writeln!(out, "pub mod enums;")?;

    if matches!(side, Side::Client | Side::Both) {
        writeln!(out, "pub mod client;")?;
    }

    if matches!(side, Side::Server | Side::Both) {
        writeln!(out, "pub mod server;")?;
    }

    Ok(out)
}

fn generate_models(
    file: &TypeSpecFile,
    scalars: &ScalarMap,
    models: &ModelMap<'_>,
) -> Result<String, CodegenError> {
    let mut out = String::new();
    let ctx = CodegenContext::new();

    writeln!(out, "//! Auto-generated models from TypeSpec.")?;
    writeln!(out, "//! DO NOT EDIT.")?;
    writeln!(out)?;
    writeln!(out, "#![allow(unused_imports)]")?;
    writeln!(out)?;
    writeln!(out, "use crate::enums::*;")?;
    writeln!(out, "use chrono::{{DateTime, Utc}};")?;
    writeln!(out, "use serde::{{Deserialize, Serialize}};")?;
    writeln!(out, "use std::collections::HashMap;")?;
    writeln!(out, "use uuid::Uuid;")?;
    writeln!(out)?;

    // First pass: collect all structs and inline enums
    let mut struct_defs = String::new();

    for model in file.models() {
        // Skip generic models - they need special handling
        if !model.type_params.is_empty() {
            write_generic_model(&mut struct_defs, model, scalars, models)?;
            continue;
        }

        writeln!(struct_defs)?;
        if let Some(desc) = get_description(&model.decorators) {
            writeln!(struct_defs, "/// {}", desc)?;
        }
        writeln!(
            struct_defs,
            "#[derive(Debug, Clone, Serialize, Deserialize)]"
        )?;
        writeln!(struct_defs, "#[serde(rename_all = \"camelCase\")]")?;
        writeln!(struct_defs, "pub struct {} {{", model.name)?;

        // Resolve spread references and get all properties
        let all_properties = resolve_properties(model, models);

        for prop in all_properties {
            let rust_type = type_to_rust_with_context(
                &prop.type_ref,
                prop.optional,
                scalars,
                &ctx,
                &model.name,
                &prop.name,
            );
            let name = prop.name.to_case(Case::Snake);

            if prop.optional {
                writeln!(
                    struct_defs,
                    "    #[serde(skip_serializing_if = \"Option::is_none\")]"
                )?;
            }

            // Handle name conflicts with Rust keywords
            let field_name = if is_rust_keyword(&name) {
                format!("r#{}", name)
            } else {
                name
            };

            writeln!(struct_defs, "    pub {}: {},", field_name, rust_type)?;
        }

        writeln!(struct_defs, "}}")?;
    }

    // Generate inline enums first
    for (enum_name, variants) in ctx.get_inline_enums() {
        writeln!(out)?;
        writeln!(
            out,
            "#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]"
        )?;
        writeln!(out, "pub enum {} {{", enum_name)?;
        for variant in &variants {
            let variant_name = variant.to_case(Case::Pascal);
            writeln!(out, r#"    #[serde(rename = "{}")]"#, variant)?;
            writeln!(out, "    {},", variant_name)?;
        }
        writeln!(out, "}}")?;
    }

    // Then write struct definitions
    out.push_str(&struct_defs);

    Ok(out)
}

fn write_generic_model(
    out: &mut String,
    model: &Model,
    scalars: &ScalarMap,
    models: &ModelMap<'_>,
) -> Result<(), CodegenError> {
    let ctx = CodegenContext::new();

    writeln!(out)?;
    if let Some(desc) = get_description(&model.decorators) {
        writeln!(out, "/// {}", desc)?;
    }
    writeln!(out, "#[derive(Debug, Clone, Serialize, Deserialize)]")?;
    writeln!(out, "#[serde(rename_all = \"camelCase\")]")?;

    // Write struct with type parameters
    let type_params = model.type_params.join(", ");
    writeln!(out, "pub struct {}<{}> {{", model.name, type_params)?;

    let all_properties = resolve_properties(model, models);

    for prop in all_properties {
        let rust_type = type_to_rust_with_context(
            &prop.type_ref,
            prop.optional,
            scalars,
            &ctx,
            &model.name,
            &prop.name,
        );
        let name = prop.name.to_case(Case::Snake);

        if prop.optional {
            writeln!(
                out,
                "    #[serde(skip_serializing_if = \"Option::is_none\")]"
            )?;
        }

        let field_name = if is_rust_keyword(&name) {
            format!("r#{}", name)
        } else {
            name
        };

        writeln!(out, "    pub {}: {},", field_name, rust_type)?;
    }

    writeln!(out, "}}")?;
    Ok(())
}

fn generate_enums(file: &TypeSpecFile) -> Result<String, CodegenError> {
    let mut out = String::new();

    writeln!(out, "//! Auto-generated enums from TypeSpec.")?;
    writeln!(out, "//! DO NOT EDIT.")?;
    writeln!(out)?;
    writeln!(out, "use serde::{{Deserialize, Serialize}};")?;
    writeln!(out)?;

    for enum_def in file.enums() {
        writeln!(out)?;
        writeln!(
            out,
            "#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]"
        )?;
        writeln!(out, "pub enum {} {{", enum_def.name)?;

        for member in &enum_def.members {
            // Get the serialization value - either explicit or snake_case of name
            let value = member
                .value
                .as_ref()
                .map(|v| match v {
                    Value::String(s) => s.clone(),
                    _ => member.name.to_case(Case::Snake),
                })
                .unwrap_or_else(|| member.name.to_case(Case::Snake));

            let variant = member.name.to_case(Case::Pascal);

            // Always add rename attribute for explicit serialization
            writeln!(out, r#"    #[serde(rename = "{}")]"#, value)?;
            writeln!(out, "    {},", variant)?;
        }

        writeln!(out, "}}")?;
    }

    Ok(out)
}

fn generate_client(file: &TypeSpecFile, scalars: &ScalarMap) -> Result<String, CodegenError> {
    let mut out = String::new();

    writeln!(out, "//! Auto-generated API client from TypeSpec.")?;
    writeln!(out, "//! DO NOT EDIT.")?;
    writeln!(out)?;
    writeln!(out, "#![allow(unused_imports)]")?;
    writeln!(out)?;
    writeln!(out, "use crate::models::*;")?;
    writeln!(out, "use crate::enums::*;")?;
    writeln!(out, "use reqwest::{{Client, Method}};")?;
    writeln!(out, "use serde::{{de::DeserializeOwned, Serialize}};")?;
    writeln!(out, "use thiserror::Error;")?;
    writeln!(out, "use uuid::Uuid;")?;
    writeln!(out)?;

    // Error type
    writeln!(
        out,
        r#"
#[derive(Debug, Error)]
pub enum ApiError {{
    #[error("HTTP error: {{0}}")]
    Http(#[from] reqwest::Error),

    #[error("API error: {{status}} - {{message}}")]
    Api {{ status: u16, code: String, message: String }},
}}
"#
    )?;

    // Base client
    writeln!(
        out,
        r#"
pub struct BaseClient {{
    client: Client,
    base_url: String,
    access_token: Option<String>,
}}

impl BaseClient {{
    pub fn new(base_url: impl Into<String>) -> Self {{
        Self {{
            client: Client::new(),
            base_url: base_url.into().trim_end_matches('/').to_string(),
            access_token: None,
        }}
    }}

    pub fn with_token(mut self, token: impl Into<String>) -> Self {{
        self.access_token = Some(token.into());
        self
    }}

    pub fn set_token(&mut self, token: impl Into<String>) {{
        self.access_token = Some(token.into());
    }}

    async fn request<T, B>(
        &self,
        method: Method,
        path: &str,
        body: Option<&B>,
    ) -> Result<T, ApiError>
    where
        T: DeserializeOwned,
        B: Serialize,
    {{
        let url = format!("{{}}{{}}", self.base_url, path);
        let mut req = self.client.request(method, &url);

        if let Some(token) = &self.access_token {{
            req = req.header("Authorization", format!("Bearer {{}}", token));
        }}

        if let Some(body) = body {{
            req = req.json(body);
        }}

        let resp = req.send().await?;
        let status = resp.status();

        if !status.is_success() {{
            let err: serde_json::Value = resp.json().await.unwrap_or_default();
            return Err(ApiError::Api {{
                status: status.as_u16(),
                code: err["code"].as_str().unwrap_or("ERROR").to_string(),
                message: err["message"].as_str().unwrap_or("").to_string(),
            }});
        }}

        if status == reqwest::StatusCode::NO_CONTENT {{
            return Ok(serde_json::from_value(serde_json::Value::Null).unwrap());
        }}

        Ok(resp.json().await?)
    }}
}}
"#
    )?;

    // Service clients
    for iface in file.interfaces() {
        let base_path = get_route(&iface.decorators).unwrap_or_default();
        let struct_name = format!("{}Client", iface.name);

        writeln!(out)?;
        writeln!(out, "pub struct {}<'a> {{", struct_name)?;
        writeln!(out, "    client: &'a BaseClient,")?;
        writeln!(out, "}}")?;
        writeln!(out)?;
        writeln!(out, "impl<'a> {}<'a> {{", struct_name)?;
        writeln!(out, "    pub fn new(client: &'a BaseClient) -> Self {{")?;
        writeln!(out, "        Self {{ client }}")?;
        writeln!(out, "    }}")?;

        for op in &iface.operations {
            let method = get_http_method(&op.decorators);
            let op_path = get_route(&op.decorators).unwrap_or_default();
            let full_path = format!("{}{}", base_path, op_path);
            let fn_name = op.name.to_case(Case::Snake);

            writeln!(out)?;
            write!(out, "    pub async fn {}(&self", fn_name)?;

            // Parameters
            for param in &op.params {
                // Skip spread params without explicit names
                if param.spread && param.name.is_empty() {
                    continue;
                }
                let name = param.name.to_case(Case::Snake);
                if has_decorator(&param.decorators, "path") {
                    write!(out, ", {}: &str", name)?;
                } else if has_decorator(&param.decorators, "body") {
                    let ty = type_to_rust(&param.type_ref, false, scalars);
                    write!(out, ", body: &{}", ty)?;
                } else if has_decorator(&param.decorators, "query") {
                    let ty = type_to_rust(&param.type_ref, param.optional, scalars);
                    write!(out, ", {}: {}", name, ty)?;
                }
            }

            let return_type = op
                .return_type
                .as_ref()
                .map(|t| type_to_rust(t, false, scalars))
                .unwrap_or_else(|| "()".to_string());

            writeln!(out, ") -> Result<{}, ApiError> {{", return_type)?;

            // Build path
            let mut path_expr = format!(r#"let path = format!("{}"#, full_path);
            for param in &op.params {
                if has_decorator(&param.decorators, "path") {
                    path_expr = path_expr.replace(&format!("{{{}}}", param.name), "{}");
                }
            }
            let path_args: Vec<_> = op
                .params
                .iter()
                .filter(|p| has_decorator(&p.decorators, "path"))
                .map(|p| p.name.to_case(Case::Snake))
                .collect();

            if path_args.is_empty() {
                writeln!(out, r#"        let path = "{}";"#, full_path)?;
            } else {
                writeln!(out, "{}\"", path_expr)?;
                for arg in &path_args {
                    write!(out, ", {}", arg)?;
                }
                writeln!(out, ");")?;
            }

            // Make request
            let has_body = op
                .params
                .iter()
                .any(|p| has_decorator(&p.decorators, "body"));

            writeln!(
                out,
                "        self.client.request(Method::{}, &path, {}).await",
                method,
                if has_body {
                    "Some(body)"
                } else {
                    "None::<&()>"
                }
            )?;

            writeln!(out, "    }}")?;
        }

        writeln!(out, "}}")?;
    }

    Ok(out)
}

/// Extracted response information from a TypeSpec return type
struct ResponseInfo {
    status_code: u16,
    body_type: Option<String>,
    /// True when body type is `bytes` — handler returns raw Response (streaming)
    is_streaming: bool,
}

/// Extract response status code and body type from a TypeSpec return type.
/// Handles patterns like `{ @statusCode: 200; @body body: T } | ApiError`
fn extract_response_info(type_ref: &TypeRef, scalars: &ScalarMap) -> ResponseInfo {
    let mut info = extract_response_info_inner(type_ref, scalars);
    // Detect streaming: when body is bytes, handler returns raw axum::response::Response
    if info.body_type.as_deref() == Some("Vec<u8>") {
        info.is_streaming = true;
        info.body_type = Some("axum::response::Response".to_string());
    }
    info
}

fn extract_response_info_inner(type_ref: &TypeRef, scalars: &ScalarMap) -> ResponseInfo {
    match type_ref {
        TypeRef::Union(variants) => {
            for variant in variants {
                if let TypeRef::AnonymousModel(props) = variant {
                    let mut status_code = 200u16;
                    let mut body_type = None;

                    for prop in props {
                        if has_decorator(&prop.decorators, "statusCode") {
                            if let TypeRef::IntLiteral(code) = &prop.type_ref {
                                status_code = *code as u16;
                            }
                        }
                        if has_decorator(&prop.decorators, "body") {
                            body_type = Some(type_to_rust(&prop.type_ref, false, scalars));
                        }
                    }

                    if status_code == 204 {
                        return ResponseInfo {
                            status_code,
                            body_type: None,
                            is_streaming: false,
                        };
                    }
                    if body_type.is_some() {
                        return ResponseInfo {
                            status_code,
                            body_type,
                            is_streaming: false,
                        };
                    }
                }
            }
            ResponseInfo {
                status_code: 200,
                body_type: None,
                is_streaming: false,
            }
        }
        TypeRef::AnonymousModel(props) => {
            let mut status_code = 200u16;
            let mut body_type = None;

            for prop in props {
                if has_decorator(&prop.decorators, "statusCode") {
                    if let TypeRef::IntLiteral(code) = &prop.type_ref {
                        status_code = *code as u16;
                    }
                }
                if has_decorator(&prop.decorators, "body") {
                    body_type = Some(type_to_rust(&prop.type_ref, false, scalars));
                }
            }

            ResponseInfo {
                status_code,
                body_type,
                is_streaming: false,
            }
        }
        _ => ResponseInfo {
            status_code: 200,
            body_type: Some(type_to_rust(type_ref, false, scalars)),
            is_streaming: false,
        },
    }
}

/// Resolved parameter for a server operation
struct ResolvedParam {
    name: String,
    rust_type: String,
    kind: ParamKind,
}

enum ParamKind {
    Path,
    Query,
    Body,
}

/// Resolve all parameters for an operation, expanding spread params
fn resolve_op_params(
    op: &Operation,
    scalars: &ScalarMap,
    models: &ModelMap<'_>,
) -> Vec<ResolvedParam> {
    let mut params = Vec::new();

    for param in &op.params {
        if param.spread && param.name.is_empty() {
            // Anonymous spread: expand model properties as query params
            if let Some(name) = param.type_ref.base_name() {
                if let Some(model) = models.get(name) {
                    let props = resolve_properties(model, models);
                    for prop in props {
                        // Properties with @query decorator or inherited from spread models are query params
                        let kind = if has_decorator(&prop.decorators, "path") {
                            ParamKind::Path
                        } else if has_decorator(&prop.decorators, "body") {
                            ParamKind::Body
                        } else {
                            ParamKind::Query
                        };
                        params.push(ResolvedParam {
                            name: prop.name.clone(),
                            rust_type: type_to_rust(&prop.type_ref, prop.optional, scalars),
                            kind,
                        });
                    }
                }
            }
        } else if has_decorator(&param.decorators, "path") {
            params.push(ResolvedParam {
                name: param.name.clone(),
                rust_type: type_to_rust(&param.type_ref, false, scalars),
                kind: ParamKind::Path,
            });
        } else if has_decorator(&param.decorators, "body") {
            params.push(ResolvedParam {
                name: param.name.clone(),
                rust_type: type_to_rust(&param.type_ref, false, scalars),
                kind: ParamKind::Body,
            });
        } else if has_decorator(&param.decorators, "query") {
            params.push(ResolvedParam {
                name: param.name.clone(),
                rust_type: type_to_rust(&param.type_ref, param.optional, scalars),
                kind: ParamKind::Query,
            });
        }
    }

    params
}

fn generate_server(
    file: &TypeSpecFile,
    scalars: &ScalarMap,
    models: &ModelMap<'_>,
    config: &RustServerConfig,
    types_crate: Option<&str>,
) -> Result<String, CodegenError> {
    let use_arc = matches!(config.state_wrapper, StateWrapper::Arc);
    let use_brace_path = matches!(config.path_style, PathStyle::Brace);
    let ctx_type = config.request_context.as_deref();

    let mut out = String::new();

    writeln!(out, "//! Auto-generated server handlers from TypeSpec.")?;
    writeln!(out, "//! DO NOT EDIT.")?;
    writeln!(out, "//!")?;
    writeln!(
        out,
        "//! Implement the handler traits and use the generated router."
    )?;
    writeln!(out)?;
    writeln!(out, "#![allow(unused_imports)]")?;
    writeln!(out)?;
    if let Some(tc) = types_crate {
        let crate_ident = tc.replace('-', "_");
        writeln!(out, "use {}::models::*;", crate_ident)?;
        writeln!(out, "use {}::enums::*;", crate_ident)?;
    } else {
        writeln!(out, "use super::models::*;")?;
        writeln!(out, "use super::enums::*;")?;
    }
    writeln!(out, "use async_trait::async_trait;")?;
    writeln!(out, "use axum::extract::{{Path, Query, State}};")?;
    writeln!(out, "use axum::http::StatusCode;")?;
    writeln!(out, "use axum::routing::{{delete, get, patch, post, put}};")?;
    writeln!(out, "use axum::{{Json, Router}};")?;
    writeln!(out, "use serde::Deserialize;")?;
    if use_arc {
        writeln!(out, "use std::sync::Arc;")?;
    }
    writeln!(out, "use uuid::Uuid;")?;
    writeln!(out)?;

    // Error type
    writeln!(
        out,
        r#"
#[derive(Debug, serde::Serialize)]
pub struct ApiError {{
    pub status: u16,
    pub code: String,
    pub message: String,
}}

impl axum::response::IntoResponse for ApiError {{
    fn into_response(self) -> axum::response::Response {{
        let status = StatusCode::from_u16(self.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        (status, Json(self)).into_response()
    }}
}}
"#
    )?;

    // Collect all interfaces for the combined router
    let mut interface_names: Vec<String> = Vec::new();

    for iface in file.interfaces() {
        let trait_name = format!("{}Handler", iface.name);
        let iface_snake = iface.name.to_case(Case::Snake);
        interface_names.push(iface.name.clone());

        // Collect query structs needed for this interface
        let mut query_structs: Vec<(String, Vec<(String, String)>)> = Vec::new();

        // --- Handler trait ---
        writeln!(out)?;
        writeln!(out, "#[async_trait]")?;
        writeln!(out, "pub trait {}: Send + Sync + 'static {{", trait_name)?;

        for op in &iface.operations {
            let fn_name = op.name.to_case(Case::Snake);
            let resolved = resolve_op_params(op, scalars, models);
            let resp = op
                .return_type
                .as_ref()
                .map(|t| extract_response_info(t, scalars))
                .unwrap_or(ResponseInfo {
                    status_code: 200,
                    body_type: None,
                    is_streaming: false,
                });

            write!(out, "    async fn {}(&self", fn_name)?;

            // Request context param (e.g. AuthUser)
            if let Some(ctx) = ctx_type {
                write!(out, ", ctx: {}", ctx)?;
            }

            // Path params
            for p in resolved.iter().filter(|p| matches!(p.kind, ParamKind::Path)) {
                let name = p.name.to_case(Case::Snake);
                write!(out, ", {}: {}", name, p.rust_type)?;
            }

            // Query params - if multiple, use a query struct
            let query_params: Vec<_> = resolved
                .iter()
                .filter(|p| matches!(p.kind, ParamKind::Query))
                .collect();
            if !query_params.is_empty() {
                let struct_name = format!(
                    "{}{}Query",
                    iface.name,
                    op.name.to_case(Case::Pascal)
                );
                write!(out, ", query: {}", struct_name)?;

                // Collect fields for later struct generation
                let fields: Vec<(String, String)> = query_params
                    .iter()
                    .map(|p| (p.name.clone(), p.rust_type.clone()))
                    .collect();
                query_structs.push((struct_name, fields));
            }

            // Body param
            for p in resolved.iter().filter(|p| matches!(p.kind, ParamKind::Body)) {
                write!(out, ", body: {}", p.rust_type)?;
            }

            // Return type
            let return_type = match &resp.body_type {
                Some(ty) => ty.clone(),
                None => "()".to_string(),
            };

            writeln!(out, ") -> Result<{}, ApiError>;", return_type)?;
        }

        writeln!(out, "}}")?;

        // --- Query param structs ---
        for (struct_name, fields) in &query_structs {
            writeln!(out)?;
            writeln!(out, "#[derive(Debug, Deserialize)]")?;
            writeln!(out, "#[serde(rename_all = \"camelCase\")]")?;
            writeln!(out, "pub struct {} {{", struct_name)?;
            for (name, ty) in fields {
                let field_name = name.to_case(Case::Snake);
                let field_name = if is_rust_keyword(&field_name) {
                    format!("r#{}", field_name)
                } else {
                    field_name
                };
                writeln!(out, "    pub {}: {},", field_name, ty)?;
            }
            writeln!(out, "}}")?;
        }

        // --- Handler wrapper functions ---
        let base_path = get_route(&iface.decorators).unwrap_or_default();

        for op in &iface.operations {
            let fn_name = op.name.to_case(Case::Snake);
            let handler_fn = format!("{}_{}", iface_snake, fn_name);
            let resolved = resolve_op_params(op, scalars, models);
            let resp = op
                .return_type
                .as_ref()
                .map(|t| extract_response_info(t, scalars))
                .unwrap_or(ResponseInfo {
                    status_code: 200,
                    body_type: None,
                    is_streaming: false,
                });

            let path_params: Vec<_> = resolved
                .iter()
                .filter(|p| matches!(p.kind, ParamKind::Path))
                .collect();
            let query_params: Vec<_> = resolved
                .iter()
                .filter(|p| matches!(p.kind, ParamKind::Query))
                .collect();
            let body_param = resolved
                .iter()
                .find(|p| matches!(p.kind, ParamKind::Body));
            let query_struct_name = if !query_params.is_empty() {
                Some(format!(
                    "{}{}Query",
                    iface.name,
                    op.name.to_case(Case::Pascal)
                ))
            } else {
                None
            };

            // Determine return type of the handler wrapper
            let has_body_response = resp.body_type.is_some();
            let is_no_content = resp.status_code == 204;
            let is_created = resp.status_code == 201;

            writeln!(out)?;
            write!(out, "async fn {}<S: {}>(", handler_fn, trait_name)?;

            // State extractor
            writeln!(out)?;
            if use_arc {
                writeln!(out, "    State(state): State<Arc<S>>,")?;
            } else {
                writeln!(out, "    State(state): State<S>,")?;
            }

            // Request context extractor (e.g. AuthUser)
            if let Some(ctx) = ctx_type {
                writeln!(out, "    ctx: {},", ctx)?;
            }

            // Path extractor
            if path_params.len() == 1 {
                let p = &path_params[0];
                writeln!(
                    out,
                    "    Path({}): Path<{}>,",
                    p.name.to_case(Case::Snake),
                    p.rust_type
                )?;
            } else if path_params.len() > 1 {
                let types: Vec<_> = path_params.iter().map(|p| p.rust_type.as_str()).collect();
                let names: Vec<_> = path_params
                    .iter()
                    .map(|p| p.name.to_case(Case::Snake))
                    .collect();
                writeln!(
                    out,
                    "    Path(({})):  Path<({})>,",
                    names.join(", "),
                    types.join(", ")
                )?;
            }

            // Query extractor
            if let Some(qs) = &query_struct_name {
                writeln!(out, "    Query(query): Query<{}>,", qs)?;
            }

            // Body extractor
            let is_bytes_body = body_param.as_ref().map_or(false, |bp| bp.rust_type == "Vec<u8>");
            if let Some(bp) = body_param {
                if is_bytes_body {
                    writeln!(out, "    body: axum::body::Bytes,")?;
                } else {
                    writeln!(out, "    Json(body): Json<{}>,", bp.rust_type)?;
                }
            }

            // Return type
            let is_streaming = resp.is_streaming;
            if is_streaming {
                writeln!(out, ") -> Result<axum::response::Response, ApiError> {{")?;
            } else if is_no_content {
                writeln!(out, ") -> Result<StatusCode, ApiError> {{")?;
            } else if is_created {
                writeln!(
                    out,
                    ") -> Result<(StatusCode, Json<{}>), ApiError> {{",
                    resp.body_type.as_deref().unwrap_or("()")
                )?;
            } else if has_body_response {
                writeln!(
                    out,
                    ") -> Result<Json<{}>, ApiError> {{",
                    resp.body_type.as_deref().unwrap_or("()")
                )?;
            } else {
                writeln!(out, ") -> Result<StatusCode, ApiError> {{")?;
            }

            // Build trait method call
            write!(out, "    let result = state.{}(", fn_name)?;

            let mut first = true;
            // Context arg
            if ctx_type.is_some() {
                write!(out, "ctx")?;
                first = false;
            }
            // Path args
            for p in &path_params {
                if !first {
                    write!(out, ", ")?;
                }
                write!(out, "{}", p.name.to_case(Case::Snake))?;
                first = false;
            }
            // Query args
            if query_struct_name.is_some() {
                if !first {
                    write!(out, ", ")?;
                }
                write!(out, "query")?;
                first = false;
            }
            // Body arg
            if body_param.is_some() {
                if !first {
                    write!(out, ", ")?;
                }
                if is_bytes_body {
                    write!(out, "body.to_vec()")?;
                } else {
                    write!(out, "body")?;
                }
            }

            writeln!(out, ").await?;")?;

            // Return
            if is_streaming {
                writeln!(out, "    Ok(result)")?;
            } else if is_no_content {
                writeln!(out, "    Ok(StatusCode::NO_CONTENT)")?;
            } else if is_created {
                writeln!(out, "    Ok((StatusCode::CREATED, Json(result)))")?;
            } else if has_body_response {
                writeln!(out, "    Ok(Json(result))")?;
            } else {
                writeln!(out, "    Ok(StatusCode::OK)")?;
            }

            writeln!(out, "}}")?;
        }

        // --- Per-interface router factory ---
        writeln!(out)?;
        let state_type = if use_arc { "Arc<S>" } else { "S" };
        let clone_bound = if !use_arc { " + Clone + 'static" } else { "" };
        writeln!(
            out,
            "pub fn {}_routes<S: {}{}>() -> Router<{}> {{",
            iface_snake, trait_name, clone_bound, state_type
        )?;

        // Group operations by path to use method chaining
        let mut path_ops: Vec<(String, Vec<(&Operation, &str)>)> = Vec::new();

        for op in &iface.operations {
            let op_path = get_route(&op.decorators).unwrap_or_default();
            let full_path = format!("{}{}", base_path, op_path);
            // Convert TypeSpec path params based on configured style
            let axum_path = if use_brace_path {
                // axum 0.8+: keep {id} as-is (TypeSpec native format)
                full_path.clone()
            } else {
                // axum 0.7: convert {id} to :id
                full_path
                    .split('/')
                    .map(|seg| {
                        if seg.starts_with('{') && seg.ends_with('}') {
                            format!(":{}", &seg[1..seg.len() - 1])
                        } else {
                            seg.to_string()
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("/")
            };
            let method = get_http_method(&op.decorators);

            if let Some(entry) = path_ops.iter_mut().find(|(p, _)| *p == axum_path) {
                entry.1.push((op, method));
            } else {
                path_ops.push((axum_path, vec![(op, method)]));
            }
        }

        writeln!(out, "    Router::new()")?;
        for (path, ops) in &path_ops {
            let method_chain: Vec<String> = ops
                .iter()
                .map(|(op, method)| {
                    let handler_fn = format!(
                        "{}_{}", iface_snake, op.name.to_case(Case::Snake)
                    );
                    let method_fn = match *method {
                        "GET" => "get",
                        "POST" => "post",
                        "PUT" => "put",
                        "PATCH" => "patch",
                        "DELETE" => "delete",
                        _ => "get",
                    };
                    format!("{}({}::<S>)", method_fn, handler_fn)
                })
                .collect();
            writeln!(
                out,
                "        .route(\"{}\", {})",
                path,
                method_chain.join(".")
            )?;
        }
        writeln!(out, "}}")?;
    }

    // --- Combined router factory ---
    let state_type = if use_arc { "Arc<S>" } else { "S" };
    if interface_names.len() > 1 {
        writeln!(out)?;
        let trait_bounds: Vec<String> = interface_names
            .iter()
            .map(|n| format!("{}Handler", n))
            .collect();
        let clone_bound = if !use_arc { " + Clone + 'static" } else { "" };
        writeln!(
            out,
            "pub fn create_router<S: {}{}>() -> Router<{}> {{",
            trait_bounds.join(" + "), clone_bound, state_type
        )?;
        writeln!(out, "    Router::new()")?;
        for name in &interface_names {
            writeln!(
                out,
                "        .merge({}_routes())",
                name.to_case(Case::Snake)
            )?;
        }
        writeln!(out, "}}")?;
    } else if interface_names.len() == 1 {
        writeln!(out)?;
        let name = &interface_names[0];
        let clone_bound = if !use_arc { " + Clone + 'static" } else { "" };
        writeln!(
            out,
            "pub fn create_router<S: {}Handler{}>() -> Router<{}> {{",
            name, clone_bound, state_type
        )?;
        writeln!(
            out,
            "    {}_routes()",
            name.to_case(Case::Snake)
        )?;
        writeln!(out, "}}")?;
    }

    Ok(out)
}

// ========== Types-only crate generation ==========

fn generate_types_crate(
    file: &TypeSpecFile,
    output_dir: &Path,
    package_name: &str,
    scalars: &ScalarMap,
    models: &ModelMap<'_>,
) -> Result<Vec<String>, CodegenError> {
    let mut generated = Vec::new();

    let src_dir = output_dir.join("src");
    fs::create_dir_all(&src_dir)?;

    // Cargo.toml — lightweight, no axum/async_trait
    let cargo_content = generate_types_cargo_toml(package_name)?;
    let cargo_path = output_dir.join("Cargo.toml");
    fs::write(&cargo_path, cargo_content)?;
    generated.push(cargo_path.display().to_string());

    // lib.rs
    let lib_content = generate_types_lib()?;
    let lib_path = src_dir.join("lib.rs");
    fs::write(&lib_path, lib_content)?;
    generated.push(lib_path.display().to_string());

    // models.rs
    let models_content = generate_models(file, scalars, models)?;
    let models_path = src_dir.join("models.rs");
    fs::write(&models_path, models_content)?;
    generated.push(models_path.display().to_string());

    // enums.rs
    let enums_content = generate_enums(file)?;
    let enums_path = src_dir.join("enums.rs");
    fs::write(&enums_path, enums_content)?;
    generated.push(enums_path.display().to_string());

    Ok(generated)
}

fn generate_types_cargo_toml(package_name: &str) -> Result<String, CodegenError> {
    let mut out = String::new();
    writeln!(out, "[package]")?;
    writeln!(out, r#"name = "{}""#, package_name)?;
    writeln!(out, r#"version = "0.1.0""#)?;
    writeln!(out, r#"edition = "2021""#)?;
    writeln!(out)?;
    writeln!(out, "[dependencies]")?;
    writeln!(
        out,
        r#"serde = {{ version = "1.0", features = ["derive"] }}"#
    )?;
    writeln!(out, r#"serde_json = "1.0""#)?;
    writeln!(
        out,
        r#"chrono = {{ version = "0.4", features = ["serde"] }}"#
    )?;
    writeln!(
        out,
        r#"uuid = {{ version = "1.0", features = ["serde", "v4"] }}"#
    )?;
    Ok(out)
}

fn generate_types_lib() -> Result<String, CodegenError> {
    let mut out = String::new();
    writeln!(out, "//! Auto-generated types from TypeSpec.")?;
    writeln!(out, "//! DO NOT EDIT.")?;
    writeln!(out)?;
    writeln!(out, "pub mod models;")?;
    writeln!(out, "pub mod enums;")?;
    Ok(out)
}

// ========== AdiService crate generation ==========

fn generate_adi_service_crate(
    file: &TypeSpecFile,
    output_dir: &Path,
    package_name: &str,
    scalars: &ScalarMap,
    models: &ModelMap<'_>,
    config: &RustAdiServiceConfig,
) -> Result<Vec<String>, CodegenError> {
    let mut generated = Vec::new();

    let src_dir = output_dir.join("src");
    fs::create_dir_all(&src_dir)?;

    // Cargo.toml
    let cargo_content = generate_adi_cargo_toml(package_name, &config.types_crate)?;
    let cargo_path = output_dir.join("Cargo.toml");
    fs::write(&cargo_path, cargo_content)?;
    generated.push(cargo_path.display().to_string());

    // lib.rs
    let lib_content = generate_adi_lib()?;
    let lib_path = src_dir.join("lib.rs");
    fs::write(&lib_path, lib_content)?;
    generated.push(lib_path.display().to_string());

    // adi_service.rs
    let adi_content = generate_adi_service(file, scalars, models, config)?;
    let adi_path = src_dir.join("adi_service.rs");
    fs::write(&adi_path, adi_content)?;
    generated.push(adi_path.display().to_string());

    Ok(generated)
}

fn generate_adi_cargo_toml(
    package_name: &str,
    types_crate: &str,
) -> Result<String, CodegenError> {
    let mut out = String::new();
    writeln!(out, "[package]")?;
    writeln!(out, r#"name = "{}""#, package_name)?;
    writeln!(out, r#"version = "0.1.0""#)?;
    writeln!(out, r#"edition = "2021""#)?;
    writeln!(out)?;
    writeln!(out, "[dependencies]")?;
    writeln!(
        out,
        r#"{} = {{ path = "../types" }}"#,
        types_crate
    )?;
    writeln!(
        out,
        r#"serde = {{ version = "1.0", features = ["derive"] }}"#
    )?;
    writeln!(out, r#"serde_json = "1.0""#)?;
    writeln!(out, r#"async-trait = "0.1""#)?;
    writeln!(
        out,
        r#"chrono = {{ version = "0.4", features = ["serde"] }}"#
    )?;
    writeln!(
        out,
        r#"uuid = {{ version = "1.0", features = ["serde", "v4"] }}"#
    )?;
    Ok(out)
}

fn generate_adi_lib() -> Result<String, CodegenError> {
    let mut out = String::new();
    writeln!(out, "//! Auto-generated AdiService implementation from TypeSpec.")?;
    writeln!(out, "//! DO NOT EDIT.")?;
    writeln!(out)?;
    writeln!(out, "pub mod adi_service;")?;
    Ok(out)
}

fn generate_adi_service(
    file: &TypeSpecFile,
    scalars: &ScalarMap,
    models: &ModelMap<'_>,
    config: &RustAdiServiceConfig,
) -> Result<String, CodegenError> {
    let types_crate_ident = config.types_crate.replace('-', "_");

    let mut out = String::new();

    writeln!(out, "//! Auto-generated AdiService handlers from TypeSpec.")?;
    writeln!(out, "//! DO NOT EDIT.")?;
    writeln!(out, "//!")?;
    writeln!(
        out,
        "//! Implement the handler trait and wrap with the generated AdiService struct."
    )?;
    writeln!(out)?;
    writeln!(out, "#![allow(unused_imports)]")?;
    writeln!(out)?;
    writeln!(out, "use {}::models::*;", types_crate_ident)?;
    writeln!(out, "use {}::enums::*;", types_crate_ident)?;
    writeln!(out, "use async_trait::async_trait;")?;
    writeln!(out, "use serde_json::Value as JsonValue;")?;
    writeln!(out)?;

    // Re-export error/result types so consumers don't need cocoon dependency for the trait
    writeln!(out, "/// Error type for AdiService handlers.")?;
    writeln!(out, "#[derive(Debug, Clone)]")?;
    writeln!(out, "pub struct AdiServiceError {{")?;
    writeln!(out, "    pub code: String,")?;
    writeln!(out, "    pub message: String,")?;
    writeln!(out, "}}")?;
    writeln!(out)?;
    writeln!(out, "impl AdiServiceError {{")?;
    writeln!(
        out,
        "    pub fn not_found(message: impl Into<String>) -> Self {{"
    )?;
    writeln!(
        out,
        r#"        Self {{ code: "not_found".to_string(), message: message.into() }}"#
    )?;
    writeln!(out, "    }}")?;
    writeln!(
        out,
        "    pub fn invalid_params(message: impl Into<String>) -> Self {{"
    )?;
    writeln!(
        out,
        r#"        Self {{ code: "invalid_params".to_string(), message: message.into() }}"#
    )?;
    writeln!(out, "    }}")?;
    writeln!(
        out,
        "    pub fn internal(message: impl Into<String>) -> Self {{"
    )?;
    writeln!(
        out,
        r#"        Self {{ code: "internal".to_string(), message: message.into() }}"#
    )?;
    writeln!(out, "    }}")?;
    writeln!(
        out,
        "    pub fn method_not_found(method: &str) -> Self {{"
    )?;
    writeln!(
        out,
        r#"        Self {{ code: "method_not_found".to_string(), message: format!("Method '{{}}' not found", method) }}"#
    )?;
    writeln!(out, "    }}")?;
    writeln!(out, "}}")?;
    writeln!(out)?;

    // AdiHandleResult
    writeln!(out, "/// Result of handling an AdiService request.")?;
    writeln!(out, "pub enum AdiHandleResult {{")?;
    writeln!(out, "    Success(JsonValue),")?;
    writeln!(out, "}}")?;
    writeln!(out)?;

    // AdiMethodInfo
    writeln!(out, "/// Metadata about an AdiService method.")?;
    writeln!(out, "#[derive(Debug, Clone, Default)]")?;
    writeln!(out, "pub struct AdiMethodInfo {{")?;
    writeln!(out, "    pub name: String,")?;
    writeln!(out, "    pub description: String,")?;
    writeln!(out, "    pub streaming: bool,")?;
    writeln!(out, "    pub params_schema: Option<JsonValue>,")?;
    writeln!(out, "    pub result_schema: Option<JsonValue>,")?;
    writeln!(out, "    pub deprecated: bool,")?;
    writeln!(out, "    pub deprecation_message: Option<String>,")?;
    writeln!(out, "}}")?;
    writeln!(out)?;

    // AdiService trait (simplified version matching cocoon's)
    writeln!(out, "/// Trait for AdiService implementations.")?;
    writeln!(out, "#[async_trait]")?;
    writeln!(out, "pub trait AdiServiceTrait: Send + Sync {{")?;
    writeln!(out, "    fn service_id(&self) -> &str;")?;
    writeln!(out, "    fn name(&self) -> &str;")?;
    writeln!(out, "    fn version(&self) -> &str;")?;
    writeln!(
        out,
        "    fn description(&self) -> Option<&str> {{ None }}"
    )?;
    writeln!(
        out,
        "    fn methods(&self) -> Vec<AdiMethodInfo>;"
    )?;
    writeln!(
        out,
        "    async fn handle(&self, method: &str, params: JsonValue) -> Result<AdiHandleResult, AdiServiceError>;"
    )?;
    writeln!(out, "}}")?;
    writeln!(out)?;

    // For each interface, generate a handler trait and AdiService wrapper
    for iface in file.interfaces() {
        let trait_name = format!("{}AdiHandler", iface.name);
        let wrapper_name = format!("{}Adi", iface.name);

        // --- Handler trait ---
        writeln!(out, "#[async_trait]")?;
        writeln!(
            out,
            "pub trait {}: Send + Sync {{",
            trait_name
        )?;

        for op in &iface.operations {
            let fn_name = op.name.to_case(Case::Snake);
            let resolved = resolve_op_params(op, scalars, models);
            let resp = op
                .return_type
                .as_ref()
                .map(|t| extract_response_info(t, scalars))
                .unwrap_or(ResponseInfo {
                    status_code: 200,
                    body_type: None,
                    is_streaming: false,
                });

            write!(out, "    async fn {}(&self", fn_name)?;

            // Path params
            for p in resolved.iter().filter(|p| matches!(p.kind, ParamKind::Path)) {
                let name = p.name.to_case(Case::Snake);
                write!(out, ", {}: {}", name, p.rust_type)?;
            }

            // Query params — collect into a struct or inline
            let query_params: Vec<_> = resolved
                .iter()
                .filter(|p| matches!(p.kind, ParamKind::Query))
                .collect();
            if !query_params.is_empty() {
                // Use the same query struct as server side
                let struct_name = format!(
                    "{}{}Query",
                    iface.name,
                    op.name.to_case(Case::Pascal)
                );
                write!(out, ", query: {}", struct_name)?;
            }

            // Body param
            for p in resolved.iter().filter(|p| matches!(p.kind, ParamKind::Body)) {
                write!(out, ", body: {}", p.rust_type)?;
            }

            let return_type = match &resp.body_type {
                Some(ty) if !resp.is_streaming => ty.clone(),
                _ => "()".to_string(),
            };

            writeln!(
                out,
                ") -> Result<{}, AdiServiceError>;",
                return_type
            )?;
        }

        writeln!(out, "}}")?;
        writeln!(out)?;

        // --- Query param structs (same as server) ---
        let mut emitted_query_structs = std::collections::HashSet::new();
        for op in &iface.operations {
            let resolved = resolve_op_params(op, scalars, models);
            let query_params: Vec<_> = resolved
                .iter()
                .filter(|p| matches!(p.kind, ParamKind::Query))
                .collect();
            if !query_params.is_empty() {
                let struct_name = format!(
                    "{}{}Query",
                    iface.name,
                    op.name.to_case(Case::Pascal)
                );
                if emitted_query_structs.insert(struct_name.clone()) {
                    writeln!(out, "#[derive(Debug, serde::Deserialize, serde::Serialize)]")?;
                    writeln!(out, "#[serde(rename_all = \"camelCase\")]")?;
                    writeln!(out, "pub struct {} {{", struct_name)?;
                    for p in &query_params {
                        let field_name = p.name.to_case(Case::Snake);
                        let field_name = if is_rust_keyword(&field_name) {
                            format!("r#{}", field_name)
                        } else {
                            field_name
                        };
                        writeln!(out, "    pub {}: {},", field_name, p.rust_type)?;
                    }
                    writeln!(out, "}}")?;
                    writeln!(out)?;
                }
            }
        }

        // --- AdiService wrapper ---
        writeln!(
            out,
            "/// AdiService wrapper for {} that dispatches to the handler trait.",
            iface.name
        )?;
        writeln!(
            out,
            "pub struct {}<H: {}> {{",
            wrapper_name, trait_name
        )?;
        writeln!(out, "    handler: H,")?;
        writeln!(out, "}}")?;
        writeln!(out)?;

        writeln!(
            out,
            "impl<H: {}> {}<H> {{",
            trait_name, wrapper_name
        )?;
        writeln!(out, "    pub fn new(handler: H) -> Self {{")?;
        writeln!(out, "        Self {{ handler }}")?;
        writeln!(out, "    }}")?;
        writeln!(out, "}}")?;
        writeln!(out)?;

        // Implement AdiServiceTrait
        writeln!(out, "#[async_trait]")?;
        writeln!(
            out,
            "impl<H: {} + 'static> AdiServiceTrait for {}<H> {{",
            trait_name, wrapper_name
        )?;

        // service_id
        writeln!(
            out,
            "    fn service_id(&self) -> &str {{ \"{}\" }}",
            config.service_id
        )?;

        // name
        writeln!(
            out,
            "    fn name(&self) -> &str {{ \"{}\" }}",
            config.service_name
        )?;

        // version
        writeln!(
            out,
            "    fn version(&self) -> &str {{ \"{}\" }}",
            config.service_version
        )?;

        // methods — generate AdiMethodInfo for each operation
        writeln!(out, "    fn methods(&self) -> Vec<AdiMethodInfo> {{")?;
        writeln!(out, "        vec![")?;

        for op in &iface.operations {
            let fn_name = op.name.to_case(Case::Snake);
            let description = get_description(&op.decorators).unwrap_or_default();
            let resolved = resolve_op_params(op, scalars, models);

            // Build a simple params schema
            let has_params = !resolved.is_empty();

            writeln!(out, "            AdiMethodInfo {{")?;
            writeln!(
                out,
                "                name: \"{}\".to_string(),",
                fn_name
            )?;
            writeln!(
                out,
                "                description: \"{}\".to_string(),",
                description.replace('"', "\\\"")
            )?;
            writeln!(out, "                streaming: false,")?;

            if has_params {
                // Generate a JSON schema for params
                writeln!(out, "                params_schema: Some(serde_json::json!({{")?;
                writeln!(out, "                    \"type\": \"object\",")?;
                writeln!(out, "                    \"properties\": {{")?;
                for p in &resolved {
                    let json_type = rust_type_to_json_schema_type(&p.rust_type);
                    writeln!(
                        out,
                        "                        \"{}\": {{ \"type\": \"{}\" }},",
                        p.name.to_case(Case::Snake),
                        json_type
                    )?;
                }
                writeln!(out, "                    }}")?;
                writeln!(out, "                }})),")?;
            } else {
                writeln!(out, "                params_schema: None,")?;
            }

            writeln!(out, "                result_schema: None,")?;
            writeln!(out, "                ..Default::default()")?;
            writeln!(out, "            }},")?;
        }

        writeln!(out, "        ]")?;
        writeln!(out, "    }}")?;
        writeln!(out)?;

        // handle — match dispatch
        writeln!(
            out,
            "    async fn handle(&self, method: &str, params: JsonValue) -> Result<AdiHandleResult, AdiServiceError> {{"
        )?;
        writeln!(out, "        match method {{")?;

        for op in &iface.operations {
            let fn_name = op.name.to_case(Case::Snake);
            let resolved = resolve_op_params(op, scalars, models);

            let path_params: Vec<_> = resolved
                .iter()
                .filter(|p| matches!(p.kind, ParamKind::Path))
                .collect();
            let query_params: Vec<_> = resolved
                .iter()
                .filter(|p| matches!(p.kind, ParamKind::Query))
                .collect();
            let body_param = resolved
                .iter()
                .find(|p| matches!(p.kind, ParamKind::Body));

            writeln!(out, "            \"{}\" => {{", fn_name)?;

            // Deserialize path params from params object
            for p in &path_params {
                let snake = p.name.to_case(Case::Snake);
                let deser = json_deserialize_expr(&p.rust_type, &snake);
                writeln!(out, "                let {} = {};", snake, deser)?;
            }

            // Deserialize query params
            if !query_params.is_empty() {
                let struct_name = format!(
                    "{}{}Query",
                    iface.name,
                    op.name.to_case(Case::Pascal)
                );
                writeln!(
                    out,
                    "                let query: {} = serde_json::from_value(params.clone()).map_err(|e| AdiServiceError::invalid_params(e.to_string()))?;",
                    struct_name
                )?;
            }

            // Deserialize body param
            if let Some(bp) = body_param {
                writeln!(
                    out,
                    "                let body: {} = serde_json::from_value(params.clone()).map_err(|e| AdiServiceError::invalid_params(e.to_string()))?;",
                    bp.rust_type
                )?;
            }

            // Build call args
            write!(out, "                let result = self.handler.{}(", fn_name)?;
            let mut first = true;
            for p in &path_params {
                if !first {
                    write!(out, ", ")?;
                }
                write!(out, "{}", p.name.to_case(Case::Snake))?;
                first = false;
            }
            if !query_params.is_empty() {
                if !first {
                    write!(out, ", ")?;
                }
                write!(out, "query")?;
                first = false;
            }
            if body_param.is_some() {
                if !first {
                    write!(out, ", ")?;
                }
                write!(out, "body")?;
            }
            writeln!(out, ").await?;")?;

            writeln!(
                out,
                "                Ok(AdiHandleResult::Success(serde_json::to_value(result).unwrap()))"
            )?;
            writeln!(out, "            }}")?;
        }

        writeln!(
            out,
            "            _ => Err(AdiServiceError::method_not_found(method)),"
        )?;
        writeln!(out, "        }}")?;
        writeln!(out, "    }}")?;

        writeln!(out, "}}")?;
        writeln!(out)?;
    }

    // If multiple interfaces, generate a combined wrapper
    let interfaces: Vec<_> = file.interfaces().collect();
    if interfaces.len() > 1 {
        let trait_bounds: Vec<String> = interfaces
            .iter()
            .map(|i| format!("{}AdiHandler", i.name))
            .collect();
        let combined_trait = format!("CombinedAdiHandler");

        writeln!(
            out,
            "/// Combined handler trait for all interfaces."
        )?;
        writeln!(
            out,
            "pub trait {}: {} {{}}",
            combined_trait,
            trait_bounds.join(" + ")
        )?;
        writeln!(
            out,
            "impl<T: {}> {} for T {{}}",
            trait_bounds.join(" + "),
            combined_trait,
        )?;
        writeln!(out)?;

        // Combined wrapper
        writeln!(
            out,
            "/// Combined AdiService wrapper for all interfaces."
        )?;
        writeln!(
            out,
            "pub struct CombinedAdi<H: CombinedAdiHandler> {{"
        )?;
        writeln!(out, "    handler: H,")?;
        writeln!(out, "}}")?;
        writeln!(out)?;

        writeln!(
            out,
            "impl<H: CombinedAdiHandler> CombinedAdi<H> {{"
        )?;
        writeln!(out, "    pub fn new(handler: H) -> Self {{")?;
        writeln!(out, "        Self {{ handler }}")?;
        writeln!(out, "    }}")?;
        writeln!(out, "}}")?;
        writeln!(out)?;

        writeln!(out, "#[async_trait]")?;
        writeln!(
            out,
            "impl<H: CombinedAdiHandler + 'static> AdiServiceTrait for CombinedAdi<H> {{"
        )?;
        writeln!(
            out,
            "    fn service_id(&self) -> &str {{ \"{}\" }}",
            config.service_id
        )?;
        writeln!(
            out,
            "    fn name(&self) -> &str {{ \"{}\" }}",
            config.service_name
        )?;
        writeln!(
            out,
            "    fn version(&self) -> &str {{ \"{}\" }}",
            config.service_version
        )?;

        // Merge methods from all interfaces
        writeln!(out, "    fn methods(&self) -> Vec<AdiMethodInfo> {{")?;
        writeln!(out, "        let mut methods = Vec::new();")?;
        for iface in &interfaces {
            for op in &iface.operations {
                let fn_name = op.name.to_case(Case::Snake);
                let description = get_description(&op.decorators).unwrap_or_default();
                let resolved = resolve_op_params(op, scalars, models);
                let has_params = !resolved.is_empty();

                writeln!(out, "        methods.push(AdiMethodInfo {{")?;
                writeln!(
                    out,
                    "            name: \"{}\".to_string(),",
                    fn_name
                )?;
                writeln!(
                    out,
                    "            description: \"{}\".to_string(),",
                    description.replace('"', "\\\"")
                )?;
                writeln!(out, "            streaming: false,")?;
                if has_params {
                    writeln!(out, "            params_schema: Some(serde_json::json!({{")?;
                    writeln!(out, "                \"type\": \"object\",")?;
                    writeln!(out, "                \"properties\": {{")?;
                    for p in &resolved {
                        let json_type = rust_type_to_json_schema_type(&p.rust_type);
                        writeln!(
                            out,
                            "                    \"{}\": {{ \"type\": \"{}\" }},",
                            p.name.to_case(Case::Snake),
                            json_type
                        )?;
                    }
                    writeln!(out, "                }}")?;
                    writeln!(out, "            }})),")?;
                } else {
                    writeln!(out, "            params_schema: None,")?;
                }
                writeln!(out, "            result_schema: None,")?;
                writeln!(out, "            ..Default::default()")?;
                writeln!(out, "        }});")?;
            }
        }
        writeln!(out, "        methods")?;
        writeln!(out, "    }}")?;
        writeln!(out)?;

        // Combined handle dispatch
        writeln!(
            out,
            "    async fn handle(&self, method: &str, params: JsonValue) -> Result<AdiHandleResult, AdiServiceError> {{"
        )?;
        writeln!(out, "        match method {{")?;

        for iface in &interfaces {
            for op in &iface.operations {
                let fn_name = op.name.to_case(Case::Snake);
                let resolved = resolve_op_params(op, scalars, models);

                let path_params: Vec<_> = resolved
                    .iter()
                    .filter(|p| matches!(p.kind, ParamKind::Path))
                    .collect();
                let query_params: Vec<_> = resolved
                    .iter()
                    .filter(|p| matches!(p.kind, ParamKind::Query))
                    .collect();
                let body_param = resolved
                    .iter()
                    .find(|p| matches!(p.kind, ParamKind::Body));

                writeln!(out, "            \"{}\" => {{", fn_name)?;

                for p in &path_params {
                    let snake = p.name.to_case(Case::Snake);
                    let deser = json_deserialize_expr(&p.rust_type, &snake);
                    writeln!(out, "                let {} = {};", snake, deser)?;
                }

                if !query_params.is_empty() {
                    let struct_name = format!(
                        "{}{}Query",
                        iface.name,
                        op.name.to_case(Case::Pascal)
                    );
                    writeln!(
                        out,
                        "                let query: {} = serde_json::from_value(params.clone()).map_err(|e| AdiServiceError::invalid_params(e.to_string()))?;",
                        struct_name
                    )?;
                }

                if let Some(bp) = body_param {
                    writeln!(
                        out,
                        "                let body: {} = serde_json::from_value(params.clone()).map_err(|e| AdiServiceError::invalid_params(e.to_string()))?;",
                        bp.rust_type
                    )?;
                }

                write!(out, "                let result = self.handler.{}(", fn_name)?;
                let mut first = true;
                for p in &path_params {
                    if !first {
                        write!(out, ", ")?;
                    }
                    write!(out, "{}", p.name.to_case(Case::Snake))?;
                    first = false;
                }
                if !query_params.is_empty() {
                    if !first {
                        write!(out, ", ")?;
                    }
                    write!(out, "query")?;
                    first = false;
                }
                if body_param.is_some() {
                    if !first {
                        write!(out, ", ")?;
                    }
                    write!(out, "body")?;
                }
                writeln!(out, ").await?;")?;

                writeln!(
                    out,
                    "                Ok(AdiHandleResult::Success(serde_json::to_value(result).unwrap()))"
                )?;
                writeln!(out, "            }}")?;
            }
        }

        writeln!(
            out,
            "            _ => Err(AdiServiceError::method_not_found(method)),"
        )?;
        writeln!(out, "        }}")?;
        writeln!(out, "    }}")?;
        writeln!(out, "}}")?;
        writeln!(out)?;
    }

    Ok(out)
}

/// Map a Rust type string to a JSON Schema type string
fn rust_type_to_json_schema_type(rust_type: &str) -> &str {
    match rust_type {
        "String" => "string",
        "i8" | "i16" | "i32" | "i64" | "u8" | "u16" | "u32" | "u64" => "integer",
        "f32" | "f64" => "number",
        "bool" => "boolean",
        t if t.starts_with("Vec<") => "array",
        t if t.starts_with("Option<") => "string",
        t if t.starts_with("HashMap<") => "object",
        "Uuid" => "string",
        "DateTime<Utc>" => "string",
        _ => "object",
    }
}

/// Generate a deserialization expression for extracting a typed param from JSON
fn json_deserialize_expr(rust_type: &str, name: &str) -> String {
    match rust_type {
        "i64" => format!(
            "params[\"{}\"].as_i64().ok_or_else(|| AdiServiceError::invalid_params(\"missing {}\"))?",
            name, name
        ),
        "i32" => format!(
            "params[\"{}\"].as_i64().ok_or_else(|| AdiServiceError::invalid_params(\"missing {}\"))? as i32",
            name, name
        ),
        "u64" => format!(
            "params[\"{}\"].as_u64().ok_or_else(|| AdiServiceError::invalid_params(\"missing {}\"))?",
            name, name
        ),
        "String" => format!(
            "params[\"{}\"].as_str().ok_or_else(|| AdiServiceError::invalid_params(\"missing {}\"))?.to_string()",
            name, name
        ),
        "bool" => format!(
            "params[\"{}\"].as_bool().ok_or_else(|| AdiServiceError::invalid_params(\"missing {}\"))?",
            name, name
        ),
        _ => format!(
            "serde_json::from_value(params[\"{}\"].clone()).map_err(|e| AdiServiceError::invalid_params(e.to_string()))?",
            name
        ),
    }
}

/// Convert TypeSpec type to Rust type string
pub fn type_to_rust(type_ref: &TypeRef, optional: bool, scalars: &ScalarMap) -> String {
    let base = match type_ref {
        TypeRef::Builtin(name) => builtin_to_rust(name),
        TypeRef::Named(name) => {
            // Check if this is a well-known scalar type
            match name.as_str() {
                "uuid" => "Uuid".to_string(),
                "email" | "url" => "String".to_string(),
                _ => {
                    // Check if this is a custom scalar type
                    if let Some(base_type) = scalars.get(name) {
                        builtin_to_rust(base_type)
                    } else {
                        name.clone()
                    }
                }
            }
        }
        TypeRef::Qualified(parts) => parts.last().cloned().unwrap_or_default(),
        TypeRef::Array(inner) => format!("Vec<{}>", type_to_rust(inner, false, scalars)),
        TypeRef::Generic { base, args } => {
            let base_name = type_to_rust(base, false, scalars);
            // Handle Record<T> -> HashMap<String, T>
            if base_name == "Record" && args.len() == 1 {
                format!(
                    "std::collections::HashMap<String, {}>",
                    type_to_rust(&args[0], false, scalars)
                )
            } else {
                let args_str: Vec<_> = args
                    .iter()
                    .map(|a| type_to_rust(a, false, scalars))
                    .collect();
                format!("{}<{}>", base_name, args_str.join(", "))
            }
        }
        TypeRef::Optional(inner) => format!("Option<{}>", type_to_rust(inner, false, scalars)),
        TypeRef::Union(_) => "serde_json::Value".to_string(),
        _ => "serde_json::Value".to_string(),
    };

    if optional && !matches!(type_ref, TypeRef::Optional(_)) {
        format!("Option<{}>", base)
    } else {
        base
    }
}

/// Convert TypeSpec type to Rust type string with context for inline enum generation
fn type_to_rust_with_context(
    type_ref: &TypeRef,
    optional: bool,
    scalars: &ScalarMap,
    ctx: &CodegenContext,
    model_name: &str,
    prop_name: &str,
) -> String {
    let base = match type_ref {
        TypeRef::Builtin(name) => builtin_to_rust(name),
        TypeRef::Named(name) => match name.as_str() {
            "uuid" => "Uuid".to_string(),
            "email" | "url" => "String".to_string(),
            _ => {
                if let Some(base_type) = scalars.get(name) {
                    builtin_to_rust(base_type)
                } else {
                    name.clone()
                }
            }
        },
        TypeRef::Qualified(parts) => parts.last().cloned().unwrap_or_default(),
        TypeRef::Array(inner) => format!(
            "Vec<{}>",
            type_to_rust_with_context(inner, false, scalars, ctx, model_name, prop_name)
        ),
        TypeRef::Generic { base, args } => {
            let base_name =
                type_to_rust_with_context(base, false, scalars, ctx, model_name, prop_name);
            if base_name == "Record" && args.len() == 1 {
                format!(
                    "HashMap<String, {}>",
                    type_to_rust_with_context(&args[0], false, scalars, ctx, model_name, prop_name)
                )
            } else {
                let args_str: Vec<_> = args
                    .iter()
                    .map(|a| {
                        type_to_rust_with_context(a, false, scalars, ctx, model_name, prop_name)
                    })
                    .collect();
                format!("{}<{}>", base_name, args_str.join(", "))
            }
        }
        TypeRef::Optional(inner) => format!(
            "Option<{}>",
            type_to_rust_with_context(inner, false, scalars, ctx, model_name, prop_name)
        ),
        TypeRef::Union(variants) => {
            // Check if all variants are string literals -> generate inline enum
            let string_literals: Vec<String> = variants
                .iter()
                .filter_map(|v| match v {
                    TypeRef::StringLiteral(s) => Some(s.clone()),
                    _ => None,
                })
                .collect();

            if string_literals.len() == variants.len() && !string_literals.is_empty() {
                // All variants are string literals - register inline enum
                ctx.register_inline_enum(model_name, prop_name, &string_literals)
            } else {
                "serde_json::Value".to_string()
            }
        }
        TypeRef::StringLiteral(_) => "String".to_string(),
        TypeRef::IntLiteral(_) => "i64".to_string(),
        _ => "serde_json::Value".to_string(),
    };

    if optional && !matches!(type_ref, TypeRef::Optional(_)) {
        format!("Option<{}>", base)
    } else {
        base
    }
}

/// Convert builtin TypeSpec type to Rust
fn builtin_to_rust(name: &str) -> String {
    match name {
        "string" | "url" => "String".to_string(),
        "int8" => "i8".to_string(),
        "int16" => "i16".to_string(),
        "int32" => "i32".to_string(),
        "int64" => "i64".to_string(),
        "uint8" => "u8".to_string(),
        "uint16" => "u16".to_string(),
        "uint32" => "u32".to_string(),
        "uint64" => "u64".to_string(),
        "float32" => "f32".to_string(),
        "float64" => "f64".to_string(),
        "boolean" => "bool".to_string(),
        "utcDateTime" | "offsetDateTime" => "DateTime<Utc>".to_string(),
        "plainDate" => "chrono::NaiveDate".to_string(),
        "plainTime" => "chrono::NaiveTime".to_string(),
        "bytes" => "Vec<u8>".to_string(),
        "void" | "null" => "()".to_string(),
        _ => "serde_json::Value".to_string(),
    }
}

fn get_description(decorators: &[Decorator]) -> Option<String> {
    decorators
        .iter()
        .find(|d| d.name == "doc")
        .and_then(|d| d.get_string_arg(0).map(|s| s.to_string()))
}

fn get_route(decorators: &[Decorator]) -> Option<String> {
    decorators
        .iter()
        .find(|d| d.name == "route")
        .and_then(|d| d.get_string_arg(0).map(|s| s.to_string()))
}

fn get_http_method(decorators: &[Decorator]) -> &'static str {
    for d in decorators {
        match d.name.as_str() {
            "get" => return "GET",
            "post" => return "POST",
            "put" => return "PUT",
            "patch" => return "PATCH",
            "delete" => return "DELETE",
            _ => {}
        }
    }
    "GET"
}

fn has_decorator(decorators: &[Decorator], name: &str) -> bool {
    decorators.iter().any(|d| d.name == name)
}

fn is_rust_keyword(name: &str) -> bool {
    matches!(
        name,
        "as" | "break"
            | "const"
            | "continue"
            | "crate"
            | "else"
            | "enum"
            | "extern"
            | "false"
            | "fn"
            | "for"
            | "if"
            | "impl"
            | "in"
            | "let"
            | "loop"
            | "match"
            | "mod"
            | "move"
            | "mut"
            | "pub"
            | "ref"
            | "return"
            | "self"
            | "Self"
            | "static"
            | "struct"
            | "super"
            | "trait"
            | "true"
            | "type"
            | "unsafe"
            | "use"
            | "where"
            | "while"
            | "async"
            | "await"
            | "dyn"
    )
}
