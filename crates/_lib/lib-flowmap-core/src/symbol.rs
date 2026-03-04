use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Unique identifier for a symbol (class, method, property)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SymbolId(pub String);

impl SymbolId {
    pub fn class(file: &str, name: &str) -> Self {
        Self(format!("{}::{}", file, name))
    }

    pub fn method(file: &str, class: &str, method: &str) -> Self {
        Self(format!("{}::{}::{}", file, class, method))
    }

    pub fn property(file: &str, class: &str, prop: &str) -> Self {
        Self(format!("{}::{}#{}", file, class, prop))
    }
}

/// Information about a class in the codebase
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassInfo {
    pub id: SymbolId,
    pub name: String,
    pub file_path: String,
    pub kind: ClassKind,
    pub decorators: Vec<Decorator>,
    pub methods: Vec<MethodInfo>,
    pub injections: Vec<InjectionInfo>,
    pub start_line: u32,
    pub end_line: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClassKind {
    Controller,
    Service,
    Repository,
    Guard,
    Pipe,
    Middleware,
    Entity,
    Other,
}

/// A decorator on a class or method
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Decorator {
    pub name: String,
    pub args: Vec<String>,
}

impl Decorator {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            args: vec![],
        }
    }

    pub fn with_arg(mut self, arg: &str) -> Self {
        self.args.push(arg.to_string());
        self
    }
}

/// Information about a method
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MethodInfo {
    pub id: SymbolId,
    pub name: String,
    pub is_async: bool,
    pub is_private: bool,
    pub decorators: Vec<Decorator>,
    pub parameters: Vec<ParameterInfo>,
    pub start_line: u32,
    pub end_line: u32,
}

/// Information about a method parameter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterInfo {
    pub name: String,
    pub type_name: Option<String>,
    pub decorators: Vec<Decorator>,
}

/// Information about an injected dependency
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InjectionInfo {
    pub property_name: String,
    pub type_name: String,
    pub kind: InjectionKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InjectionKind {
    /// @Inject(ServiceClass)
    Inject,
    /// @InjectRepository(Entity)
    InjectRepository,
    /// Constructor parameter
    Constructor,
}

/// Index of all symbols in a codebase
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SymbolIndex {
    /// All classes indexed by SymbolId
    pub classes: HashMap<SymbolId, ClassInfo>,
    /// Quick lookup: class name -> SymbolId (may have duplicates across files)
    pub class_by_name: HashMap<String, Vec<SymbolId>>,
    /// Quick lookup: method -> (class SymbolId, MethodInfo)
    pub methods: HashMap<SymbolId, (SymbolId, MethodInfo)>,
    /// Import resolution: file -> (import name -> source file)
    pub imports: HashMap<String, HashMap<String, ImportInfo>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportInfo {
    pub source: String,
    pub original_name: Option<String>,
}

impl SymbolIndex {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_class(&mut self, class: ClassInfo) {
        let name = class.name.clone();
        let id = class.id.clone();

        // Index methods
        for method in &class.methods {
            self.methods
                .insert(method.id.clone(), (id.clone(), method.clone()));
        }

        // Add to name lookup
        self.class_by_name
            .entry(name)
            .or_default()
            .push(id.clone());

        self.classes.insert(id, class);
    }

    pub fn add_import(&mut self, file: &str, name: &str, info: ImportInfo) {
        self.imports
            .entry(file.to_string())
            .or_default()
            .insert(name.to_string(), info);
    }

    /// Find a class by name, optionally scoped to imports from a file
    pub fn resolve_class(&self, from_file: &str, name: &str) -> Option<&ClassInfo> {
        // First check imports from this file
        if let Some(imports) = self.imports.get(from_file) {
            if let Some(import_info) = imports.get(name) {
                // Find class in the imported file
                let original_name = import_info.original_name.as_deref().unwrap_or(name);
                if let Some(ids) = self.class_by_name.get(original_name) {
                    for id in ids {
                        if let Some(class) = self.classes.get(id) {
                            // Check if source matches (simplified path matching)
                            if import_info.source.contains(&class.name)
                                || class.file_path.contains(&import_info.source)
                            {
                                return Some(class);
                            }
                        }
                    }
                    // Fallback: return first match
                    return ids.first().and_then(|id| self.classes.get(id));
                }
            }
        }

        // Fallback: direct name lookup
        self.class_by_name
            .get(name)
            .and_then(|ids| ids.first())
            .and_then(|id| self.classes.get(id))
    }

    /// Find a method by class and method name
    pub fn resolve_method<'a>(&self, class: &'a ClassInfo, method_name: &str) -> Option<&'a MethodInfo> {
        class.methods.iter().find(|m| m.name == method_name)
    }

    /// Get all HTTP endpoints (controllers with @Get/@Post/etc methods)
    pub fn http_endpoints(&self) -> Vec<HttpEndpoint> {
        let mut endpoints = vec![];

        for class in self.classes.values() {
            if !matches!(class.kind, ClassKind::Controller) {
                continue;
            }

            // Get controller path prefix from @Controller decorator
            let prefix = class
                .decorators
                .iter()
                .find(|d| d.name == "Controller")
                .and_then(|d| d.args.first())
                .map(|s| s.trim_matches(|c| c == '"' || c == '\'').to_string())
                .unwrap_or_default();

            // Get class-level guards
            let class_guards: Vec<String> = class
                .decorators
                .iter()
                .filter(|d| d.name == "UseGuards")
                .flat_map(|d| d.args.clone())
                .collect();

            for method in &class.methods {
                // Check for HTTP method decorators
                for decorator in &method.decorators {
                    let http_method = match decorator.name.as_str() {
                        "Get" => Some("GET"),
                        "Post" => Some("POST"),
                        "Put" => Some("PUT"),
                        "Delete" => Some("DELETE"),
                        "Patch" => Some("PATCH"),
                        _ => None,
                    };

                    if let Some(http_method) = http_method {
                        let path_suffix = decorator
                            .args
                            .first()
                            .map(|s| s.trim_matches(|c| c == '"' || c == '\'').to_string())
                            .unwrap_or_default();

                        let full_path = normalize_path(&prefix, &path_suffix);

                        // Collect method-level guards
                        let method_guards: Vec<String> = method
                            .decorators
                            .iter()
                            .filter(|d| d.name == "UseGuards")
                            .flat_map(|d| d.args.clone())
                            .collect();

                        endpoints.push(HttpEndpoint {
                            method: http_method.to_string(),
                            path: full_path,
                            controller_class: class.id.clone(),
                            controller_method: method.id.clone(),
                            guards: [class_guards.clone(), method_guards].concat(),
                            file_path: class.file_path.clone(),
                            line: method.start_line,
                        });
                    }
                }
            }
        }

        endpoints
    }
}

fn normalize_path(prefix: &str, suffix: &str) -> String {
    let prefix = prefix.trim_matches('/');
    let suffix = suffix.trim_matches('/');

    if prefix.is_empty() && suffix.is_empty() {
        "/".to_string()
    } else if prefix.is_empty() {
        format!("/{}", suffix)
    } else if suffix.is_empty() {
        format!("/{}", prefix)
    } else {
        format!("/{}/{}", prefix, suffix)
    }
}

/// An HTTP endpoint extracted from the codebase
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpEndpoint {
    pub method: String,
    pub path: String,
    pub controller_class: SymbolId,
    pub controller_method: SymbolId,
    pub guards: Vec<String>,
    pub file_path: String,
    pub line: u32,
}
