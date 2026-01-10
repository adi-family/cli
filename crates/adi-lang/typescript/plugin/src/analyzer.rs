//! TypeScript/JavaScript language analyzer implementation.

use lib_indexer_lang_abi::{
    LocationAbi, ParsedReferenceAbi, ParsedSymbolAbi, ReferenceKindAbi, SymbolKindAbi,
    VisibilityAbi,
};
use tree_sitter::{Node, Parser, Tree};

pub fn extract_symbols(source: &str) -> Vec<ParsedSymbolAbi> {
    let tree = match parse_typescript(source) {
        Some(t) => t,
        None => return vec![],
    };
    let mut symbols = Vec::new();
    extract_ts_symbols(tree.root_node(), source, &mut symbols);
    symbols
}

pub fn extract_references(source: &str) -> Vec<ParsedReferenceAbi> {
    let tree = match parse_typescript(source) {
        Some(t) => t,
        None => return vec![],
    };
    let mut refs = Vec::new();
    collect_ts_references(tree.root_node(), source, &mut refs);
    refs
}

fn parse_typescript(source: &str) -> Option<Tree> {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into())
        .ok()?;
    parser.parse(source, None)
}

fn node_text(node: Node, source: &str) -> String {
    source[node.byte_range()].to_string()
}

fn node_location(node: Node) -> LocationAbi {
    let start = node.start_position();
    let end = node.end_position();
    LocationAbi::new(
        start.row as u32,
        start.column as u32,
        end.row as u32,
        end.column as u32,
        node.start_byte() as u32,
        node.end_byte() as u32,
    )
}

fn extract_ts_symbols(node: Node, source: &str, symbols: &mut Vec<ParsedSymbolAbi>) {
    match node.kind() {
        "function_declaration" | "function" => {
            if let Some(name) = node.child_by_field_name("name") {
                let name_text = node_text(name, source);
                let sig = extract_function_signature(node, source);
                symbols.push(
                    ParsedSymbolAbi::new(name_text, SymbolKindAbi::Function, node_location(node))
                        .with_signature(sig),
                );
            }
        }
        "class_declaration" | "class" => {
            if let Some(name) = node.child_by_field_name("name") {
                let name_text = node_text(name, source);
                let mut children = Vec::new();
                if let Some(body) = node.child_by_field_name("body") {
                    for i in 0..body.child_count() {
                        if let Some(child) = body.child(i) {
                            if child.kind() == "method_definition" {
                                if let Some(method_name) = child.child_by_field_name("name") {
                                    children.push(ParsedSymbolAbi::new(
                                        node_text(method_name, source),
                                        SymbolKindAbi::Method,
                                        node_location(child),
                                    ));
                                }
                            }
                        }
                    }
                }
                symbols.push(
                    ParsedSymbolAbi::new(name_text, SymbolKindAbi::Class, node_location(node))
                        .with_children(children),
                );
            }
        }
        "interface_declaration" => {
            if let Some(name) = node.child_by_field_name("name") {
                symbols.push(ParsedSymbolAbi::new(
                    node_text(name, source),
                    SymbolKindAbi::Interface,
                    node_location(node),
                ));
            }
        }
        "type_alias_declaration" => {
            if let Some(name) = node.child_by_field_name("name") {
                symbols.push(ParsedSymbolAbi::new(
                    node_text(name, source),
                    SymbolKindAbi::Type,
                    node_location(node),
                ));
            }
        }
        "enum_declaration" => {
            if let Some(name) = node.child_by_field_name("name") {
                symbols.push(ParsedSymbolAbi::new(
                    node_text(name, source),
                    SymbolKindAbi::Enum,
                    node_location(node),
                ));
            }
        }
        _ => {
            for i in 0..node.child_count() {
                if let Some(child) = node.child(i) {
                    extract_ts_symbols(child, source, symbols);
                }
            }
        }
    }
}

fn extract_function_signature(node: Node, source: &str) -> String {
    let mut parts = Vec::new();
    if let Some(name) = node.child_by_field_name("name") {
        parts.push(node_text(name, source));
    }
    if let Some(params) = node.child_by_field_name("parameters") {
        parts.push(node_text(params, source));
    }
    if let Some(ret) = node.child_by_field_name("return_type") {
        parts.push(format!(": {}", node_text(ret, source)));
    }
    parts.join("")
}

fn collect_ts_references(node: Node, source: &str, refs: &mut Vec<ParsedReferenceAbi>) {
    match node.kind() {
        "call_expression" => {
            if let Some(func) = node.child_by_field_name("function") {
                let name = node_text(func, source);
                if !is_builtin(&name) {
                    refs.push(ParsedReferenceAbi::new(
                        name,
                        ReferenceKindAbi::Call,
                        node_location(func),
                    ));
                }
            }
        }
        "import_statement" => {
            if let Some(source_node) = node.child_by_field_name("source") {
                let module = node_text(source_node, source)
                    .trim_matches(|c| c == '"' || c == '\'')
                    .to_string();
                refs.push(ParsedReferenceAbi::new(
                    module,
                    ReferenceKindAbi::Import,
                    node_location(source_node),
                ));
            }
        }
        "type_identifier" => {
            let name = node_text(node, source);
            if !is_primitive(&name) {
                refs.push(ParsedReferenceAbi::new(
                    name,
                    ReferenceKindAbi::TypeReference,
                    node_location(node),
                ));
            }
        }
        "member_expression" => {
            if let Some(prop) = node.child_by_field_name("property") {
                refs.push(ParsedReferenceAbi::new(
                    node_text(prop, source),
                    ReferenceKindAbi::FieldAccess,
                    node_location(prop),
                ));
            }
        }
        _ => {}
    }
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            collect_ts_references(child, source, refs);
        }
    }
}

fn is_builtin(name: &str) -> bool {
    matches!(
        name,
        "console.log"
            | "console.error"
            | "JSON.parse"
            | "JSON.stringify"
            | "Object.keys"
            | "Array.isArray"
            | "Promise.all"
            | "Promise.resolve"
    )
}

fn is_primitive(name: &str) -> bool {
    matches!(
        name,
        "string"
            | "number"
            | "boolean"
            | "any"
            | "void"
            | "null"
            | "undefined"
            | "never"
            | "unknown"
            | "object"
    )
}
