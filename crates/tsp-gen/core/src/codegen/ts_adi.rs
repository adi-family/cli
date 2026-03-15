//! TypeScript AdiService Client Generator
//!
//! Generates typed API client functions from `@channel` interfaces with `@request` operations.
//! Each `@request` operation becomes an exported function that calls `c.request<T>(SVC, method, params)`.
//!
//! Output: `adi-client.ts` with typed wrapper functions.

use std::collections::BTreeSet;
use std::fmt::Write;
use std::fs;
use std::path::Path;

use convert_case::{Case, Casing};

use crate::ast::{Decorator, OperationParam, TypeRef, TypeSpecFile};

use super::protocol::get_channel_name;
use super::typescript::type_to_typescript;
use super::{build_model_map, build_scalar_map, CodegenError, ModelMap};

/// A resolved @channel interface ready for TS client generation.
struct ChannelService {
    /// Channel name from @channel("adi.my-plugin")
    channel: String,
    /// Interface name (PascalCase)
    #[allow(dead_code)]
    name: String,
    /// Operations marked with @request
    operations: Vec<ServiceOperation>,
}

struct ServiceOperation {
    /// camelCase function name
    fn_name: String,
    /// snake_case wire method name
    wire_name: String,
    /// Parameters (excluding spread resolution)
    params: Vec<OpParam>,
    /// Return type as TypeScript string
    return_type: String,
}

struct OpParam {
    name: String,
    ts_type: String,
    optional: bool,
}

/// Generate TypeScript adi-client from TypeSpec.
pub fn generate(
    file: &TypeSpecFile,
    output_dir: &Path,
    _package_name: &str,
) -> Result<Vec<String>, CodegenError> {
    let _scalars = build_scalar_map(file);
    let models = build_model_map(file);

    fs::create_dir_all(output_dir)?;

    let mut services = collect_channel_services(file, &models);
    if services.is_empty() {
        return Ok(Vec::new());
    }

    // Prefix function names with channel name when multiple channels exist
    if services.len() > 1 {
        for service in &mut services {
            let prefix = service.channel.to_case(Case::Camel);
            for op in &mut service.operations {
                let capitalized = {
                    let mut chars = op.fn_name.chars();
                    match chars.next() {
                        Some(c) => c.to_uppercase().to_string() + chars.as_str(),
                        None => String::new(),
                    }
                };
                op.fn_name = format!("{}{}", prefix, capitalized);
            }
        }
    }

    let mut generated = Vec::new();

    let content = generate_adi_client(file, &services)?;
    let path = output_dir.join("adi-client.ts");
    fs::write(&path, &content)?;
    generated.push(path.display().to_string());

    Ok(generated)
}

/// Collect @channel interfaces with @request operations.
fn collect_channel_services(file: &TypeSpecFile, models: &ModelMap<'_>) -> Vec<ChannelService> {
    let mut services = Vec::new();

    for iface in file.interfaces() {
        let channel = match get_channel_name(&iface.decorators) {
            Some(name) => name,
            None => continue,
        };

        let mut operations = Vec::new();

        for op in &iface.operations {
            if !has_decorator(&op.decorators, "request") {
                continue;
            }

            let fn_name = escape_js_reserved(&op.name.to_case(Case::Camel));
            let wire_name = op.name.to_case(Case::Snake);
            let return_type = op
                .return_type
                .as_ref()
                .map(|t| type_to_typescript(t))
                .unwrap_or_else(|| "void".to_string());

            let params = resolve_op_params(&op.params, models);

            operations.push(ServiceOperation {
                fn_name,
                wire_name,
                params,
                return_type,
            });
        }

        if !operations.is_empty() {
            services.push(ChannelService {
                channel,
                name: iface.name.clone(),
                operations,
            });
        }
    }

    services
}

