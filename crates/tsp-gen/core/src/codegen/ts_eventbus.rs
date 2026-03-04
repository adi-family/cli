//! TypeScript EventBus Code Generator
//!
//! Generates TypeScript module augmentation and payload types from `@bus` interfaces.
//! Reads `@bus("name")` decorator on interfaces and `@event` on operations to produce:
//! - `types.ts` — shared models/enums + auto-generated payload interfaces
//! - `events.ts` — module augmentation for EventRegistry
//! - `index.ts` — re-exports

use std::collections::BTreeSet;
use std::fmt::Write;
use std::fs;
use std::path::Path;

use convert_case::{Case, Casing};

use crate::ast::{Decorator, OperationParam, TypeRef, TypeSpecFile};

use super::typescript::type_to_typescript;
use super::{build_model_map, build_scalar_map, resolve_properties, CodegenError, ModelMap, ScalarMap};

/// Configuration for EventBus code generation.
#[derive(Debug, Clone)]
pub struct EventBusConfig {
    /// Module path for augmentation (e.g., "@adi-family/sdk-plugin/types")
    pub module_path: String,
    /// Interface name to augment (e.g., "EventRegistry")
    pub interface_name: String,
    /// Event name rename strategy (e.g., "kebab-case")
    pub rename: String,
}

/// A resolved bus event ready for code generation.
struct BusEvent {
    /// Bus name from @bus("name") decorator
    bus_name: String,
    /// PascalCase payload type name (e.g., "SignalingAuthOkEvent")
    payload_type: String,
    /// Wire event name (e.g., "signaling:auth-ok")
    event_name: String,
    /// Flattened operation params as fields
    fields: Vec<EventField>,
}

struct EventField {
    /// Field name (camelCase, matching TypeSpec param name)
    name: String,
    /// Type reference
    type_ref: TypeRef,
    /// Whether optional
    optional: bool,
}

/// Generate TypeScript eventbus files from TypeSpec.
pub fn generate(
    file: &TypeSpecFile,
    output_dir: &Path,
    _package_name: &str,
    config: &EventBusConfig,
) -> Result<Vec<String>, CodegenError> {
    let scalars = build_scalar_map(file);
    let models = build_model_map(file);

    fs::create_dir_all(output_dir)?;

    let events = collect_eventbus_data(file, &models, config);
    let mut generated = Vec::new();

    // types.ts — shared models/enums + auto-generated payload interfaces
    let types_content = generate_types(file, &scalars, &models, &events)?;
    let types_path = output_dir.join("types.ts");
    fs::write(&types_path, &types_content)?;
    generated.push(types_path.display().to_string());

    // events.ts — module augmentation
    let events_content = generate_events(&events, config, file)?;
    let events_path = output_dir.join("events.ts");
    fs::write(&events_path, &events_content)?;
    generated.push(events_path.display().to_string());

    // index.ts
    let index_content = generate_index()?;
    let index_path = output_dir.join("index.ts");
    fs::write(&index_path, &index_content)?;
    generated.push(index_path.display().to_string());

    Ok(generated)
}

/// Collect bus events from all @bus interfaces.
fn collect_eventbus_data(
    file: &TypeSpecFile,
    models: &ModelMap<'_>,
    config: &EventBusConfig,
) -> Vec<BusEvent> {
    let rename_case = parse_rename_case(&config.rename);
    let mut events = Vec::new();

    for iface in file.interfaces() {
        let bus_name = match get_bus_name(&iface.decorators) {
            Some(name) => name,
            None => continue,
        };

        let bus_pascal = bus_name.to_case(Case::Pascal);

        for op in &iface.operations {
            let op_pascal = op.name.to_case(Case::Pascal);
            let payload_type = format!("{}{}Event", bus_pascal, op_pascal);
            let event_name = format!("{}:{}", bus_name, op.name.to_case(rename_case));
            let fields = resolve_event_fields(&op.params, models);

            events.push(BusEvent {
                bus_name: bus_name.clone(),
                payload_type,
                event_name,
                fields,
            });
        }
    }

    events
}

