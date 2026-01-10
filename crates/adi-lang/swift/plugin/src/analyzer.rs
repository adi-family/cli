//! Swift language analyzer implementation.

use lib_indexer_lang_abi::{
    LocationAbi, ParsedReferenceAbi, ParsedSymbolAbi, ReferenceKindAbi, SymbolKindAbi,
    VisibilityAbi,
};
use tree_sitter::{Node, Parser, Tree};

pub fn extract_symbols(source: &str) -> Vec<ParsedSymbolAbi> {
    let tree = match parse_swift(source) {
        Some(t) => t,
        None => return vec![],
    };
    let mut symbols = Vec::new();
    extract_swift_symbols(tree.root_node(), source, &mut symbols);
    symbols
}

pub fn extract_references(source: &str) -> Vec<ParsedReferenceAbi> {
    let tree = match parse_swift(source) {
        Some(t) => t,
        None => return vec![],
    };
    let mut refs = Vec::new();
    collect_swift_references(tree.root_node(), source, &mut refs);
    refs
}

fn parse_swift(source: &str) -> Option<Tree> {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_swift::LANGUAGE.into())
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
    let mut comments = Vec::new();

    while let Some(sibling) = prev {
        match sibling.kind() {
            "comment" | "multiline_comment" => {
                let text = node_text(sibling, source);
                if text.starts_with("///") || text.starts_with("/**") {
                    comments.push(
                        text.trim_start_matches("///")
                            .trim_start_matches("/**")
                            .trim_end_matches("*/")
                            .lines()
                            .map(|l| l.trim().trim_start_matches('*').trim())
                            .filter(|l| !l.is_empty())
                            .collect::<Vec<_>>()
                            .join("\n"),
                    );
                } else {
                    break;
                }
            }
            "attribute" => {}
            _ => break,
        }
        prev = sibling.prev_sibling();
    }

    if comments.is_empty() {
        None
    } else {
        comments.reverse();
        Some(comments.join("\n"))
    }
}

fn extract_visibility(node: Node, source: &str) -> VisibilityAbi {
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            if child.kind() == "modifiers" {
                for j in 0..child.child_count() {
                    if let Some(modifier) = child.child(j) {
                        let text = node_text(modifier, source);
                        match text.as_str() {
                            "public" => return VisibilityAbi::Public,
                            "private" => return VisibilityAbi::Private,
                            "fileprivate" => return VisibilityAbi::Private,
                            "internal" => return VisibilityAbi::Internal,
                            "open" => return VisibilityAbi::Public,
                            _ => {}
                        }
                    }
                }
            }
        }
    }
    VisibilityAbi::Internal
}

fn extract_function_signature(node: Node, source: &str) -> String {
    let text = node_text(node, source);
    if let Some(brace_pos) = text.find('{') {
        text[..brace_pos].trim().to_string()
    } else {
        text.lines().next().unwrap_or("").to_string()
    }
}

fn extract_swift_symbols(node: Node, source: &str, symbols: &mut Vec<ParsedSymbolAbi>) {
    match node.kind() {
        "class_declaration" => {
            if let Some(symbol) = parse_swift_class(node, source) {
                symbols.push(symbol);
            }
        }
        "struct_declaration" => {
            if let Some(symbol) = parse_swift_struct(node, source) {
                symbols.push(symbol);
            }
        }
        "protocol_declaration" => {
            if let Some(symbol) = parse_swift_protocol(node, source) {
                symbols.push(symbol);
            }
        }
        "enum_declaration" => {
            if let Some(symbol) = parse_swift_enum(node, source) {
                symbols.push(symbol);
            }
        }
        "function_declaration" => {
            if let Some(symbol) = parse_swift_function(node, source) {
                symbols.push(symbol);
            }
        }
        "property_declaration" => {
            if let Some(symbol) = parse_swift_property(node, source) {
                symbols.push(symbol);
            }
        }
        "init_declaration" => {
            if let Some(symbol) = parse_swift_init(node, source) {
                symbols.push(symbol);
            }
        }
        "deinit_declaration" => {
            symbols.push(parse_swift_deinit(node));
        }
        "typealias_declaration" => {
            if let Some(symbol) = parse_swift_typealias(node, source) {
                symbols.push(symbol);
            }
        }
        "extension_declaration" => {
            if let Some(symbol) = parse_swift_extension(node, source) {
                symbols.push(symbol);
            }
        }
        _ => {}
    }

    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            extract_swift_symbols(child, source, symbols);
        }
    }
}

