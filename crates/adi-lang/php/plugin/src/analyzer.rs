//! PHP language analyzer implementation.

use lib_indexer_lang_abi::{
    LocationAbi, ParsedReferenceAbi, ParsedSymbolAbi, ReferenceKindAbi, SymbolKindAbi,
    VisibilityAbi,
};
use tree_sitter::{Node, Parser, Tree};

pub fn extract_symbols(source: &str) -> Vec<ParsedSymbolAbi> {
    let tree = match parse_php(source) {
        Some(t) => t,
        None => return vec![],
    };
    let mut symbols = Vec::new();
    extract_php_symbols(tree.root_node(), source, &mut symbols);
    symbols
}

pub fn extract_references(source: &str) -> Vec<ParsedReferenceAbi> {
    let tree = match parse_php(source) {
        Some(t) => t,
        None => return vec![],
    };
    let mut refs = Vec::new();
    collect_php_references(tree.root_node(), source, &mut refs);
    refs
}

fn parse_php(source: &str) -> Option<Tree> {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_php::LANGUAGE_PHP.into())
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
            "comment" => {
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
            "attribute_list" => {}
            _ => break,
        }
        prev = sibling.prev_sibling();
    }
    None
}

fn extract_visibility(node: Node, source: &str) -> VisibilityAbi {
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            if child.kind() == "visibility_modifier" {
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
    VisibilityAbi::Public
}

fn extract_function_signature(node: Node, source: &str) -> String {
    let text = node_text(node, source);
    if let Some(brace_pos) = text.find('{') {
        text[..brace_pos].trim().to_string()
    } else {
        text.lines().next().unwrap_or("").to_string()
    }
}

fn extract_php_symbols(node: Node, source: &str, symbols: &mut Vec<ParsedSymbolAbi>) {
    match node.kind() {
        "class_declaration" => {
            if let Some(symbol) = parse_php_class(node, source) {
                symbols.push(symbol);
            }
        }
        "interface_declaration" => {
            if let Some(symbol) = parse_php_interface(node, source) {
                symbols.push(symbol);
            }
        }
        "trait_declaration" => {
            if let Some(symbol) = parse_php_trait(node, source) {
                symbols.push(symbol);
            }
        }
        "enum_declaration" => {
            if let Some(symbol) = parse_php_enum(node, source) {
                symbols.push(symbol);
            }
        }
        "function_definition" => {
            if let Some(symbol) = parse_php_function(node, source) {
                symbols.push(symbol);
            }
        }
        "method_declaration" => {
            if let Some(symbol) = parse_php_method(node, source) {
                symbols.push(symbol);
            }
        }
        "property_declaration" => {
            parse_php_properties(node, source, symbols);
        }
        "const_declaration" => {
            parse_php_constants(node, source, symbols);
        }
        "namespace_definition" => {
            if let Some(symbol) = parse_php_namespace(node, source) {
                symbols.push(symbol);
            }
        }
        _ => {}
    }

    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            extract_php_symbols(child, source, symbols);
        }
    }
}

fn parse_php_class(node: Node, source: &str) -> Option<ParsedSymbolAbi> {
    let name = node.child_by_field_name("name")?;
    let name_text = node_text(name, source);
    let doc_comment = extract_doc_comment(node, source);

    Some(
        ParsedSymbolAbi::new(name_text, SymbolKindAbi::Class, node_location(node))
            .with_visibility(VisibilityAbi::Public)
            .with_doc_comment_opt(doc_comment),
    )
}

fn parse_php_interface(node: Node, source: &str) -> Option<ParsedSymbolAbi> {
    let name = node.child_by_field_name("name")?;
    let name_text = node_text(name, source);
    let doc_comment = extract_doc_comment(node, source);

    Some(
        ParsedSymbolAbi::new(name_text, SymbolKindAbi::Interface, node_location(node))
            .with_visibility(VisibilityAbi::Public)
            .with_doc_comment_opt(doc_comment),
    )
}

fn parse_php_trait(node: Node, source: &str) -> Option<ParsedSymbolAbi> {
    let name = node.child_by_field_name("name")?;
    let name_text = node_text(name, source);
    let doc_comment = extract_doc_comment(node, source);

    Some(
        ParsedSymbolAbi::new(name_text, SymbolKindAbi::Trait, node_location(node))
            .with_visibility(VisibilityAbi::Public)
            .with_doc_comment_opt(doc_comment),
    )
}

fn parse_php_enum(node: Node, source: &str) -> Option<ParsedSymbolAbi> {
    let name = node.child_by_field_name("name")?;
    let name_text = node_text(name, source);
    let doc_comment = extract_doc_comment(node, source);

    Some(
        ParsedSymbolAbi::new(name_text, SymbolKindAbi::Enum, node_location(node))
            .with_visibility(VisibilityAbi::Public)
            .with_doc_comment_opt(doc_comment),
    )
}

fn parse_php_function(node: Node, source: &str) -> Option<ParsedSymbolAbi> {
    let name = node.child_by_field_name("name")?;
    let name_text = node_text(name, source);
    let doc_comment = extract_doc_comment(node, source);
    let signature = extract_function_signature(node, source);

    Some(
        ParsedSymbolAbi::new(name_text, SymbolKindAbi::Function, node_location(node))
            .with_signature(signature)
            .with_visibility(VisibilityAbi::Public)
            .with_doc_comment_opt(doc_comment),
    )
}