/// Generate types.ts with shared models/enums + payload interfaces.
fn generate_types(
    file: &TypeSpecFile,
    _scalars: &ScalarMap,
    models: &ModelMap<'_>,
    events: &[BusEvent],
) -> Result<String, CodegenError> {
    let mut out = String::new();

    writeln!(out, "/**")?;
    writeln!(out, " * Auto-generated eventbus types from TypeSpec.")?;
    writeln!(out, " * DO NOT EDIT.")?;
    writeln!(out, " */")?;

    // Enums first (payload types may reference them)
    for enum_def in file.enums() {
        writeln!(out)?;
        writeln!(out, "export enum {} {{", enum_def.name)?;
        for member in &enum_def.members {
            let value = member
                .value
                .as_ref()
                .map(|v| match v {
                    crate::ast::Value::String(s) => s.clone(),
                    _ => member.name.to_case(Case::Snake),
                })
                .unwrap_or_else(|| member.name.to_case(Case::Snake));
            let variant = member.name.to_case(Case::Pascal);
            writeln!(out, r#"  {} = "{}","#, variant, value)?;
        }
        writeln!(out, "}}")?;
    }

    // Models (shared types referenced by events)
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

    // Auto-generated payload interfaces from @bus operations
    for event in events {
        writeln!(out)?;
        writeln!(out, "export interface {} {{", event.payload_type)?;
        for field in &event.fields {
            let ts_type = type_to_typescript(&field.type_ref);
            let opt = if field.optional { "?" } else { "" };
            writeln!(out, "  {}{}: {};", field.name, opt, ts_type)?;
        }
        writeln!(out, "}}")?;
    }

    Ok(out)
}

/// Generate events.ts with module augmentation.
fn generate_events(
    events: &[BusEvent],
    config: &EventBusConfig,
    file: &TypeSpecFile,
) -> Result<String, CodegenError> {
    let mut out = String::new();

    writeln!(out, "/**")?;
    writeln!(out, " * Auto-generated eventbus registry from TypeSpec.")?;
    writeln!(out, " * DO NOT EDIT.")?;
    writeln!(out, " */")?;
    writeln!(out)?;

    // Import all payload types
    let imports: Vec<&str> = events.iter().map(|e| e.payload_type.as_str()).collect();
    if !imports.is_empty() {
        // Also collect type imports referenced by payload fields
        let extra_imports = collect_type_imports(events, file);
        let mut all_imports: BTreeSet<String> = imports.iter().map(|s| s.to_string()).collect();
        all_imports.extend(extra_imports);

        writeln!(
            out,
            "import type {{ {} }} from './types';",
            all_imports.into_iter().collect::<Vec<_>>().join(", ")
        )?;
        writeln!(out)?;
    }

    writeln!(out, "declare module '{}' {{", config.module_path)?;
    writeln!(out, "  interface {} {{", config.interface_name)?;

    let mut current_bus = String::new();
    for event in events {
        if event.bus_name != current_bus {
            if !current_bus.is_empty() {
                writeln!(out)?;
            }
            writeln!(out, "    // ── {} ──", event.bus_name)?;
            current_bus.clone_from(&event.bus_name);
        }
        writeln!(out, "    '{}': {};", event.event_name, event.payload_type)?;
    }

    writeln!(out, "  }}")?;
    writeln!(out, "}}")?;

    Ok(out)
}

/// Generate the index.ts re-export file.
fn generate_index() -> Result<String, CodegenError> {
    let mut out = String::new();
    writeln!(out, "/**")?;
    writeln!(out, " * Auto-generated from TypeSpec.")?;
    writeln!(out, " */")?;
    writeln!(out)?;
    writeln!(out, "export * from './types';")?;
    writeln!(out, "import './events';")?;
    Ok(out)
}

/// Extract @bus("name") value from interface decorators.
fn get_bus_name(decorators: &[Decorator]) -> Option<String> {
    decorators
        .iter()
        .find(|d| d.name == "bus")
        .and_then(|d| d.get_string_arg(0).map(|s| s.to_string()))
}

/// Resolve operation parameters into event fields.
fn resolve_event_fields(params: &[OperationParam], models: &ModelMap<'_>) -> Vec<EventField> {
    let mut fields = Vec::new();

    for param in params {
        if param.spread {
            if let Some(name) = type_ref_name(&param.type_ref) {
                if let Some(model) = models.get(name.as_str()) {
                    for prop in &model.properties {
                        fields.push(EventField {
                            name: prop.name.clone(),
                            type_ref: prop.type_ref.clone(),
                            optional: prop.optional,
                        });
                    }
                }
            }
            continue;
        }

        fields.push(EventField {
            name: param.name.clone(),
            type_ref: param.type_ref.clone(),
            optional: param.optional,
        });
    }

    fields
}

/// Collect type names referenced in event fields that need importing.
fn collect_type_imports(events: &[BusEvent], file: &TypeSpecFile) -> BTreeSet<String> {
    let known_types: BTreeSet<String> = file
        .models()
        .map(|m| m.name.clone())
        .chain(file.enums().map(|e| e.name.clone()))
        .collect();

    let mut imports = BTreeSet::new();

    for event in events {
        for field in &event.fields {
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

/// Extract type name from a TypeRef.
fn type_ref_name(type_ref: &TypeRef) -> Option<String> {
    match type_ref {
        TypeRef::Named(name) => Some(name.clone()),
        TypeRef::Qualified(parts) => parts.last().cloned(),
        _ => None,
    }
}

/// Parse rename strategy string into a convert_case Case.
fn parse_rename_case(rename: &str) -> Case {
    match rename {
        "snake_case" => Case::Snake,
        "camelCase" => Case::Camel,
        "PascalCase" => Case::Pascal,
        "kebab-case" => Case::Kebab,
        "SCREAMING_SNAKE_CASE" => Case::ScreamingSnake,
        _ => Case::Kebab,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse;

    #[test]
    fn test_collect_eventbus_data() {
        let source = r#"
@bus("signaling")
interface SignalingBus {
    @event state(url: string, state: string): void;
    @event authOk(url: string, userId: string): void;
}
"#;

        let file = parse(source).expect("parse failed");
        let models = build_model_map(&file);
        let config = EventBusConfig {
            module_path: "@test/types".to_string(),
            interface_name: "EventRegistry".to_string(),
            rename: "kebab-case".to_string(),
        };

        let events = collect_eventbus_data(&file, &models, &config);

        assert_eq!(events.len(), 2);

        assert_eq!(events[0].event_name, "signaling:state");
        assert_eq!(events[0].payload_type, "SignalingStateEvent");
        assert_eq!(events[0].fields.len(), 2);
        assert_eq!(events[0].fields[0].name, "url");
        assert_eq!(events[0].fields[1].name, "state");

        assert_eq!(events[1].event_name, "signaling:auth-ok");
        assert_eq!(events[1].payload_type, "SignalingAuthOkEvent");
        assert_eq!(events[1].fields.len(), 2);
    }

    #[test]
    fn test_generates_payload_interfaces() {
        let source = r#"
enum WsState {
    disconnected: "disconnected",
    connected: "connected",
}

@bus("signaling")
interface SignalingBus {
    @event state(url: string, state: WsState): void;
}
"#;

        let file = parse(source).expect("parse failed");
        let config = EventBusConfig {
            module_path: "@test/types".to_string(),
            interface_name: "EventRegistry".to_string(),
            rename: "kebab-case".to_string(),
        };

        let dir = tempfile::tempdir().unwrap();
        let generated = generate(&file, dir.path(), "test", &config).unwrap();

        assert_eq!(generated.len(), 3);

        let types = std::fs::read_to_string(dir.path().join("types.ts")).unwrap();
        assert!(types.contains("export enum WsState {"));
        assert!(types.contains("export interface SignalingStateEvent {"));
        assert!(types.contains("url: string;"));
        assert!(types.contains("state: WsState;"));
    }

    #[test]
    fn test_generates_module_augmentation() {
        let source = r#"
@bus("signaling")
interface SignalingBus {
    @event state(url: string): void;
    @event authOk(url: string, userId: string): void;
}

@bus("auth")
interface AuthBus {
    @event stateChanged(user: unknown): void;
}
"#;

        let file = parse(source).expect("parse failed");
        let config = EventBusConfig {
            module_path: "@adi-family/sdk-plugin/types".to_string(),
            interface_name: "EventRegistry".to_string(),
            rename: "kebab-case".to_string(),
        };

        let dir = tempfile::tempdir().unwrap();
        generate(&file, dir.path(), "test", &config).unwrap();

        let events = std::fs::read_to_string(dir.path().join("events.ts")).unwrap();
        assert!(events.contains("declare module '@adi-family/sdk-plugin/types'"));
        assert!(events.contains("interface EventRegistry {"));
        assert!(events.contains("'signaling:state': SignalingStateEvent;"));
        assert!(events.contains("'signaling:auth-ok': SignalingAuthOkEvent;"));
        assert!(events.contains("'auth:state-changed': AuthStateChangedEvent;"));
        assert!(events.contains("// ── signaling ──"));
        assert!(events.contains("// ── auth ──"));
    }

    #[test]
    fn test_optional_fields() {
        let source = r#"
@bus("auth")
interface AuthBus {
    @event getToken(authDomain: string, sourceUrl?: string): void;
}
"#;

        let file = parse(source).expect("parse failed");
        let models = build_model_map(&file);
        let config = EventBusConfig {
            module_path: "@test/types".to_string(),
            interface_name: "EventRegistry".to_string(),
            rename: "kebab-case".to_string(),
        };

        let events = collect_eventbus_data(&file, &models, &config);
        assert_eq!(events[0].fields.len(), 2);
        assert!(!events[0].fields[0].optional);
        assert!(events[0].fields[1].optional);

        let dir = tempfile::tempdir().unwrap();
        generate(&file, dir.path(), "test", &config).unwrap();

        let types = std::fs::read_to_string(dir.path().join("types.ts")).unwrap();
        assert!(types.contains("authDomain: string;"));
        assert!(types.contains("sourceUrl?: string;"));
    }

    #[test]
    fn test_model_type_imports_in_events() {
        let source = r#"
model ConnectionInfo {
    manual_allowed: boolean;
}

model DeviceInfo {
    device_id: string;
    online: boolean;
}

@bus("signaling")
interface SignalingBus {
    @event connectionInfo(url: string, connectionInfo: ConnectionInfo): void;
    @event devices(url: string, devices: DeviceInfo[]): void;
}
"#;

        let file = parse(source).expect("parse failed");
        let config = EventBusConfig {
            module_path: "@test/types".to_string(),
            interface_name: "EventRegistry".to_string(),
            rename: "kebab-case".to_string(),
        };

        let dir = tempfile::tempdir().unwrap();
        generate(&file, dir.path(), "test", &config).unwrap();

        let events = std::fs::read_to_string(dir.path().join("events.ts")).unwrap();
        assert!(events.contains("ConnectionInfo"));
        assert!(events.contains("DeviceInfo"));

        let types = std::fs::read_to_string(dir.path().join("types.ts")).unwrap();
        assert!(types.contains("export interface ConnectionInfo {"));
        assert!(types.contains("export interface DeviceInfo {"));
    }

    #[test]
    fn test_spread_params() {
        let source = r#"
model SessionInfo {
    accessToken: string;
    email: string;
}

@bus("auth")
interface AuthBus {
    @event sessionSave(...SessionInfo, authUrl: string): void;
}
"#;

        let file = parse(source).expect("parse failed");
        let models = build_model_map(&file);
        let config = EventBusConfig {
            module_path: "@test/types".to_string(),
            interface_name: "EventRegistry".to_string(),
            rename: "kebab-case".to_string(),
        };

        let events = collect_eventbus_data(&file, &models, &config);
        assert_eq!(events[0].fields.len(), 3);
        assert_eq!(events[0].fields[0].name, "accessToken");
        assert_eq!(events[0].fields[1].name, "email");
        assert_eq!(events[0].fields[2].name, "authUrl");
    }
}
