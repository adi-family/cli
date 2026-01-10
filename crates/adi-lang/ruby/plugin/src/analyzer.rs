//! Ruby language analyzer implementation.

use lib_indexer_lang_abi::{
    LocationAbi, ParsedReferenceAbi, ParsedSymbolAbi, ReferenceKindAbi, SymbolKindAbi,
    VisibilityAbi,
};
use tree_sitter::{Node, Parser, Tree};

pub fn extract_symbols(source: &str) -> Vec<ParsedSymbolAbi> {
    let tree = match parse_ruby(source) {
        Some(t) => t,
        None => return vec![],
    };
    let mut symbols = Vec::new();
    let mut visibility = VisibilityAbi::Public;
    extract_ruby_symbols(tree.root_node(), source, &mut symbols, &mut visibility);
    symbols
}

pub fn extract_references(source: &str) -> Vec<ParsedReferenceAbi> {
    let tree = match parse_ruby(source) {
        Some(t) => t,
        None => return vec![],
    };
    let mut refs = Vec::new();
    collect_ruby_references(tree.root_node(), source, &mut refs);
    refs
}

fn parse_ruby(source: &str) -> Option<Tree> {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_ruby::LANGUAGE.into())
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
        if sibling.kind() == "comment" {
            let text = node_text(sibling, source);
            comments.push(text.trim_start_matches('#').trim().to_string());
        } else {
            break;
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

fn extract_method_signature(node: Node, source: &str) -> String {
    let text = node_text(node, source);
    if let Some(newline_pos) = text.find('\n') {
        text[..newline_pos].trim().to_string()
    } else {
        text.trim().to_string()
    }
}

fn extract_ruby_symbols(
    node: Node,
    source: &str,
    symbols: &mut Vec<ParsedSymbolAbi>,
    current_visibility: &mut VisibilityAbi,
) {
    match node.kind() {
        "class" => {
            if let Some(symbol) = parse_ruby_class(node, source) {
                symbols.push(symbol);
            }
            let mut class_visibility = VisibilityAbi::Public;
            for i in 0..node.child_count() {
                if let Some(child) = node.child(i) {
                    extract_ruby_symbols(child, source, symbols, &mut class_visibility);
                }
            }
            return;
        }
        "module" => {
            if let Some(symbol) = parse_ruby_module(node, source) {
                symbols.push(symbol);
            }
            let mut mod_visibility = VisibilityAbi::Public;
            for i in 0..node.child_count() {
                if let Some(child) = node.child(i) {
                    extract_ruby_symbols(child, source, symbols, &mut mod_visibility);
                }
            }
            return;
        }
        "method" => {
            if let Some(symbol) = parse_ruby_method(node, source, *current_visibility) {
                symbols.push(symbol);
            }
        }
        "singleton_method" => {
            if let Some(symbol) = parse_ruby_singleton_method(node, source) {
                symbols.push(symbol);
            }
        }
        "identifier" => {
            let text = node_text(node, source);
            match text.as_str() {
                "private" => *current_visibility = VisibilityAbi::Private,
                "protected" => *current_visibility = VisibilityAbi::Protected,
                "public" => *current_visibility = VisibilityAbi::Public,
                _ => {}
            }
        }
        "assignment" => {
            if let Some(symbol) = parse_ruby_constant(node, source) {
                symbols.push(symbol);
            }
        }
        _ => {}
    }

    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            extract_ruby_symbols(child, source, symbols, current_visibility);
        }
    }
}

fn parse_ruby_class(node: Node, source: &str) -> Option<ParsedSymbolAbi> {
    let name = node.child_by_field_name("name")?;
    let name_text = node_text(name, source);
    let doc_comment = extract_doc_comment(node, source);

    Some(
        ParsedSymbolAbi::new(name_text, SymbolKindAbi::Class, node_location(node))
            .with_visibility(VisibilityAbi::Public)
            .with_doc_comment_opt(doc_comment),
    )
}

