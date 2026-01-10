//! Go language analyzer implementation.

use lib_indexer_lang_abi::{
    LocationAbi, ParsedReferenceAbi, ParsedSymbolAbi, ReferenceKindAbi, SymbolKindAbi,
    VisibilityAbi,
};
use tree_sitter::{Node, Parser, Tree};

pub fn extract_symbols(source: &str) -> Vec<ParsedSymbolAbi> {
    let tree = match parse_go(source) {
        Some(t) => t,
        None => return vec![],
    };
    let mut symbols = Vec::new();
    extract_go_symbols(tree.root_node(), source, &mut symbols);
    symbols
}

pub fn extract_references(source: &str) -> Vec<ParsedReferenceAbi> {
    let tree = match parse_go(source) {
        Some(t) => t,
        None => return vec![],
    };
    let mut refs = Vec::new();
    collect_go_references(tree.root_node(), source, &mut refs);
    refs
}

fn parse_go(source: &str) -> Option<Tree> {
    let mut parser = Parser::new();
    parser.set_language(&tree_sitter_go::LANGUAGE.into()).ok()?;
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
    if name
        .chars()
        .next()
        .map(|c| c.is_uppercase())
        .unwrap_or(false)
    {
        VisibilityAbi::Public
    } else {
        VisibilityAbi::Private
    }
}

fn extract_go_symbols(node: Node, source: &str, symbols: &mut Vec<ParsedSymbolAbi>) {
    match node.kind() {
        "function_declaration" => {
            if let Some(name) = node.child_by_field_name("name") {
                let name_text = node_text(name, source);
                let sig = extract_function_signature(node, source);
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
        "method_declaration" => {
            if let Some(name) = node.child_by_field_name("name") {
                let name_text = node_text(name, source);
                let receiver = node
                    .child_by_field_name("receiver")
                    .and_then(|r| r.child(1))
                    .map(|n| node_text(n, source))
                    .unwrap_or_default();
                let full_name = if receiver.is_empty() {
                    name_text.clone()
                } else {
                    format!("{}.{}", receiver, name_text)
                };
                symbols.push(
                    ParsedSymbolAbi::new(full_name, SymbolKindAbi::Method, node_location(node))
                        .with_visibility(detect_visibility(&name_text)),
                );
            }
        }
        "type_declaration" => {
            for i in 0..node.child_count() {
                if let Some(child) = node.child(i) {
                    if child.kind() == "type_spec" {
                        if let Some(name) = child.child_by_field_name("name") {
                            let name_text = node_text(name, source);
                            let type_node = child.child_by_field_name("type");
                            let kind = match type_node.map(|t| t.kind()) {
                                Some("struct_type") => SymbolKindAbi::Struct,
                                Some("interface_type") => SymbolKindAbi::Interface,
                                _ => SymbolKindAbi::Type,
                            };
                            let mut children = Vec::new();
                            if let Some(type_def) = type_node {
                                if type_def.kind() == "struct_type" {
                                    if let Some(fields) = type_def.child_by_field_name("fields") {
                                        for j in 0..fields.child_count() {
                                            if let Some(field) = fields.child(j) {
                                                if field.kind() == "field_declaration" {
                                                    if let Some(field_name) =
                                                        field.child_by_field_name("name")
                                                    {
                                                        children.push(ParsedSymbolAbi::new(
                                                            node_text(field_name, source),
                                                            SymbolKindAbi::Field,
                                                            node_location(field),
                                                        ));
                                                    }
                                                }
                                            }
                                        }
                                    }
                                } else if type_def.kind() == "interface_type" {
                                    for j in 0..type_def.child_count() {
                                        if let Some(member) = type_def.child(j) {
                                            if member.kind() == "method_spec" {
                                                if let Some(method_name) =
                                                    member.child_by_field_name("name")
                                                {
                                                    children.push(ParsedSymbolAbi::new(
                                                        node_text(method_name, source),
                                                        SymbolKindAbi::Method,
                                                        node_location(member),
                                                    ));
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            symbols.push(
                                ParsedSymbolAbi::new(name_text.clone(), kind, node_location(child))
                                    .with_visibility(detect_visibility(&name_text))
                                    .with_children(children),
                            );
                        }
                    }
                }
            }
        }
        "const_declaration" | "var_declaration" => {
            for i in 0..node.child_count() {
                if let Some(child) = node.child(i) {
                    if child.kind() == "const_spec" || child.kind() == "var_spec" {
                        if let Some(name) = child.child_by_field_name("name") {
                            let name_text = node_text(name, source);
                            let kind = if node.kind() == "const_declaration" {
                                SymbolKindAbi::Constant
                            } else {
                                SymbolKindAbi::Variable
                            };
                            symbols.push(
                                ParsedSymbolAbi::new(name_text.clone(), kind, node_location(child))
                                    .with_visibility(detect_visibility(&name_text)),
                            );
                        }
                    }
                }
            }
        }
        _ => {
            for i in 0..node.child_count() {
                if let Some(child) = node.child(i) {
                    extract_go_symbols(child, source, symbols);
                }
            }
        }
    }
}

fn extract_function_signature(node: Node, source: &str) -> String {
    let name = node
        .child_by_field_name("name")
        .map(|n| node_text(n, source))
        .unwrap_or_default();
    let params = node
        .child_by_field_name("parameters")
        .map(|n| node_text(n, source))
        .unwrap_or_else(|| "()".to_string());
    let result = node
        .child_by_field_name("result")
        .map(|n| format!(" {}", node_text(n, source)))
        .unwrap_or_default();
    format!("func {}{}{}", name, params, result)
}

fn collect_go_references(node: Node, source: &str, refs: &mut Vec<ParsedReferenceAbi>) {
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
        "import_declaration" => {
            for i in 0..node.child_count() {
                if let Some(child) = node.child(i) {
                    if child.kind() == "import_spec" {
                        if let Some(path) = child.child_by_field_name("path") {
                            let import_path = node_text(path, source).trim_matches('"').to_string();
                            refs.push(ParsedReferenceAbi::new(
                                import_path,
                                ReferenceKindAbi::Import,
                                node_location(path),
                            ));
                        }
                    } else if child.kind() == "interpreted_string_literal" {
                        let import_path = node_text(child, source).trim_matches('"').to_string();
                        refs.push(ParsedReferenceAbi::new(
                            import_path,
                            ReferenceKindAbi::Import,
                            node_location(child),
                        ));
                    }
                }
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
        "selector_expression" => {
            if let Some(field) = node.child_by_field_name("field") {
                refs.push(ParsedReferenceAbi::new(
                    node_text(field, source),
                    ReferenceKindAbi::FieldAccess,
                    node_location(field),
                ));
            }
        }
        _ => {}
    }
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            collect_go_references(child, source, refs);
        }
    }
}

fn is_builtin(name: &str) -> bool {
    matches!(
        name,
        "fmt.Println"
            | "fmt.Printf"
            | "fmt.Sprintf"
            | "make"
            | "new"
            | "append"
            | "len"
            | "cap"
            | "copy"
            | "delete"
            | "close"
            | "panic"
            | "recover"
            | "print"
            | "println"
    )
}

fn is_primitive(name: &str) -> bool {
    matches!(
        name,
        "int"
            | "int8"
            | "int16"
            | "int32"
            | "int64"
            | "uint"
            | "uint8"
            | "uint16"
            | "uint32"
            | "uint64"
            | "uintptr"
            | "float32"
            | "float64"
            | "complex64"
            | "complex128"
            | "string"
            | "bool"
            | "byte"
            | "rune"
            | "error"
            | "any"
    )
}
