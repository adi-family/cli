//! TypeScript Protocol Code Generator
//!
//! Generates TypeScript discriminated union types from TypeSpec protocol definitions.
//! Produces `types.ts`, `messages.ts`, and `index.ts`.

use std::collections::BTreeSet;
use std::fmt::Write;
use std::fs;
use std::path::Path;

use convert_case::{Case, Casing};

use crate::ast::{TypeRef, TypeSpecFile, Value};

use super::protocol::{collect_protocol_data, EnumVariant, RustProtocolConfig};
use super::typescript::type_to_typescript;
use super::{build_model_map, build_scalar_map, resolve_properties, CodegenError, ModelMap, ScalarMap};

/// Generate TypeScript protocol files from TypeSpec.
pub fn generate(
    file: &TypeSpecFile,
    output_dir: &Path,
    _package_name: &str,
    config: &RustProtocolConfig,
) -> Result<Vec<String>, CodegenError> {
    let scalars = build_scalar_map(file);
    let models = build_model_map(file);

    fs::create_dir_all(output_dir)?;

    let mut generated = Vec::new();

    // types.ts — models + enums in a single file
    let types_content = generate_types(file, &scalars, &models)?;
    let types_path = output_dir.join("types.ts");
    fs::write(&types_path, &types_content)?;
    generated.push(types_path.display().to_string());

    // messages.ts — discriminated union
    let (variants, _) = collect_protocol_data(file, &models);
    let messages_content = generate_messages(&variants, config, file)?;
    let messages_path = output_dir.join("messages.ts");
    fs::write(&messages_path, &messages_content)?;
    generated.push(messages_path.display().to_string());

    // index.ts
    let index_content = generate_index()?;
    let index_path = output_dir.join("index.ts");
    fs::write(&index_path, &index_content)?;
    generated.push(index_path.display().to_string());

    Ok(generated)
}

/// Generate the messages.ts discriminated union type.
fn generate_messages(
    variants: &[EnumVariant],
    config: &RustProtocolConfig,
    file: &TypeSpecFile,
) -> Result<String, CodegenError> {
    let mut out = String::new();

    writeln!(out, "/**")?;
    writeln!(out, " * Auto-generated protocol messages from TypeSpec.")?;
    writeln!(out, " * DO NOT EDIT.")?;
    writeln!(out, " */")?;
    writeln!(out)?;

    // Collect referenced type names for imports
    let imports = collect_type_imports(variants, file);
    if !imports.is_empty() {
        writeln!(
            out,
            "import type {{ {} }} from './types';",
            imports.into_iter().collect::<Vec<_>>().join(", ")
        )?;
        writeln!(out)?;
    }

    // Discriminated union
    writeln!(out, "export type {} =", config.enum_name)?;

    let rename_case = parse_rename_case(&config.rename);
    let mut current_channel = String::new();

    for (i, variant) in variants.iter().enumerate() {
        // Channel separator comment
        if variant.channel != current_channel {
            if !current_channel.is_empty() {
                writeln!(out)?;
            }
            writeln!(out, "  // ── {} ──", variant.channel)?;
            current_channel.clone_from(&variant.channel);
        }

        let wire_name = apply_rename(&variant.name, rename_case);

        write!(out, "  | {{ {}: '{}'", config.tag, wire_name)?;

        for field in &variant.fields {
            let ts_type = type_to_typescript(&field.type_ref);
            let opt = if field.optional { "?" } else { "" };
            write!(out, "; {}{}: {}", field.name, opt, ts_type)?;
        }

        let terminator = if i == variants.len() - 1 { ";" } else { "" };
        writeln!(out, " }}{}", terminator)?;
    }

    Ok(out)
}