/// Resolve operation parameters, expanding spread types.
fn resolve_op_params(params: &[OperationParam], models: &ModelMap<'_>) -> Vec<OpParam> {
    let mut result = Vec::new();

    for param in params {
        if param.spread {
            if let Some(name) = type_ref_name(&param.type_ref) {
                if let Some(model) = models.get(name.as_str()) {
                    for prop in &model.properties {
                        result.push(OpParam {
                            name: prop.name.clone(),
                            ts_type: type_to_typescript(&prop.type_ref),
                            optional: prop.optional,
                        });
                    }
                }
            }
            continue;
        }

        result.push(OpParam {
            name: param.name.clone(),
            ts_type: type_to_typescript(&param.type_ref),
            optional: param.optional,
        });
    }

    result
}

/// Generate adi-client.ts content.
fn generate_adi_client(
    file: &TypeSpecFile,
    services: &[ChannelService],
) -> Result<String, CodegenError> {
    let mut out = String::new();

    writeln!(out, "/**")?;
    writeln!(out, " * Auto-generated ADI service client from TypeSpec.")?;
    writeln!(out, " * DO NOT EDIT.")?;
    writeln!(out, " */")?;
    writeln!(out, "import type {{ Connection }} from '@adi-family/cocoon-plugin-interface';")?;

    // Collect type imports from return types and params, split by source file
    let (model_imports, enum_imports) = collect_type_imports_split(services, file);
    if !model_imports.is_empty() {
        writeln!(
            out,
            "import type {{ {} }} from './models.js';",
            model_imports.into_iter().collect::<Vec<_>>().join(", ")
        )?;
    }
    if !enum_imports.is_empty() {
        writeln!(
            out,
            "import {{ {} }} from './enums.js';",
            enum_imports.into_iter().collect::<Vec<_>>().join(", ")
        )?;
    }

    for service in services {
        let svc_const = format!("SVC_{}", service.channel.replace('.', "_").to_case(Case::ScreamingSnake));
        writeln!(out)?;
        writeln!(out, "const {} = '{}';", svc_const, service.channel)?;

        for op in &service.operations {
            writeln!(out)?;
            write_operation(&mut out, op, &svc_const)?;
        }
    }

    Ok(out)
}

/// Write a single operation as an exported function.
fn write_operation(out: &mut String, op: &ServiceOperation, svc_const: &str) -> Result<(), CodegenError> {
    let required: Vec<&OpParam> = op.params.iter().filter(|p| !p.optional).collect();
    let optional: Vec<&OpParam> = op.params.iter().filter(|p| p.optional).collect();

    // Decide signature style:
    // - Single required param, no optional → positional: fn(c, value)
    // - Otherwise → params object: fn(c, params)
    if required.len() == 1 && optional.is_empty() {
        let param = required[0];
        writeln!(
            out,
            "export const {} = (c: Connection, {}: {}) =>",
            op.fn_name, param.name, param.ts_type
        )?;
        writeln!(
            out,
            "  c.request<{}>({}, '{}', {{ {} }});",
            op.return_type, svc_const, op.wire_name, param.name
        )?;
    } else if op.params.is_empty() {
        writeln!(
            out,
            "export const {} = (c: Connection) =>",
            op.fn_name
        )?;
        writeln!(
            out,
            "  c.request<{}>({}, '{}', {{}});",
            op.return_type, svc_const, op.wire_name
        )?;
    } else {
        // Build params object type
        let has_required = !required.is_empty();
        let has_optional = !optional.is_empty();

        let param_type = if has_required && has_optional {
            // Mix of required and optional → single object
            let mut fields = String::new();
            for p in &required {
                write!(fields, " {}: {};", p.name, p.ts_type)?;
            }
            for p in &optional {
                write!(fields, " {}?: {};", p.name, p.ts_type)?;
            }
            format!("{{{} }}", fields)
        } else if has_optional {
            // All optional
            let mut fields = String::new();
            for p in &optional {
                write!(fields, " {}?: {};", p.name, p.ts_type)?;
            }
            format!("{{{} }}", fields)
        } else {
            // All required (2+)
            let mut fields = String::new();
            for p in &required {
                write!(fields, " {}: {};", p.name, p.ts_type)?;
            }
            format!("{{{} }}", fields)
        };

        let optional_marker = if !has_required { "?" } else { "" };
        let fallback = if !has_required { " ?? {}" } else { "" };

        writeln!(
            out,
            "export const {} = (c: Connection, params{}: {}) =>",
            op.fn_name, optional_marker, param_type
        )?;
        writeln!(
            out,
            "  c.request<{}>({}, '{}', params{});",
            op.return_type, svc_const, op.wire_name, fallback
        )?;
    }

    Ok(())
}