fn parse_swift_class(node: Node, source: &str) -> Option<ParsedSymbolAbi> {
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

fn parse_swift_struct(node: Node, source: &str) -> Option<ParsedSymbolAbi> {
    let name = node.child_by_field_name("name")?;
    let name_text = node_text(name, source);
    let doc_comment = extract_doc_comment(node, source);
    let visibility = extract_visibility(node, source);

    Some(
        ParsedSymbolAbi::new(name_text, SymbolKindAbi::Struct, node_location(node))
            .with_visibility(visibility)
            .with_doc_comment_opt(doc_comment),
    )
}

fn parse_swift_protocol(node: Node, source: &str) -> Option<ParsedSymbolAbi> {
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

fn parse_swift_enum(node: Node, source: &str) -> Option<ParsedSymbolAbi> {
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

fn parse_swift_function(node: Node, source: &str) -> Option<ParsedSymbolAbi> {
    let name = node.child_by_field_name("name")?;
    let name_text = node_text(name, source);
    let doc_comment = extract_doc_comment(node, source);
    let visibility = extract_visibility(node, source);
    let signature = extract_function_signature(node, source);

    Some(
        ParsedSymbolAbi::new(name_text, SymbolKindAbi::Function, node_location(node))
            .with_signature(signature)
            .with_visibility(visibility)
            .with_doc_comment_opt(doc_comment),
    )
}

fn parse_swift_property(node: Node, source: &str) -> Option<ParsedSymbolAbi> {
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            if child.kind() == "pattern" {
                let name_text = node_text(child, source);
                let doc_comment = extract_doc_comment(node, source);
                let visibility = extract_visibility(node, source);

                return Some(
                    ParsedSymbolAbi::new(name_text, SymbolKindAbi::Property, node_location(node))
                        .with_visibility(visibility)
                        .with_doc_comment_opt(doc_comment),
                );
            }
        }
    }
    None
}

fn parse_swift_init(node: Node, source: &str) -> Option<ParsedSymbolAbi> {
    let doc_comment = extract_doc_comment(node, source);
    let visibility = extract_visibility(node, source);
    let signature = extract_function_signature(node, source);

    Some(
        ParsedSymbolAbi::new(
            "init".to_string(),
            SymbolKindAbi::Constructor,
            node_location(node),
        )
        .with_signature(signature)
        .with_visibility(visibility)
        .with_doc_comment_opt(doc_comment),
    )
}

fn parse_swift_deinit(node: Node) -> ParsedSymbolAbi {
    ParsedSymbolAbi::new(
        "deinit".to_string(),
        SymbolKindAbi::Destructor,
        node_location(node),
    )
    .with_visibility(VisibilityAbi::Internal)
}

fn parse_swift_typealias(node: Node, source: &str) -> Option<ParsedSymbolAbi> {
    let name = node.child_by_field_name("name")?;
    let name_text = node_text(name, source);
    let doc_comment = extract_doc_comment(node, source);
    let visibility = extract_visibility(node, source);

    Some(
        ParsedSymbolAbi::new(name_text, SymbolKindAbi::Type, node_location(node))
            .with_visibility(visibility)
            .with_doc_comment_opt(doc_comment),
    )
}

fn parse_swift_extension(node: Node, source: &str) -> Option<ParsedSymbolAbi> {
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            if child.kind() == "user_type" || child.kind() == "type_identifier" {
                let name_text = format!("extension {}", node_text(child, source));
                return Some(ParsedSymbolAbi::new(
                    name_text,
                    SymbolKindAbi::Class,
                    node_location(node),
                ));
            }
        }
    }
    None
}

fn collect_swift_references(node: Node, source: &str, refs: &mut Vec<ParsedReferenceAbi>) {
    match node.kind() {
        "call_expression" => {
            if let Some(func) = node.child(0) {
                let name = extract_call_name(func, source);
                if !name.is_empty() && !is_common_function(&name) {
                    refs.push(ParsedReferenceAbi::new(
                        name,
                        ReferenceKindAbi::Call,
                        node_location(func),
                    ));
                }
            }
        }
        "navigation_expression" => {
            if let Some(suffix) = node.child_by_field_name("suffix") {
                refs.push(ParsedReferenceAbi::new(
                    node_text(suffix, source),
                    ReferenceKindAbi::FieldAccess,
                    node_location(suffix),
                ));
            }
        }
        "user_type" | "type_identifier" => {
            let name = node_text(node, source);
            if !is_primitive_type(&name) {
                refs.push(ParsedReferenceAbi::new(
                    name,
                    ReferenceKindAbi::TypeReference,
                    node_location(node),
                ));
            }
        }
        "import_declaration" => {
            for i in 0..node.child_count() {
                if let Some(child) = node.child(i) {
                    if child.kind() == "identifier" {
                        refs.push(ParsedReferenceAbi::new(
                            node_text(child, source),
                            ReferenceKindAbi::Import,
                            node_location(child),
                        ));
                    }
                }
            }
        }
        "inheritance_specifier" => {
            for i in 0..node.child_count() {
                if let Some(child) = node.child(i) {
                    if child.kind() == "user_type" || child.kind() == "type_identifier" {
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
            collect_swift_references(child, source, refs);
        }
    }
}

fn extract_call_name(node: Node, source: &str) -> String {
    match node.kind() {
        "simple_identifier" => node_text(node, source),
        "navigation_expression" => {
            if let Some(suffix) = node.child_by_field_name("suffix") {
                node_text(suffix, source)
            } else {
                String::new()
            }
        }
        _ => node_text(node, source),
    }
}

fn is_primitive_type(name: &str) -> bool {
    matches!(
        name,
        "Int"
            | "Int8"
            | "Int16"
            | "Int32"
            | "Int64"
            | "UInt"
            | "UInt8"
            | "UInt16"
            | "UInt32"
            | "UInt64"
            | "Float"
            | "Double"
            | "Bool"
            | "String"
            | "Character"
            | "Void"
            | "Never"
            | "Any"
            | "AnyObject"
            | "Self"
            | "Optional"
            | "Array"
            | "Dictionary"
            | "Set"
    )
}

fn is_common_function(name: &str) -> bool {
    matches!(
        name,
        "print"
            | "debugPrint"
            | "dump"
            | "fatalError"
            | "precondition"
            | "preconditionFailure"
            | "assert"
            | "assertionFailure"
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