/// Generate types.ts with models and enums in a single file (no cross-imports).
fn generate_types(
    file: &TypeSpecFile,
    _scalars: &ScalarMap,
    models: &ModelMap<'_>,
) -> Result<String, CodegenError> {
    let mut out = String::new();

    writeln!(out, "/**")?;
    writeln!(out, " * Auto-generated protocol types from TypeSpec.")?;
    writeln!(out, " * DO NOT EDIT.")?;
    writeln!(out, " */")?;

    // Enums first (models may reference them)
    for enum_def in file.enums() {
        writeln!(out)?;
        writeln!(out, "export enum {} {{", enum_def.name)?;
        for member in &enum_def.members {
            let value = member
                .value
                .as_ref()
                .map(|v| match v {
                    Value::String(s) => s.clone(),
                    _ => member.name.to_case(Case::Snake),
                })
                .unwrap_or_else(|| member.name.to_case(Case::Snake));
            let variant = member.name.to_case(Case::Pascal);
            writeln!(out, r#"  {} = "{}","#, variant, value)?;
        }
        writeln!(out, "}}")?;
    }

    // Models
    for model in file.models() {
        writeln!(out)?;
        let type_params = if model.type_params.is_empty() {
            String::new()
        } else {
            format!("<{}>", model.type_params.join(", "))
        };
        writeln!(out, "export interface {}{} {{", model.name, type_params)?;

        let all_properties = resolve_properties(model, models);
        for prop in all_properties {
            let ts_type = type_to_typescript(&prop.type_ref);
            let optional = if prop.optional { "?" } else { "" };
            writeln!(out, "  {}{}: {};", prop.name, optional, ts_type)?;
        }

        writeln!(out, "}}")?;
    }

    Ok(out)
}

/// Collect type names referenced in variant fields that need importing from types.ts.
fn collect_type_imports(variants: &[EnumVariant], file: &TypeSpecFile) -> BTreeSet<String> {
    let known_types: BTreeSet<String> = file
        .models()
        .map(|m| m.name.clone())
        .chain(file.enums().map(|e| e.name.clone()))
        .collect();

    let mut imports = BTreeSet::new();

    for variant in variants {
        for field in &variant.fields {
            collect_named_refs(&field.type_ref, &known_types, &mut imports);
        }
    }

    imports
}

/// Recursively extract named type references from a TypeRef.
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

/// Parse a serde rename strategy string into a convert_case Case.
fn parse_rename_case(rename: &str) -> Case {
    match rename {
        "snake_case" => Case::Snake,
        "camelCase" => Case::Camel,
        "PascalCase" => Case::Pascal,
        "kebab-case" => Case::Kebab,
        "SCREAMING_SNAKE_CASE" => Case::ScreamingSnake,
        _ => Case::Snake,
    }
}

/// Apply rename strategy to a PascalCase variant name.
fn apply_rename(name: &str, case: Case) -> String {
    name.to_case(case)
}

/// Generate the index.ts re-export file.
fn generate_index() -> Result<String, CodegenError> {
    let mut out = String::new();
    writeln!(out, "/**")?;
    writeln!(out, " * Auto-generated from TypeSpec.")?;
    writeln!(out, " */")?;
    writeln!(out)?;
    writeln!(out, "export * from './types';")?;
    writeln!(out, "export * from './messages';")?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse;

    #[test]
    fn test_generates_discriminated_union() {
        let source = r#"
model DeviceInfo {
    device_id: string;
    online: boolean;
}

@channel("auth")
interface Auth {
    @request
    authenticate(access_token: string): {
        user_id: string;
    };

    @serverPush
    hello(auth_kind: string): void;
}

@channel("device")
interface Device {
    @request
    queryDevices(tag_filter: Record<string>): {
        devices: DeviceInfo[];
    };

    @event
    peerConnected(peer_id: string): void;
}
"#;

        let file = parse(source).expect("parse failed");
        let models = build_model_map(&file);
        let (variants, _) = collect_protocol_data(&file, &models);

        let config = RustProtocolConfig {
            tag: "type".to_string(),
            rename: "snake_case".to_string(),
            enum_name: "SignalingMessage".to_string(),
        };

        let output = generate_messages(&variants, &config, &file).unwrap();

        // Check union type declaration
        assert!(output.contains("export type SignalingMessage ="));

        // Check variant wire names (snake_case)
        assert!(output.contains("type: 'auth_authenticate'"));
        assert!(output.contains("type: 'auth_authenticate_response'"));
        assert!(output.contains("type: 'auth_hello'"));
        assert!(output.contains("type: 'device_query_devices'"));
        assert!(output.contains("type: 'device_query_devices_response'"));
        assert!(output.contains("type: 'device_peer_connected'"));

        // Check field types
        assert!(output.contains("access_token: string"));
        assert!(output.contains("user_id: string"));
        assert!(output.contains("devices: DeviceInfo[]"));
        assert!(output.contains("peer_id: string"));

        // Check imports
        assert!(output.contains("import type { DeviceInfo } from './types'"));

        // Check channel comments
        assert!(output.contains("// ── auth ──"));
        assert!(output.contains("// ── device ──"));
    }

    #[test]
    fn test_optional_fields_in_union() {
        let source = r#"
@channel("device")
interface Device {
    @request
    register(secret: string, device_id?: string, version: string): {
        device_id: string;
    };
}
"#;

        let file = parse(source).expect("parse failed");
        let models = build_model_map(&file);
        let (variants, _) = collect_protocol_data(&file, &models);

        let config = RustProtocolConfig {
            tag: "type".to_string(),
            rename: "snake_case".to_string(),
            enum_name: "Msg".to_string(),
        };

        let output = generate_messages(&variants, &config, &file).unwrap();

        // Optional fields should have ? suffix
        assert!(output.contains("device_id?: string"));
        // Required fields should not
        assert!(output.contains("secret: string"));
        assert!(output.contains("version: string"));
    }

    #[test]
    fn test_enum_type_imports() {
        let source = r#"
enum AuthRequirement {
    required: "required",
    optional: "optional",
}

@channel("auth")
interface Auth {
    @serverPush
    hello(auth_kind: string, auth_requirement: AuthRequirement): void;
}
"#;

        let file = parse(source).expect("parse failed");
        let models = build_model_map(&file);
        let (variants, _) = collect_protocol_data(&file, &models);

        let config = RustProtocolConfig {
            tag: "type".to_string(),
            rename: "snake_case".to_string(),
            enum_name: "Msg".to_string(),
        };

        let output = generate_messages(&variants, &config, &file).unwrap();

        assert!(output.contains("import type { AuthRequirement } from './types'"));
        assert!(output.contains("auth_requirement: AuthRequirement"));
    }

    #[test]
    fn test_full_generation() {
        let source = r#"
enum AuthOption {
    verified: "verified",
    anonymous: "anonymous",
}

model ConnectionInfo {
    manual_allowed: boolean;
}

@channel("auth")
interface Auth {
    @serverPush
    hello(auth_kind: string, auth_options: AuthOption[]): void;

    @serverPush
    helloAuthed(user_id: string, connection_info: ConnectionInfo): void;
}

@channel("system")
interface System {
    @event
    error(message: string): void;
}
"#;

        let file = parse(source).expect("parse failed");
        let config = RustProtocolConfig {
            tag: "type".to_string(),
            rename: "snake_case".to_string(),
            enum_name: "SignalingMessage".to_string(),
        };

        let dir = tempfile::tempdir().unwrap();
        let generated = generate(&file, dir.path(), "test", &config).unwrap();

        assert_eq!(generated.len(), 3); // types.ts, messages.ts, index.ts

        let types = std::fs::read_to_string(dir.path().join("types.ts")).unwrap();
        assert!(types.contains("export interface ConnectionInfo"));
        assert!(types.contains("export enum AuthOption"));

        let messages = std::fs::read_to_string(dir.path().join("messages.ts")).unwrap();
        assert!(messages.contains("export type SignalingMessage ="));
        assert!(messages.contains("type: 'auth_hello'"));
        assert!(messages.contains("type: 'system_error'"));

        let index = std::fs::read_to_string(dir.path().join("index.ts")).unwrap();
        assert!(index.contains("export * from './types'"));
        assert!(index.contains("export * from './messages'"));
    }
}