/// Collect type names split into model imports and enum imports.
fn collect_type_imports_split(
    services: &[ChannelService],
    file: &TypeSpecFile,
) -> (BTreeSet<String>, BTreeSet<String>) {
    let known_models: BTreeSet<String> = file.models().map(|m| m.name.clone()).collect();
    let known_enums: BTreeSet<String> = file.enums().map(|e| e.name.clone()).collect();
    let known_all: BTreeSet<String> = known_models.union(&known_enums).cloned().collect();

    let mut all_imports = BTreeSet::new();

    for service in services {
        for op in &service.operations {
            if let Some(rt) = &file
                .interfaces()
                .find(|i| get_channel_name(&i.decorators).as_deref() == Some(&service.channel))
                .and_then(|i| {
                    i.operations
                        .iter()
                        .find(|o| o.name.to_case(Case::Camel) == op.fn_name)
                })
                .and_then(|o| o.return_type.as_ref())
            {
                collect_named_refs(rt, &known_all, &mut all_imports);
            }

            for param in &op.params {
                if known_all.contains(&param.ts_type) {
                    all_imports.insert(param.ts_type.clone());
                }
            }
        }
    }

    let model_imports: BTreeSet<String> =
        all_imports.intersection(&known_models).cloned().collect();
    let enum_imports: BTreeSet<String> = all_imports.intersection(&known_enums).cloned().collect();

    (model_imports, enum_imports)
}

/// Recursively extract named type references.
fn collect_named_refs(type_ref: &TypeRef, known: &BTreeSet<String>, out: &mut BTreeSet<String>) {
    match type_ref {
        TypeRef::Named(name) if known.contains(name) => {
            out.insert(name.clone());
        }
        TypeRef::Array(inner) => collect_named_refs(inner, known, out),
        TypeRef::Generic { base, args } => {
            collect_named_refs(base, known, out);
            for arg in args {
                collect_named_refs(arg, known, out);
            }
        }
        TypeRef::Optional(inner) => collect_named_refs(inner, known, out),
        TypeRef::Union(variants) => {
            for v in variants {
                collect_named_refs(v, known, out);
            }
        }
        _ => {}
    }
}

fn has_decorator(decorators: &[Decorator], name: &str) -> bool {
    decorators.iter().any(|d| d.name == name)
}