fn parse_ruby_module(node: Node, source: &str) -> Option<ParsedSymbolAbi> {
    let name = node.child_by_field_name("name")?;
    let name_text = node_text(name, source);
    let doc_comment = extract_doc_comment(node, source);

    Some(
        ParsedSymbolAbi::new(name_text, SymbolKindAbi::Module, node_location(node))
            .with_visibility(VisibilityAbi::Public)
            .with_doc_comment_opt(doc_comment),
    )
}

fn parse_ruby_method(
    node: Node,
    source: &str,
    visibility: VisibilityAbi,
) -> Option<ParsedSymbolAbi> {
    let name = node.child_by_field_name("name")?;
    let name_text = node_text(name, source);
    let doc_comment = extract_doc_comment(node, source);
    let signature = extract_method_signature(node, source);

    Some(
        ParsedSymbolAbi::new(name_text, SymbolKindAbi::Method, node_location(node))
            .with_signature(signature)
            .with_visibility(visibility)
            .with_doc_comment_opt(doc_comment),
    )
}

fn parse_ruby_singleton_method(node: Node, source: &str) -> Option<ParsedSymbolAbi> {
    let name = node.child_by_field_name("name")?;
    let name_text = node_text(name, source);
    let doc_comment = extract_doc_comment(node, source);
    let signature = extract_method_signature(node, source);

    Some(
        ParsedSymbolAbi::new(
            format!("self.{}", name_text),
            SymbolKindAbi::Method,
            node_location(node),
        )
        .with_signature(signature)
        .with_visibility(VisibilityAbi::Public)
        .with_doc_comment_opt(doc_comment),
    )
}

fn parse_ruby_constant(node: Node, source: &str) -> Option<ParsedSymbolAbi> {
    let left = node.child_by_field_name("left")?;
    if left.kind() == "constant" {
        let name_text = node_text(left, source);
        let doc_comment = extract_doc_comment(node, source);

        return Some(
            ParsedSymbolAbi::new(name_text, SymbolKindAbi::Constant, node_location(node))
                .with_visibility(VisibilityAbi::Public)
                .with_doc_comment_opt(doc_comment),
        );
    }
    None
}

fn collect_ruby_references(node: Node, source: &str, refs: &mut Vec<ParsedReferenceAbi>) {
    match node.kind() {
        "call" | "method_call" => {
            if let Some(method) = node.child_by_field_name("method") {
                let name = node_text(method, source);
                // Handle require/require_relative as imports
                if name == "require" || name == "require_relative" {
                    if let Some(arg) = node.child_by_field_name("arguments") {
                        refs.push(ParsedReferenceAbi::new(
                            node_text(arg, source),
                            ReferenceKindAbi::Import,
                            node_location(arg),
                        ));
                    }
                } else if !is_common_method(&name) {
                    refs.push(ParsedReferenceAbi::new(
                        name,
                        ReferenceKindAbi::Call,
                        node_location(method),
                    ));
                }
            }
        }
        "constant" => {
            let name = node_text(node, source);
            let parent = node.parent();
            if let Some(p) = parent {
                if p.kind() != "class" && p.kind() != "module" {
                    refs.push(ParsedReferenceAbi::new(
                        name,
                        ReferenceKindAbi::TypeReference,
                        node_location(node),
                    ));
                }
            }
        }
        "scope_resolution" => {
            let name = node_text(node, source);
            refs.push(ParsedReferenceAbi::new(
                name,
                ReferenceKindAbi::TypeReference,
                node_location(node),
            ));
        }
        "superclass" => {
            let name = node_text(node, source);
            refs.push(ParsedReferenceAbi::new(
                name,
                ReferenceKindAbi::Inheritance,
                node_location(node),
            ));
        }
        _ => {}
    }

    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            collect_ruby_references(child, source, refs);
        }
    }
}

fn is_common_method(name: &str) -> bool {
    matches!(
        name,
        "new"
            | "initialize"
            | "to_s"
            | "to_i"
            | "to_a"
            | "to_h"
            | "inspect"
            | "class"
            | "is_a?"
            | "kind_of?"
            | "instance_of?"
            | "respond_to?"
            | "send"
            | "puts"
            | "print"
            | "p"
            | "raise"
            | "fail"
            | "attr_reader"
            | "attr_writer"
            | "attr_accessor"
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
