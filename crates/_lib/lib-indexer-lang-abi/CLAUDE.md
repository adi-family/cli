# lib-indexer-lang-abi

FFI-safe types and service contract for ADI indexer language plugins.

## Adding a New Language Plugin

### 1. Create the crate

```bash
cargo new --lib adi-lang-<language>
```

### 2. Configure Cargo.toml

```toml
[package]
name = "adi-lang-<language>"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
lib-plugin-abi = { path = "../lib-plugin-abi" }
lib-indexer-lang-abi = { path = "../lib-indexer-lang-abi" }
abi_stable = "0.11"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tree-sitter = "0.25"
tree-sitter-<language> = "<version>"  # Find on crates.io
once_cell = "1.19"
```

### 3. Create plugin.toml

```toml
[plugin]
id = "adi.lang.<language>"
name = "<Language> Language Support"
version = "0.1.0"
api_version = 2

[provides]
services = ["adi.indexer.lang.<language>"]
```

### 4. Create src/lib.rs

```rust
use abi_stable::std_types::{RString, RVec};
use lib_plugin_abi::{PluginDeclaration, PluginVTable, ServiceHandle, ServiceVTable, PLUGIN_ABI_VERSION};
use once_cell::sync::Lazy;
use std::ffi::CStr;

mod analyzer;

static PLUGIN_INFO: Lazy<PluginDeclaration> = Lazy::new(|| PluginDeclaration {
    abi_version: PLUGIN_ABI_VERSION,
    plugin_id: RString::from("adi.lang.<language>"),
    plugin_version: RString::from("0.1.0"),
    api_version: 2,
});

#[no_mangle]
pub extern "C" fn plugin_declaration() -> &'static PluginDeclaration {
    &PLUGIN_INFO
}

#[no_mangle]
pub extern "C" fn plugin_vtable() -> PluginVTable {
    PluginVTable {
        init: plugin_init,
        shutdown: plugin_shutdown,
        get_services: get_services,
    }
}

extern "C" fn plugin_init(_host: *const std::ffi::c_void) -> i32 { 0 }
extern "C" fn plugin_shutdown() {}

extern "C" fn get_services() -> RVec<ServiceHandle> {
    let handle = ServiceHandle {
        service_id: RString::from("adi.indexer.lang.<language>"),
        version: RString::from("1.0.0"),
        vtable: ServiceVTable {
            invoke: service_invoke,
            get_metadata: service_metadata,
        },
    };
    RVec::from(vec![handle])
}

extern "C" fn service_metadata() -> RString {
    RString::from(r#"{"name":"<Language> Analyzer","methods":["get_grammar","extract_symbols","extract_references","get_info"]}"#)
}

extern "C" fn service_invoke(method: *const i8, args: *const i8) -> RString {
    let method = unsafe { CStr::from_ptr(method).to_str().unwrap_or("") };
    let args = unsafe { CStr::from_ptr(args).to_str().unwrap_or("{}") };

    let result = match method {
        "get_grammar" => analyzer::get_grammar(),
        "extract_symbols" => analyzer::extract_symbols(args),
        "extract_references" => analyzer::extract_references(args),
        "get_info" => analyzer::get_info(),
        _ => r#"{"error":"unknown method"}"#.to_string(),
    };
    RString::from(result)
}
```

### 5. Create src/analyzer.rs