/// Escape JavaScript/TypeScript reserved words by appending `_`.
fn escape_js_reserved(name: &str) -> String {
    match name {
        "break" | "case" | "catch" | "class" | "const" | "continue" | "debugger" | "default"
        | "delete" | "do" | "else" | "enum" | "export" | "extends" | "false" | "finally"
        | "for" | "function" | "if" | "import" | "in" | "instanceof" | "new" | "null"
        | "return" | "super" | "switch" | "this" | "throw" | "true" | "try" | "typeof"
        | "var" | "void" | "while" | "with" | "yield" | "let" | "static" | "implements"
        | "interface" | "package" | "private" | "protected" | "public" | "await" => {
            format!("{name}_")
        }
        _ => name.to_string(),
    }
}

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
    fn test_generates_adi_client() {
        let source = r#"
enum CredentialType {
  apiKey: "api_key",
  oauth2: "oauth2",
}

model Credential {
  id: string;
  name: string;
  credential_type: CredentialType;
}

model DeleteResult {
  deleted: boolean;
}

@channel("adi.credentials")
interface CredentialsService {
  @request list(credential_type?: CredentialType, provider?: string): Credential[];
  @request get(id: string): Credential;
  @request create(name: string, credential_type: CredentialType): Credential;
  @request delete(id: string): DeleteResult;
}
"#;

        let file = parse(source).expect("parse failed");
        let dir = tempfile::tempdir().unwrap();
        let generated = generate(&file, dir.path(), "test").unwrap();

        assert_eq!(generated.len(), 1);

        let content = std::fs::read_to_string(dir.path().join("adi-client.ts")).unwrap();

        // Check header
        assert!(content.contains("Auto-generated ADI service client from TypeSpec."));
        assert!(content.contains("DO NOT EDIT."));

        // Check imports — models and enums are imported from separate files
        assert!(content.contains("import type { Connection } from '@adi-family/cocoon-plugin-interface';"));
        assert!(content.contains("from './models.js'"));
        assert!(content.contains("from './enums.js'"));
        assert!(content.contains("Credential"));
        assert!(content.contains("CredentialType"));
        assert!(content.contains("DeleteResult"));

        // Check SVC constant (uniquified per channel)
        assert!(content.contains("const SVC_ADI_CREDENTIALS = 'adi.credentials';"));

        // Check function signatures
        // list: all optional → params object with ?
        assert!(content.contains("export const list = (c: Connection, params?:"));
        // get: single required → positional
        assert!(content.contains("export const get = (c: Connection, id: string)"));
        // create: multiple required → params object
        assert!(content.contains("export const create = (c: Connection, params:"));
        // delete: single required → positional, escaped reserved word
        assert!(content.contains("export const delete_ = (c: Connection, id: string)"));

        // Check wire names are snake_case
        assert!(content.contains("'list'"));
        assert!(content.contains("'get'"));
        assert!(content.contains("'create'"));
        assert!(content.contains("'delete'"));
    }

    #[test]
    fn test_multi_word_method_snake_case_wire() {
        let source = r#"
model Item { id: string; }

@channel("adi.test")
interface TestService {
  @request getWithData(id: string): Item;
}
"#;

        let file = parse(source).expect("parse failed");
        let dir = tempfile::tempdir().unwrap();
        generate(&file, dir.path(), "test").unwrap();

        let content = std::fs::read_to_string(dir.path().join("adi-client.ts")).unwrap();

        // TS function is camelCase
        assert!(content.contains("export const getWithData ="));
        // Wire name is snake_case
        assert!(content.contains("'get_with_data'"));
    }

    #[test]
    fn test_multi_channel_prefixed_names() {
        let source = r#"
@channel("auth")
interface Auth {
  @request authenticate(token: string): { user_id: string; };
}

@channel("device")
interface Device {
  @request register(secret: string): { device_id: string; };
}
"#;

        let file = parse(source).expect("parse failed");
        let dir = tempfile::tempdir().unwrap();
        generate(&file, dir.path(), "test").unwrap();

        let content = std::fs::read_to_string(dir.path().join("adi-client.ts")).unwrap();

        // Unique SVC constants
        assert!(content.contains("const SVC_AUTH = 'auth';"));
        assert!(content.contains("const SVC_DEVICE = 'device';"));
        // No duplicate `const SVC`
        assert!(!content.contains("const SVC ="));

        // Function names prefixed with channel
        assert!(content.contains("export const authAuthenticate ="));
        assert!(content.contains("export const deviceRegister ="));
    }

    #[test]
    fn test_no_channel_no_output() {
        let source = r#"
model Item { id: string; }

interface NotAService {
  @request get(id: string): Item;
}
"#;

        let file = parse(source).expect("parse failed");
        let dir = tempfile::tempdir().unwrap();
        let generated = generate(&file, dir.path(), "test").unwrap();
        assert!(generated.is_empty());
    }
}
