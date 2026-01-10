//! Python language analyzer implementation.

use lib_indexer_lang_abi::{
    LocationAbi, ParsedReferenceAbi, ParsedSymbolAbi, ReferenceKindAbi, SymbolKindAbi,
    VisibilityAbi,
};
use tree_sitter::{Node, Parser, Tree};

pub fn extract_symbols(source: &str) -> Vec<ParsedSymbolAbi> {
    let tree = match parse_python(source) {
        Some(t) => t,
        None => return vec![],
    };
    let mut symbols = Vec::new();
    extract_python_symbols(tree.root_node(), source, &mut symbols);
    symbols
}

pub fn extract_references(source: &str) -> Vec<ParsedReferenceAbi> {
    let tree = match parse_python(source) {
        Some(t) => t,
        None => return vec![],
    };
    let mut refs = Vec::new();
    collect_python_references(tree.root_node(), source, &mut refs);
    refs
}

fn parse_python(source: &str) -> Option<Tree> {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_python::LANGUAGE.into())
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

fn detect_visibility(name: &str) -> VisibilityAbi {
    if name.starts_with("__") && !name.ends_with("__") {
        VisibilityAbi::Private
    } else if name.starts_with('_') {
        VisibilityAbi::Protected
    } else {
        VisibilityAbi::Public
    }
}

fn extract_python_symbols(node: Node, source: &str, symbols: &mut Vec<ParsedSymbolAbi>) {
    match node.kind() {
        "function_definition" => {
            if let Some(name) = node.child_by_field_name("name") {
                let name_text = node_text(name, source);
                let is_async = node.children(&mut node.walk()).any(|c| c.kind() == "async");
                let sig = extract_function_signature(node, source, is_async);
                symbols.push(
                    ParsedSymbolAbi::new(
                        name_text.clone(),
                        SymbolKindAbi::Function,
                        node_location(node),
                    )
                    .with_signature(sig)
                    .with_visibility(detect_visibility(&name_text)),
                );
            }
        }
        "class_definition" => {
            if let Some(name) = node.child_by_field_name("name") {
                let name_text = node_text(name, source);
                let mut children = Vec::new();
                if let Some(body) = node.child_by_field_name("body") {
                    for i in 0..body.child_count() {
                        if let Some(child) = body.child(i) {
                            if child.kind() == "function_definition" {
                                if let Some(method_name) = child.child_by_field_name("name") {
                                    let method_name_text = node_text(method_name, source);
                                    children.push(
                                        ParsedSymbolAbi::new(
                                            method_name_text.clone(),
                                            SymbolKindAbi::Method,
                                            node_location(child),
                                        )
                                        .with_visibility(detect_visibility(&method_name_text)),
                                    );
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
        "decorated_definition" => {
            if let Some(def) = node.child_by_field_name("definition") {
                extract_python_symbols(def, source, symbols);
            }
        }
        _ => {
            for i in 0..node.child_count() {
                if let Some(child) = node.child(i) {
                    extract_python_symbols(child, source, symbols);
                }
            }
        }
    }
}

fn extract_function_signature(node: Node, source: &str, is_async: bool) -> String {
    let name = node
        .child_by_field_name("name")
        .map(|n| node_text(n, source))
        .unwrap_or_default();
    let params = node
        .child_by_field_name("parameters")
        .map(|n| node_text(n, source))
        .unwrap_or_else(|| "()".to_string());
    let ret = node
        .child_by_field_name("return_type")
        .map(|n| format!(" -> {}", node_text(n, source)))
        .unwrap_or_default();
    let prefix = if is_async { "async def " } else { "def " };
    format!("{}{}{}{}", prefix, name, params, ret)
}

fn collect_python_references(node: Node, source: &str, refs: &mut Vec<ParsedReferenceAbi>) {
    match node.kind() {
        "call" => {
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
        "import_statement" | "import_from_statement" => {
            if let Some(module) = node.child_by_field_name("module_name") {
                refs.push(ParsedReferenceAbi::new(
                    node_text(module, source),
                    ReferenceKindAbi::Import,
                    node_location(module),
                ));
            }
        }
        "class_definition" => {
            if let Some(superclasses) = node.child_by_field_name("superclasses") {
                for i in 0..superclasses.child_count() {
                    if let Some(child) = superclasses.child(i) {
                        if child.kind() == "identifier" || child.kind() == "attribute" {
                            refs.push(ParsedReferenceAbi::new(
                                node_text(child, source),
                                ReferenceKindAbi::Inheritance,
                                node_location(child),
                            ));
                        }
                    }
                }
            }
        }
        "attribute" => {
            if let Some(attr) = node.child_by_field_name("attribute") {
                refs.push(ParsedReferenceAbi::new(
                    node_text(attr, source),
                    ReferenceKindAbi::FieldAccess,
                    node_location(attr),
                ));
            }
        }
        _ => {}
    }
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            collect_python_references(child, source, refs);
        }
    }
}

fn is_builtin(name: &str) -> bool {
    matches!(
        name,
        "print"
            | "len"
            | "range"
            | "enumerate"
            | "zip"
            | "map"
            | "filter"
            | "sum"
            | "min"
            | "max"
            | "abs"
            | "sorted"
            | "reversed"
            | "open"
            | "input"
            | "int"
            | "float"
            | "str"
            | "bool"
            | "list"
            | "dict"
            | "set"
            | "tuple"
            | "type"
            | "isinstance"
            | "hasattr"
            | "getattr"
            | "setattr"
            | "super"
    )
}