```rust
use lib_indexer_lang_abi::{LocationAbi, ParsedReferenceAbi, ParsedSymbolAbi, ReferenceKindAbi, SymbolKindAbi, VisibilityAbi};
use tree_sitter::{Language, Parser};

extern "C" { fn tree_sitter_<language>() -> Language; }

pub fn get_grammar() -> String {
    let lang = unsafe { tree_sitter_<language>() };
    let ptr = &lang as *const Language as usize;
    format!(r#"{{"language_ptr":{}}}"#, ptr)
}

pub fn get_info() -> String {
    r#"{"language":"<language>","extensions":["<ext>"],"version":"0.1.0"}"#.to_string()
}

pub fn extract_symbols(args: &str) -> String {
    let parsed: serde_json::Value = serde_json::from_str(args).unwrap_or_default();
    let source = parsed["source"].as_str().unwrap_or("");

    let mut parser = Parser::new();
    let lang = unsafe { tree_sitter_<language>() };
    parser.set_language(&lang).ok();

    let tree = match parser.parse(source, None) {
        Some(t) => t,
        None => return "[]".to_string(),
    };

    let symbols = extract_symbols_from_tree(source, &tree);
    serde_json::to_string(&symbols).unwrap_or("[]".to_string())
}

pub fn extract_references(args: &str) -> String {
    let parsed: serde_json::Value = serde_json::from_str(args).unwrap_or_default();
    let source = parsed["source"].as_str().unwrap_or("");

    let mut parser = Parser::new();
    let lang = unsafe { tree_sitter_<language>() };
    parser.set_language(&lang).ok();

    let tree = match parser.parse(source, None) {
        Some(t) => t,
        None => return "[]".to_string(),
    };

    let refs = extract_refs_from_tree(source, &tree);
    serde_json::to_string(&refs).unwrap_or("[]".to_string())
}

fn extract_symbols_from_tree(source: &str, tree: &tree_sitter::Tree) -> Vec<ParsedSymbolAbi> {
    let mut symbols = Vec::new();
    let mut cursor = tree.walk();

    // Walk the tree and extract symbols based on node types
    // Example for functions:
    visit_nodes(&mut cursor, source, &mut symbols);

    symbols
}

fn visit_nodes(cursor: &mut tree_sitter::TreeCursor, source: &str, symbols: &mut Vec<ParsedSymbolAbi>) {
    loop {
        let node = cursor.node();

        // Match node types specific to your language
        // Example: "function_definition", "class_definition", etc.
        match node.kind() {
            "function_definition" | "function_item" => {
                if let Some(name_node) = node.child_by_field_name("name") {
                    let name = &source[name_node.byte_range()];
                    symbols.push(ParsedSymbolAbi {
                        name: name.to_string(),
                        kind: SymbolKindAbi::Function,
                        visibility: VisibilityAbi::Public,
                        location: LocationAbi {
                            start_byte: node.start_byte(),
                            end_byte: node.end_byte(),
                            start_line: node.start_position().row,
                            start_col: node.start_position().column,
                            end_line: node.end_position().row,
                            end_col: node.end_position().column,
                        },
                        signature: None,
                        doc_comment: None,
                        children: vec![],
                    });
                }
            }
            // Add more node types...
            _ => {}
        }

        if cursor.goto_first_child() {
            visit_nodes(cursor, source, symbols);
            cursor.goto_parent();
        }

        if !cursor.goto_next_sibling() {
            break;
        }
    }
}

fn extract_refs_from_tree(source: &str, tree: &tree_sitter::Tree) -> Vec<ParsedReferenceAbi> {
    // Similar pattern - walk tree and extract references
    vec![]
}
```

### 6. Add to workspace

In the root `Cargo.toml`:

```toml
[workspace]
members = [
    # ...
    "crates/adi-lang-<language>",
]
```

### 7. Build and test

```bash
cargo build -p adi-lang-<language>
```

## Service Contract

| Method | Args | Returns |
|--------|------|---------|
| `get_grammar` | `{}` | `{"language_ptr": <usize>}` |
| `get_info` | `{}` | `{"language": "...", "extensions": [...], "version": "..."}` |
| `extract_symbols` | `{"source": "..."}` | `[ParsedSymbolAbi, ...]` |
| `extract_references` | `{"source": "..."}` | `[ParsedReferenceAbi, ...]` |

## Types

See `src/types.rs` for FFI-safe type definitions:
- `ParsedSymbolAbi` - function, class, struct, etc.
- `ParsedReferenceAbi` - call, import, type reference
- `SymbolKindAbi` - Function, Class, Struct, Enum, etc.
- `ReferenceKindAbi` - Call, Import, TypeReference, etc.
