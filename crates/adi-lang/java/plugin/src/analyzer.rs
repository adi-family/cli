//! Java language analyzer implementation.

use lib_indexer_lang_abi::{
    LocationAbi, ParsedReferenceAbi, ParsedSymbolAbi, ReferenceKindAbi, SymbolKindAbi,
    VisibilityAbi,
};
use tree_sitter::{Node, Parser, Tree};

pub fn extract_symbols(source: &str) -> Vec<ParsedSymbolAbi> {
    let tree = match parse_java(source) {
        Some(t) => t,
        None => return vec![],
    };
    let mut symbols = Vec::new();
    extract_java_symbols(tree.root_node(), source, &mut symbols);
    symbols
}

pub fn extract_references(source: &str) -> Vec<ParsedReferenceAbi> {
    let tree = match parse_java(source) {
        Some(t) => t,
        None => return vec![],
    };
    let mut refs = Vec::new();
    collect_java_references(tree.root_node(), source, &mut refs);
    refs
}

fn parse_java(source: &str) -> Option<Tree> {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_java::LANGUAGE.into())
        .ok()?;
    parser.parse(source, None)
}

fn node_text<'a>(node: Node<'a>, source: &'a str) -> String {
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

fn extract_doc_comment(node: Node, source: &str) -> Option<String> {
    let mut prev = node.prev_sibling();
    while let Some(sibling) = prev {
        match sibling.kind() {
            "block_comment" => {
                let text = node_text(sibling, source);
                if text.starts_with("/**") {
                    return Some(
                        text.trim_start_matches("/**")
                            .trim_end_matches("*/")
                            .lines()
                            .map(|l| l.trim().trim_start_matches('*').trim())
                            .filter(|l| !l.is_empty())
                            .collect::<Vec<_>>()
                            .join("\n"),
                    );
                }
            }
            "line_comment" => {}
            "modifiers" | "marker_annotation" | "annotation" => {}
            _ => break,
        }
        prev = sibling.prev_sibling();
    }
    None
}

fn extract_visibility(node: Node, source: &str) -> VisibilityAbi {
    if let Some(modifiers) = node.child_by_field_name("modifiers") {
        for i in 0..modifiers.child_count() {
            if let Some(child) = modifiers.child(i) {
                let text = node_text(child, source);
                match text.as_str() {
                    "public" => return VisibilityAbi::Public,
                    "private" => return VisibilityAbi::Private,
                    "protected" => return VisibilityAbi::Protected,
                    _ => {}
                }
            }
        }
    }
    VisibilityAbi::Internal // package-private
}

fn extract_method_signature(node: Node, source: &str) -> String {
    let text = node_text(node, source);
    if let Some(brace_pos) = text.find('{') {
        text[..brace_pos].trim().to_string()
    } else {
        text.lines().next().unwrap_or("").to_string()
    }
}

fn extract_java_symbols(node: Node, source: &str, symbols: &mut Vec<ParsedSymbolAbi>) {
    match node.kind() {
        "class_declaration" => {
            if let Some(symbol) = parse_java_class(node, source) {
                symbols.push(symbol);
            }
        }
        "interface_declaration" => {
            if let Some(symbol) = parse_java_interface(node, source) {
                symbols.push(symbol);
            }
        }
        "enum_declaration" => {
            if let Some(symbol) = parse_java_enum(node, source) {
                symbols.push(symbol);
            }
        }
        "method_declaration" => {
            if let Some(symbol) = parse_java_method(node, source) {
                symbols.push(symbol);
            }
        }
        "constructor_declaration" => {
            if let Some(symbol) = parse_java_constructor(node, source) {
                symbols.push(symbol);
            }
        }
        "field_declaration" => {
            parse_java_fields(node, source, symbols);
        }
        "constant_declaration" => {
            parse_java_constants(node, source, symbols);
        }
        _ => {}
    }

    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            extract_java_symbols(child, source, symbols);
        }
    }
}

fn parse_java_class(node: Node, source: &str) -> Option<ParsedSymbolAbi> {
    let name = node.child_by_field_name("name")?;
    let name_text = node_text(name, source);
    let doc_comment = extract_doc_comment(node, source);
    let visibility = extract_visibility(node, source);

    Some(
        ParsedSymbolAbi::new(name_text, SymbolKindAbi::Class, node_location(node))
            .with_visibility(visibility)
            .with_doc_comment_opt(doc_comment),
    )
}

fn parse_java_interface(node: Node, source: &str) -> Option<ParsedSymbolAbi> {
    let name = node.child_by_field_name("name")?;
    let name_text = node_text(name, source);
    let doc_comment = extract_doc_comment(node, source);
    let visibility = extract_visibility(node, source);

    Some(
        ParsedSymbolAbi::new(name_text, SymbolKindAbi::Interface, node_location(node))
            .with_visibility(visibility)
            .with_doc_comment_opt(doc_comment),
    )
}

fn parse_java_enum(node: Node, source: &str) -> Option<ParsedSymbolAbi> {
    let name = node.child_by_field_name("name")?;
    let name_text = node_text(name, source);
    let doc_comment = extract_doc_comment(node, source);
    let visibility = extract_visibility(node, source);

    Some(
        ParsedSymbolAbi::new(name_text, SymbolKindAbi::Enum, node_location(node))
            .with_visibility(visibility)
            .with_doc_comment_opt(doc_comment),
    )
}

