//! C/C++ language analyzer implementation.

use lib_indexer_lang_abi::{
    LocationAbi, ParsedReferenceAbi, ParsedSymbolAbi, ReferenceKindAbi, SymbolKindAbi,
};
use tree_sitter::{Node, Parser, Tree};

pub fn extract_symbols(source: &str, is_cpp: bool) -> Vec<ParsedSymbolAbi> {
    let tree = match parse(source, is_cpp) {
        Some(t) => t,
        None => return vec![],
    };
    let mut symbols = Vec::new();
    extract_cpp_symbols(tree.root_node(), source, &mut symbols);
    symbols
}

pub fn extract_references(source: &str, is_cpp: bool) -> Vec<ParsedReferenceAbi> {
    let tree = match parse(source, is_cpp) {
        Some(t) => t,
        None => return vec![],
    };
    let mut refs = Vec::new();
    collect_cpp_references(tree.root_node(), source, &mut refs);
    refs
}

fn parse(source: &str, is_cpp: bool) -> Option<Tree> {
    let mut parser = Parser::new();
    let lang = if is_cpp {
        tree_sitter_cpp::LANGUAGE.into()
    } else {
        tree_sitter_c::LANGUAGE.into()
    };
    parser.set_language(&lang).ok()?;
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

fn extract_cpp_symbols(node: Node, source: &str, symbols: &mut Vec<ParsedSymbolAbi>) {
    match node.kind() {
        "function_definition" => {
            if let Some(declarator) = node.child_by_field_name("declarator") {
                if let Some(name) = extract_function_name(declarator, source) {
                    let sig = extract_signature(node, source);
                    symbols.push(
                        ParsedSymbolAbi::new(name, SymbolKindAbi::Function, node_location(node))
                            .with_signature(sig),
                    );
                }
            }
        }
        "class_specifier" => {
            if let Some(name) = node.child_by_field_name("name") {
                let mut children = Vec::new();
                if let Some(body) = node.child_by_field_name("body") {
                    collect_class_members(body, source, &mut children);
                }
                symbols.push(
                    ParsedSymbolAbi::new(
                        node_text(name, source),
                        SymbolKindAbi::Class,
                        node_location(node),
                    )
                    .with_children(children),
                );
            }
        }
        "struct_specifier" => {
            if let Some(name) = node.child_by_field_name("name") {
                let mut children = Vec::new();
                if let Some(body) = node.child_by_field_name("body") {
                    collect_class_members(body, source, &mut children);
                }
                symbols.push(
                    ParsedSymbolAbi::new(
                        node_text(name, source),
                        SymbolKindAbi::Struct,
                        node_location(node),
                    )
                    .with_children(children),
                );
            }
        }
        "enum_specifier" => {
            if let Some(name) = node.child_by_field_name("name") {
                symbols.push(ParsedSymbolAbi::new(
                    node_text(name, source),
                    SymbolKindAbi::Enum,
                    node_location(node),
                ));
            }
        }
        "namespace_definition" => {
            if let Some(name) = node.child_by_field_name("name") {
                symbols.push(ParsedSymbolAbi::new(
                    node_text(name, source),
                    SymbolKindAbi::Namespace,
                    node_location(node),
                ));
            }
        }
        "template_declaration" => {
            if let Some(declaration) = node.child_by_field_name("declaration") {
                extract_cpp_symbols(declaration, source, symbols);
            }
        }
        _ => {
            for i in 0..node.child_count() {
                if let Some(child) = node.child(i) {
                    extract_cpp_symbols(child, source, symbols);
                }
            }
        }
    }
}

fn collect_class_members(body: Node, source: &str, children: &mut Vec<ParsedSymbolAbi>) {
    for i in 0..body.child_count() {
        if let Some(child) = body.child(i) {
            match child.kind() {
                "function_definition" | "declaration" => {
                    if let Some(declarator) = child.child_by_field_name("declarator") {
                        if let Some(name) = extract_function_name(declarator, source) {
                            children.push(ParsedSymbolAbi::new(
                                name,
                                SymbolKindAbi::Method,
                                node_location(child),
                            ));
                        }
                    }
                }
                "field_declaration" => {
                    for j in 0..child.child_count() {
                        if let Some(field) = child.child(j) {
                            if field.kind() == "field_identifier" {
                                children.push(ParsedSymbolAbi::new(
                                    node_text(field, source),
                                    SymbolKindAbi::Field,
                                    node_location(field),
                                ));
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }
}

fn extract_function_name(declarator: Node, source: &str) -> Option<String> {
    match declarator.kind() {
        "function_declarator" => {
            if let Some(decl) = declarator.child_by_field_name("declarator") {
                return extract_function_name(decl, source);
            }
        }
        "pointer_declarator" | "reference_declarator" => {
            if let Some(decl) = declarator.child_by_field_name("declarator") {
                return extract_function_name(decl, source);
            }
        }
        "qualified_identifier" => {
            if let Some(name) = declarator.child_by_field_name("name") {
                return Some(node_text(name, source));
            }
        }
        "identifier" | "field_identifier" | "destructor_name" | "operator_name" => {
            return Some(node_text(declarator, source));
        }
        _ => {}
    }
    None
}

fn extract_signature(node: Node, source: &str) -> String {
    let text = node_text(node, source);
    if let Some(brace) = text.find('{') {
        text[..brace].trim().to_string()
    } else if let Some(semi) = text.find(';') {
        text[..semi].trim().to_string()
    } else {
        text.lines()
            .take(3)
            .collect::<Vec<_>>()
            .join(" ")
            .trim()
            .to_string()
    }
}

fn collect_cpp_references(node: Node, source: &str, refs: &mut Vec<ParsedReferenceAbi>) {
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
        "preproc_include" => {
            if let Some(path) = node.child_by_field_name("path") {
                let include = node_text(path, source)
                    .trim_matches(|c| c == '"' || c == '<' || c == '>')
                    .to_string();
                refs.push(ParsedReferenceAbi::new(
                    include,
                    ReferenceKindAbi::Import,
                    node_location(path),
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
        "field_expression" => {
            if let Some(field) = node.child_by_field_name("field") {
                refs.push(ParsedReferenceAbi::new(
                    node_text(field, source),
                    ReferenceKindAbi::FieldAccess,
                    node_location(field),
                ));
            }
        }
        "class_specifier" | "struct_specifier" => {
            if let Some(base) = node.child_by_field_name("base_clause") {
                for i in 0..base.child_count() {
                    if let Some(child) = base.child(i) {
                        if child.kind() == "base_class_clause" {
                            if let Some(type_node) = child.child_by_field_name("type") {
                                refs.push(ParsedReferenceAbi::new(
                                    node_text(type_node, source),
                                    ReferenceKindAbi::Inheritance,
                                    node_location(type_node),
                                ));
                            }
                        }
                    }
                }
            }
        }
        _ => {}
    }
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            collect_cpp_references(child, source, refs);
        }
    }
}

fn is_builtin(name: &str) -> bool {
    matches!(
        name,
        "std::cout" | "std::cerr" | "std::endl" | "printf" | "malloc" | "free" | "sizeof"
    )
}

fn is_primitive(name: &str) -> bool {
    matches!(
        name,
        "int"
            | "char"
            | "float"
            | "double"
            | "bool"
            | "void"
            | "long"
            | "short"
            | "unsigned"
            | "signed"
            | "size_t"
            | "auto"
    )
}