fn parse_php_method(node: Node, source: &str) -> Option<ParsedSymbolAbi> {
    let name = node.child_by_field_name("name")?;
    let name_text = node_text(name, source);
    let doc_comment = extract_doc_comment(node, source);
    let visibility = extract_visibility(node, source);
    let signature = extract_function_signature(node, source);

    Some(
        ParsedSymbolAbi::new(name_text, SymbolKindAbi::Method, node_location(node))
            .with_signature(signature)
            .with_visibility(visibility)
            .with_doc_comment_opt(doc_comment),
    )
}

fn parse_php_properties(node: Node, source: &str, symbols: &mut Vec<ParsedSymbolAbi>) {
    let visibility = extract_visibility(node, source);
    let doc_comment = extract_doc_comment(node, source);

    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            if child.kind() == "property_element" {
                if let Some(name) = child.child_by_field_name("name") {
                    let name_text = node_text(name, source);
                    symbols.push(
                        ParsedSymbolAbi::new(
                            name_text,
                            SymbolKindAbi::Property,
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

fn parse_php_constants(node: Node, source: &str, symbols: &mut Vec<ParsedSymbolAbi>) {
    let visibility = extract_visibility(node, source);
    let doc_comment = extract_doc_comment(node, source);

    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            if child.kind() == "const_element" {
                if let Some(name) = child.child_by_field_name("name") {
                    let name_text = node_text(name, source);
                    symbols.push(
                        ParsedSymbolAbi::new(
                            name_text,
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

fn parse_php_namespace(node: Node, source: &str) -> Option<ParsedSymbolAbi> {
    let name = node.child_by_field_name("name")?;
    let name_text = node_text(name, source);

    Some(ParsedSymbolAbi::new(
        name_text,
        SymbolKindAbi::Namespace,
        node_location(node),
    ))
}

fn collect_php_references(node: Node, source: &str, refs: &mut Vec<ParsedReferenceAbi>) {
    match node.kind() {
        "function_call_expression" => {
            if let Some(func) = node.child_by_field_name("function") {
                let name = node_text(func, source);
                if !is_builtin_function(&name) {
                    refs.push(ParsedReferenceAbi::new(
                        name,
                        ReferenceKindAbi::Call,
                        node_location(func),
                    ));
                }
            }
        }
        "member_call_expression" | "nullsafe_member_call_expression" => {
            if let Some(name) = node.child_by_field_name("name") {
                refs.push(ParsedReferenceAbi::new(
                    node_text(name, source),
                    ReferenceKindAbi::Call,
                    node_location(name),
                ));
            }
        }
        "scoped_call_expression" => {
            if let Some(name) = node.child_by_field_name("name") {
                refs.push(ParsedReferenceAbi::new(
                    node_text(name, source),
                    ReferenceKindAbi::Call,
                    node_location(name),
                ));
            }
        }
        "object_creation_expression" => {
            for i in 0..node.child_count() {
                if let Some(child) = node.child(i) {
                    if child.kind() == "name" || child.kind() == "qualified_name" {
                        refs.push(ParsedReferenceAbi::new(
                            node_text(child, source),
                            ReferenceKindAbi::Call,
                            node_location(child),
                        ));
                    }
                }
            }
        }
        "named_type" => {
            let name = node_text(node, source);
            if !is_primitive_type(&name) {
                refs.push(ParsedReferenceAbi::new(
                    name,
                    ReferenceKindAbi::TypeReference,
                    node_location(node),
                ));
            }
        }
        "member_access_expression" | "nullsafe_member_access_expression" => {
            if let Some(name) = node.child_by_field_name("name") {
                refs.push(ParsedReferenceAbi::new(
                    node_text(name, source),
                    ReferenceKindAbi::FieldAccess,
                    node_location(name),
                ));
            }
        }
        "namespace_use_declaration" => {
            for i in 0..node.child_count() {
                if let Some(child) = node.child(i) {
                    if child.kind() == "namespace_use_clause" {
                        if let Some(name) = child.child_by_field_name("name") {
                            refs.push(ParsedReferenceAbi::new(
                                node_text(name, source),
                                ReferenceKindAbi::Import,
                                node_location(name),
                            ));
                        }
                    }
                }
            }
        }
        "base_clause" | "class_interface_clause" => {
            for i in 0..node.child_count() {
                if let Some(child) = node.child(i) {
                    if child.kind() == "name" || child.kind() == "qualified_name" {
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
            collect_php_references(child, source, refs);
        }
    }
}

fn is_primitive_type(name: &str) -> bool {
    matches!(
        name,
        "int"
            | "float"
            | "string"
            | "bool"
            | "array"
            | "object"
            | "callable"
            | "iterable"
            | "void"
            | "null"
            | "mixed"
            | "never"
            | "true"
            | "false"
            | "self"
            | "static"
            | "parent"
    )
}

fn is_builtin_function(name: &str) -> bool {
    matches!(
        name,
        "echo"
            | "print"
            | "var_dump"
            | "print_r"
            | "isset"
            | "empty"
            | "unset"
            | "die"
            | "exit"
            | "array"
            | "list"
            | "include"
            | "include_once"
            | "require"
            | "require_once"
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