fn parse_java_method(node: Node, source: &str) -> Option<ParsedSymbolAbi> {
    let name = node.child_by_field_name("name")?;
    let name_text = node_text(name, source);
    let doc_comment = extract_doc_comment(node, source);
    let visibility = extract_visibility(node, source);
    let signature = extract_method_signature(node, source);

    Some(
        ParsedSymbolAbi::new(name_text, SymbolKindAbi::Method, node_location(node))
            .with_signature(signature)
            .with_visibility(visibility)
            .with_doc_comment_opt(doc_comment),
    )
}

fn parse_java_constructor(node: Node, source: &str) -> Option<ParsedSymbolAbi> {
    let name = node.child_by_field_name("name")?;
    let name_text = node_text(name, source);
    let doc_comment = extract_doc_comment(node, source);
    let visibility = extract_visibility(node, source);
    let signature = extract_method_signature(node, source);

    Some(
        ParsedSymbolAbi::new(name_text, SymbolKindAbi::Constructor, node_location(node))
            .with_signature(signature)
            .with_visibility(visibility)
            .with_doc_comment_opt(doc_comment),
    )
}

fn parse_java_fields(node: Node, source: &str, symbols: &mut Vec<ParsedSymbolAbi>) {
    let visibility = extract_visibility(node, source);
    let doc_comment = extract_doc_comment(node, source);

    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            if child.kind() == "variable_declarator" {
                if let Some(name) = child.child_by_field_name("name") {
                    symbols.push(
                        ParsedSymbolAbi::new(
                            node_text(name, source),
                            SymbolKindAbi::Field,
                            node_location(child),
                        )
                        .with_visibility(visibility)
                        .with_doc_comment_opt(doc_comment.clone()),
                    );
                }
            }
        }
    }
}

fn parse_java_constants(node: Node, source: &str, symbols: &mut Vec<ParsedSymbolAbi>) {
    let visibility = extract_visibility(node, source);
    let doc_comment = extract_doc_comment(node, source);

    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            if child.kind() == "variable_declarator" {
                if let Some(name) = child.child_by_field_name("name") {
                    symbols.push(
                        ParsedSymbolAbi::new(
                            node_text(name, source),
                            SymbolKindAbi::Constant,
                            node_location(child),
                        )
                        .with_visibility(visibility)
                        .with_doc_comment_opt(doc_comment.clone()),
                    );
                }
            }
        }
    }
}

fn collect_java_references(node: Node, source: &str, refs: &mut Vec<ParsedReferenceAbi>) {
    match node.kind() {
        "method_invocation" => {
            if let Some(name) = node.child_by_field_name("name") {
                let name_text = node_text(name, source);
                if !is_common_method(&name_text) {
                    refs.push(ParsedReferenceAbi::new(
                        name_text,
                        ReferenceKindAbi::Call,
                        node_location(name),
                    ));
                }
            }
        }
        "object_creation_expression" => {
            if let Some(type_node) = node.child_by_field_name("type") {
                refs.push(ParsedReferenceAbi::new(
                    node_text(type_node, source),
                    ReferenceKindAbi::Call,
                    node_location(type_node),
                ));
            }
        }
        "type_identifier" => {
            let name = node_text(node, source);
            if !is_primitive_type(&name) {
                refs.push(ParsedReferenceAbi::new(
                    name,
                    ReferenceKindAbi::TypeReference,
                    node_location(node),
                ));
            }
        }
        "field_access" => {
            if let Some(field) = node.child_by_field_name("field") {
                refs.push(ParsedReferenceAbi::new(
                    node_text(field, source),
                    ReferenceKindAbi::FieldAccess,
                    node_location(field),
                ));
            }
        }
        "import_declaration" => {
            for i in 0..node.child_count() {
                if let Some(child) = node.child(i) {
                    if child.kind() == "scoped_identifier" {
                        refs.push(ParsedReferenceAbi::new(
                            node_text(child, source),
                            ReferenceKindAbi::Import,
                            node_location(child),
                        ));
                    }
                }
            }
        }
        "superclass" | "super_interfaces" => {
            for i in 0..node.child_count() {
                if let Some(child) = node.child(i) {
                    if child.kind() == "type_identifier" || child.kind() == "generic_type" {
                        refs.push(ParsedReferenceAbi::new(
                            node_text(child, source),
                            ReferenceKindAbi::Inheritance,
                            node_location(child),
                        ));
                    }
                }
            }
        }
        _ => {}
    }

    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            collect_java_references(child, source, refs);
        }
    }
}

fn is_primitive_type(name: &str) -> bool {
    matches!(
        name,
        "int"
            | "long"
            | "short"
            | "byte"
            | "float"
            | "double"
            | "boolean"
            | "char"
            | "void"
            | "String"
            | "Object"
            | "Integer"
            | "Long"
            | "Short"
            | "Byte"
            | "Float"
            | "Double"
            | "Boolean"
            | "Character"
            | "Void"
    )
}

fn is_common_method(name: &str) -> bool {
    matches!(
        name,
        "toString"
            | "equals"
            | "hashCode"
            | "getClass"
            | "clone"
            | "notify"
            | "notifyAll"
            | "wait"
            | "println"
            | "print"
            | "printf"
            | "format"
    )
}

trait WithDocCommentOpt {
    fn with_doc_comment_opt(self, doc: Option<String>) -> Self;
}

impl WithDocCommentOpt for ParsedSymbolAbi {
    fn with_doc_comment_opt(self, doc: Option<String>) -> Self {
        match doc {
            Some(d) => self.with_doc_comment(d),
            None => self,
        }
    }
}
