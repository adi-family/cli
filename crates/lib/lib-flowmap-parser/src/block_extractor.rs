use lib_flowmap_core::{Block, BlockId, BlockMetadata, BlockType, FlowMapOutput, Location};
use std::collections::HashSet;
use tree_sitter::{Node, Parser};

/// Block-based extractor for TypeScript/JavaScript
/// Parses complete AST into flat block library with data flow tracking
pub struct BlockExtractor {
    parser: Parser,
}

/// Scope for tracking variable bindings and data flow
struct Scope {
    /// Variables defined in this scope
    defined: HashSet<String>,
    /// Variables used that weren't defined locally (captured from outer scope)
    used_external: HashSet<String>,
}

impl Scope {
    fn new() -> Self {
        Self {
            defined: HashSet::new(),
            used_external: HashSet::new(),
        }
    }

    fn define(&mut self, name: &str) {
        self.defined.insert(name.to_string());
    }

    fn use_var(&mut self, name: &str) {
        if !self.defined.contains(name) {
            self.used_external.insert(name.to_string());
        }
    }

    fn is_defined(&self, name: &str) -> bool {
        self.defined.contains(name)
    }
}

impl BlockExtractor {
    pub fn new() -> crate::Result<Self> {
        let mut parser = Parser::new();
        let language = tree_sitter_typescript::LANGUAGE_TYPESCRIPT;
        parser
            .set_language(&language.into())
            .map_err(|e| crate::ParseError::TreeSitter(e.to_string()))?;

        Ok(Self { parser })
    }

    /// Create extractor for JavaScript
    pub fn new_javascript() -> crate::Result<Self> {
        let mut parser = Parser::new();
        let language = tree_sitter_typescript::LANGUAGE_TSX;
        parser
            .set_language(&language.into())
            .map_err(|e| crate::ParseError::TreeSitter(e.to_string()))?;

        Ok(Self { parser })
    }

    /// Parse a file and return the block-based output
    pub fn parse_file(&mut self, source: &str, file_path: &str) -> crate::Result<FlowMapOutput> {
        let tree =
            self.parser
                .parse(source, None)
                .ok_or_else(|| crate::ParseError::ParseFailed {
                    path: file_path.to_string(),
                })?;

        let mut output = FlowMapOutput::new()
            .with_file(file_path.to_string())
            .with_language(self.detect_language(file_path));

        let root = tree.root_node();
        let mut scope = Scope::new();

        // Extract all top-level blocks
        self.extract_node(&root, source, file_path, &mut output, &mut scope, true);

        Ok(output)
    }

    fn detect_language(&self, file_path: &str) -> String {
        if file_path.ends_with(".tsx") {
            "tsx".to_string()
        } else if file_path.ends_with(".ts") {
            "typescript".to_string()
        } else if file_path.ends_with(".jsx") {
            "jsx".to_string()
        } else {
            "javascript".to_string()
        }
    }

    /// Recursively extract blocks from a node
    fn extract_node(
        &self,
        node: &Node,
        source: &str,
        file_path: &str,
        output: &mut FlowMapOutput,
        scope: &mut Scope,
        is_top_level: bool,
    ) -> Option<BlockId> {
        let kind = node.kind();

        match kind {
            // Module/Program root
            "program" => {
                let mut module_children = Vec::new();
                let mut cursor = node.walk();

                for child in node.children(&mut cursor) {
                    if let Some(child_id) =
                        self.extract_node(&child, source, file_path, output, scope, true)
                    {
                        module_children.push(child_id);
                    }
                }

                if module_children.is_empty() {
                    return None;
                }

                let block = Block::new("module", BlockType::Module)
                    .with_children(module_children.clone())
                    .with_location(self.node_location(node, file_path));

                let id = output.add_block_auto(block);
                output.add_root(id.clone());
                Some(id)
            }

            // Imports
            "import_statement" => {
                let imports = self.extract_import_names(node, source);
                let from = self.get_import_source(node, source);

                for name in &imports {
                    scope.define(name);
                }

                let block = Block::new(
                    format!("import from {}", from.as_deref().unwrap_or("?")),
                    BlockType::Import,
                )
                .with_produces(imports)
                .with_location(self.node_location(node, file_path))
                .with_code(self.node_text(node, source))
                .with_metadata(BlockMetadata {
                    target: from,
                    ..Default::default()
                });

                let id = output.add_block_auto(block);
                if is_top_level {
                    output.add_root(id.clone());
                }
                Some(id)
            }

            // Exports
            "export_statement" => {
                let mut children = Vec::new();

                if let Some(decl) = node.child_by_field_name("declaration") {
                    if let Some(child_id) =
                        self.extract_node(&decl, source, file_path, output, scope, false)
                    {
                        children.push(child_id);
                    }
                }

                // Handle export default expression
                if let Some(value) = node.child_by_field_name("value") {
                    if let Some(child_id) =
                        self.extract_node(&value, source, file_path, output, scope, false)
                    {
                        children.push(child_id);
                    }
                }

                let is_default = self.has_child_kind(node, "default");
                let block_type = if is_default {
                    BlockType::ExportDefault
                } else {
                    BlockType::Export
                };

                let block = Block::new("export", block_type)
                    .with_children(children)
                    .with_location(self.node_location(node, file_path))
                    .with_metadata(BlockMetadata {
                        is_exported: Some(true),
                        ..Default::default()
                    });

                let id = output.add_block_auto(block);
                if is_top_level {
                    output.add_root(id.clone());
                }
                Some(id)
            }

            "function_declaration" => {
                self.extract_function(node, source, file_path, output, is_top_level)
            }

            // Arrow functions
            "arrow_function" => self.extract_arrow_function(node, source, file_path, output),

            "function_expression" | "function" => {
                self.extract_function(node, source, file_path, output, false)
            }

            // Generator functions
            "generator_function_declaration" | "generator_function" => self
                .extract_function_with_type(
                    node,
                    source,
                    file_path,
                    output,
                    BlockType::Generator,
                    false,
                ),

            // Class declarations
            "class_declaration" | "class" => {
                self.extract_class(node, source, file_path, output, is_top_level)
            }

            // Variable declarations
            "lexical_declaration" | "variable_declaration" => self.extract_variable_declaration(
                node,
                source,
                file_path,
                output,
                scope,
                is_top_level,
            ),

            // If statements
            "if_statement" => self.extract_if_statement(node, source, file_path, output, scope),

            // Switch statements
            "switch_statement" => {
                self.extract_switch_statement(node, source, file_path, output, scope)
            }

            // Try statements
            "try_statement" => self.extract_try_statement(node, source, file_path, output, scope),

            // Loops
            "for_statement" => {
                self.extract_loop(node, source, file_path, output, scope, BlockType::For)
            }
            "for_in_statement" => {
                self.extract_loop(node, source, file_path, output, scope, BlockType::ForIn)
            }
            "for_of_statement" => {
                self.extract_loop(node, source, file_path, output, scope, BlockType::ForOf)
            }
            "while_statement" => {
                self.extract_loop(node, source, file_path, output, scope, BlockType::While)
            }
            "do_statement" => {
                self.extract_loop(node, source, file_path, output, scope, BlockType::DoWhile)
            }

            // Expression statements
            "expression_statement" => {
                if let Some(expr) = node.child(0) {
                    self.extract_node(&expr, source, file_path, output, scope, false)
                } else {
                    None
                }
            }

            // Call expressions
            "call_expression" => {
                self.extract_call_expression(node, source, file_path, output, scope)
            }

            // Await expressions
            "await_expression" => {
                self.extract_await_expression(node, source, file_path, output, scope)
            }

            // New expressions
            "new_expression" => self.extract_new_expression(node, source, file_path, output, scope),

            // Assignment expressions
            "assignment_expression" => {
                self.extract_assignment(node, source, file_path, output, scope)
            }

            // Return statements
            "return_statement" => {
                self.extract_return_statement(node, source, file_path, output, scope)
            }

            // Throw statements
            "throw_statement" => {
                self.extract_throw_statement(node, source, file_path, output, scope)
            }

            // Break/continue
            "break_statement" => {
                let label = node
                    .child_by_field_name("label")
                    .map(|n| self.node_text(&n, source));
                let block = Block::new(
                    label.unwrap_or_else(|| "break".to_string()),
                    BlockType::Break,
                )
                .with_location(self.node_location(node, file_path));
                Some(output.add_block_auto(block))
            }
            "continue_statement" => {
                let label = node
                    .child_by_field_name("label")
                    .map(|n| self.node_text(&n, source));
                let block = Block::new(
                    label.unwrap_or_else(|| "continue".to_string()),
                    BlockType::Continue,
                )
                .with_location(self.node_location(node, file_path));
                Some(output.add_block_auto(block))
            }

            // Yield expressions
            "yield_expression" => {
                let mut uses = Vec::new();
                if let Some(arg) = node.child_by_field_name("argument") {
                    uses.extend(self.extract_used_identifiers(&arg, source, scope));
                }
                let is_delegate = self.has_child_kind(node, "*");
                let block_type = if is_delegate {
                    BlockType::YieldFrom
                } else {
                    BlockType::Yield
                };

                let block = Block::new("yield", block_type)
                    .with_uses(uses)
                    .with_location(self.node_location(node, file_path))
                    .with_code(self.node_text(node, source));
                Some(output.add_block_auto(block))
            }

            // Interface declarations (TypeScript)
            "interface_declaration" => {
                let name = node
                    .child_by_field_name("name")
                    .map(|n| self.node_text(&n, source))
                    .unwrap_or_else(|| "interface".to_string());

                let block = Block::new(name, BlockType::Interface)
                    .with_location(self.node_location(node, file_path))
                    .with_code(self.node_text(node, source));

                let id = output.add_block_auto(block);
                if is_top_level {
                    output.add_root(id.clone());
                }
                Some(id)
            }

            // Type aliases (TypeScript)
            "type_alias_declaration" => {
                let name = node
                    .child_by_field_name("name")
                    .map(|n| self.node_text(&n, source))
                    .unwrap_or_else(|| "type".to_string());

                let block = Block::new(name, BlockType::TypeAlias)
                    .with_location(self.node_location(node, file_path))
                    .with_code(self.node_text(node, source));

                let id = output.add_block_auto(block);
                if is_top_level {
                    output.add_root(id.clone());
                }
                Some(id)
            }

            "enum_declaration" => self.extract_enum(node, source, file_path, output, is_top_level),

            // Statement block
            "statement_block" => {
                let mut children = Vec::new();
                let mut cursor = node.walk();

                for child in node.children(&mut cursor) {
                    if child.kind() == "{" || child.kind() == "}" {
                        continue;
                    }
                    if let Some(child_id) =
                        self.extract_node(&child, source, file_path, output, scope, false)
                    {
                        children.push(child_id);
                    }
                }

                if children.is_empty() {
                    None
                } else if children.len() == 1 {
                    // Don't wrap single child in block
                    Some(children.into_iter().next().unwrap())
                } else {
                    let block = Block::new("block", BlockType::Block)
                        .with_children(children)
                        .with_location(self.node_location(node, file_path));
                    Some(output.add_block_auto(block))
                }
            }

            // Ternary expressions
            "ternary_expression" => {
                let condition = node.child_by_field_name("condition");
                let consequence = node.child_by_field_name("consequence");
                let alternative = node.child_by_field_name("alternative");

                let mut uses = Vec::new();
                let mut children = Vec::new();

                if let Some(cond) = condition {
                    uses.extend(self.extract_used_identifiers(&cond, source, scope));
                }
                if let Some(cons) = consequence {
                    if let Some(id) =
                        self.extract_node(&cons, source, file_path, output, scope, false)
                    {
                        children.push(id);
                    }
                    uses.extend(self.extract_used_identifiers(&cons, source, scope));
                }
                if let Some(alt) = alternative {
                    if let Some(id) =
                        self.extract_node(&alt, source, file_path, output, scope, false)
                    {
                        children.push(id);
                    }
                    uses.extend(self.extract_used_identifiers(&alt, source, scope));
                }

                let cond_text = condition
                    .map(|c| self.node_text(&c, source))
                    .unwrap_or_default();
                let block = Block::new(format!("{} ? : ", cond_text), BlockType::Ternary)
                    .with_uses(uses)
                    .with_children(children)
                    .with_location(self.node_location(node, file_path))
                    .with_metadata(BlockMetadata {
                        condition: condition.map(|c| self.node_text(&c, source)),
                        ..Default::default()
                    });

                Some(output.add_block_auto(block))
            }

            // Binary expressions
            "binary_expression" => {
                let uses = self.extract_used_identifiers(node, source, scope);
                let block = Block::new(
                    self.truncate(&self.node_text(node, source), 50),
                    BlockType::Binary,
                )
                .with_uses(uses)
                .with_location(self.node_location(node, file_path))
                .with_code(self.node_text(node, source));
                Some(output.add_block_auto(block))
            }

            // Unary expressions
            "unary_expression" | "update_expression" => {
                let uses = self.extract_used_identifiers(node, source, scope);
                let block = Block::new(
                    self.truncate(&self.node_text(node, source), 50),
                    BlockType::Unary,
                )
                .with_uses(uses)
                .with_location(self.node_location(node, file_path))
                .with_code(self.node_text(node, source));
                Some(output.add_block_auto(block))
            }

            // Object expressions
            "object" | "object_expression" => {
                let mut children = Vec::new();
                let mut produces = Vec::new();
                let mut uses = Vec::new();

                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    match child.kind() {
                        "pair" | "property" => {
                            if let Some(key) = child.child_by_field_name("key") {
                                produces.push(self.node_text(&key, source));
                            }
                            if let Some(value) = child.child_by_field_name("value") {
                                uses.extend(self.extract_used_identifiers(&value, source, scope));
                                if let Some(id) = self
                                    .extract_node(&value, source, file_path, output, scope, false)
                                {
                                    children.push(id);
                                }
                            }
                        }
                        "shorthand_property_identifier"
                        | "shorthand_property_identifier_pattern" => {
                            let name = self.node_text(&child, source);
                            produces.push(name.clone());
                            uses.push(name);
                        }
                        "spread_element" => {
                            if let Some(arg) = child.child(1) {
                                uses.extend(self.extract_used_identifiers(&arg, source, scope));
                            }
                        }
                        _ => {}
                    }
                }

                let block = Block::new("object", BlockType::Object)
                    .with_uses(uses)
                    .with_produces(produces)
                    .with_children(children)
                    .with_location(self.node_location(node, file_path));
                Some(output.add_block_auto(block))
            }

            // Array expressions
            "array" | "array_expression" => {
                let mut uses = Vec::new();
                let mut children = Vec::new();

                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    if child.kind() == "[" || child.kind() == "]" || child.kind() == "," {
                        continue;
                    }
                    uses.extend(self.extract_used_identifiers(&child, source, scope));
                    if let Some(id) =
                        self.extract_node(&child, source, file_path, output, scope, false)
                    {
                        children.push(id);
                    }
                }

                let block = Block::new("array", BlockType::Array)
                    .with_uses(uses)
                    .with_children(children)
                    .with_location(self.node_location(node, file_path));
                Some(output.add_block_auto(block))
            }

            // Template literals
            "template_string" => {
                let uses = self.extract_used_identifiers(node, source, scope);
                let block = Block::new("template", BlockType::Template)
                    .with_uses(uses)
                    .with_location(self.node_location(node, file_path))
                    .with_code(self.node_text(node, source));
                Some(output.add_block_auto(block))
            }

            // Member expressions
            "member_expression" => {
                let uses = self.extract_used_identifiers(node, source, scope);
                let text = self.node_text(node, source);
                let block = Block::new(self.truncate(&text, 50), BlockType::Member)
                    .with_uses(uses)
                    .with_location(self.node_location(node, file_path))
                    .with_code(text);
                Some(output.add_block_auto(block))
            }

            // Subscript expressions
            "subscript_expression" => {
                let uses = self.extract_used_identifiers(node, source, scope);
                let text = self.node_text(node, source);
                let block = Block::new(self.truncate(&text, 50), BlockType::Index)
                    .with_uses(uses)
                    .with_location(self.node_location(node, file_path))
                    .with_code(text);
                Some(output.add_block_auto(block))
            }

            // Spread elements
            "spread_element" => {
                let uses = if let Some(arg) = node.child(1) {
                    self.extract_used_identifiers(&arg, source, scope)
                } else {
                    Vec::new()
                };
                let block = Block::new("...spread", BlockType::Spread)
                    .with_uses(uses)
                    .with_location(self.node_location(node, file_path))
                    .with_code(self.node_text(node, source));
                Some(output.add_block_auto(block))
            }

            // Decorators (TypeScript)
            "decorator" => {
                let name = self.get_decorator_name(node, source);
                let block = Block::new(format!("@{}", name), BlockType::Decorator)
                    .with_location(self.node_location(node, file_path))
                    .with_code(self.node_text(node, source))
                    .with_metadata(BlockMetadata {
                        target: Some(name),
                        ..Default::default()
                    });
                Some(output.add_block_auto(block))
            }

            // Skip these node types (no block needed, comments don't contribute to flow)
            "comment" | "hash_bang_line" | "{" | "}" | "(" | ")" | "[" | "]" | "," | ";" | ":"
            | "=>" => None,
            "identifier"
            | "property_identifier"
            | "string"
            | "number"
            | "true"
            | "false"
            | "null"
            | "undefined" => None,
            "type_annotation" | "type_identifier" | "predefined_type" | "generic_type" => None,

            // Default: create unknown block for unhandled cases
            _ => {
                // For complex expressions, recursively extract children
                let mut children = Vec::new();
                let mut cursor = node.walk();

                for child in node.children(&mut cursor) {
                    if let Some(child_id) =
                        self.extract_node(&child, source, file_path, output, scope, false)
                    {
                        children.push(child_id);
                    }
                }

                if children.is_empty() && !self.is_significant_node(kind) {
                    None
                } else if children.is_empty() {
                    let block = Block::new(
                        self.truncate(&self.node_text(node, source), 50),
                        BlockType::Unknown,
                    )
                    .with_location(self.node_location(node, file_path))
                    .with_code(self.node_text(node, source));
                    Some(output.add_block_auto(block))
                } else {
                    let block = Block::new(kind.to_string(), BlockType::Statement)
                        .with_children(children)
                        .with_location(self.node_location(node, file_path));
                    Some(output.add_block_auto(block))
                }
            }
        }
    }

    fn extract_function(
        &self,
        node: &Node,
        source: &str,
        file_path: &str,
        output: &mut FlowMapOutput,
        is_top_level: bool,
    ) -> Option<BlockId> {
        let is_async = self.has_child_kind(node, "async");
        let block_type = if is_async {
            BlockType::AsyncFunction
        } else {
            BlockType::Function
        };
        self.extract_function_with_type(node, source, file_path, output, block_type, is_top_level)
    }

    fn extract_function_with_type(
        &self,
        node: &Node,
        source: &str,
        file_path: &str,
        output: &mut FlowMapOutput,
        block_type: BlockType,
        is_top_level: bool,
    ) -> Option<BlockId> {
        let name = node
            .child_by_field_name("name")
            .map(|n| self.node_text(&n, source))
            .unwrap_or_else(|| "anonymous".to_string());

        let mut scope = Scope::new();
        let params = self.extract_parameters(node, source, &mut scope);
        let mut children = Vec::new();

        // Extract decorators
        let decorators = self.extract_decorators(node, source, output);
        children.extend(decorators);

        // Extract body
        if let Some(body) = node.child_by_field_name("body") {
            if let Some(body_id) =
                self.extract_node(&body, source, file_path, output, &mut scope, false)
            {
                children.push(body_id);
            }
        }

        let return_type = node
            .child_by_field_name("return_type")
            .map(|n| self.node_text(&n, source));

        let is_async = matches!(
            block_type,
            BlockType::AsyncFunction | BlockType::AsyncMethod
        );

        let block = Block::new(name, block_type)
            .with_uses(scope.used_external.into_iter().collect())
            .with_produces(params.clone())
            .with_children(children)
            .with_location(self.node_location(node, file_path))
            .with_metadata(BlockMetadata {
                is_async: Some(is_async),
                parameters: Some(params),
                return_type,
                ..Default::default()
            });

        let id = output.add_block_auto(block);
        if is_top_level {
            output.add_root(id.clone());
        }
        Some(id)
    }

    fn extract_arrow_function(
        &self,
        node: &Node,
        source: &str,
        file_path: &str,
        output: &mut FlowMapOutput,
    ) -> Option<BlockId> {
        let mut scope = Scope::new();
        let params = self.extract_parameters(node, source, &mut scope);
        let mut children = Vec::new();

        if let Some(body) = node.child_by_field_name("body") {
            if let Some(body_id) =
                self.extract_node(&body, source, file_path, output, &mut scope, false)
            {
                children.push(body_id);
            }
        }

        let return_type = node
            .child_by_field_name("return_type")
            .map(|n| self.node_text(&n, source));

        let block = Block::new("arrow", BlockType::Arrow)
            .with_uses(scope.used_external.into_iter().collect())
            .with_produces(params.clone())
            .with_children(children)
            .with_location(self.node_location(node, file_path))
            .with_metadata(BlockMetadata {
                parameters: Some(params),
                return_type,
                ..Default::default()
            });

        Some(output.add_block_auto(block))
    }

    fn extract_class(
        &self,
        node: &Node,
        source: &str,
        file_path: &str,
        output: &mut FlowMapOutput,
        is_top_level: bool,
    ) -> Option<BlockId> {
        let name = node
            .child_by_field_name("name")
            .map(|n| self.node_text(&n, source))
            .unwrap_or_else(|| "anonymous".to_string());

        let mut children = Vec::new();
        let mut scope = Scope::new();

        // Extract decorators
        let decorators = self.extract_decorators(node, source, output);
        children.extend(decorators);

        // Extract class body members
        if let Some(body) = node.child_by_field_name("body") {
            let mut cursor = body.walk();
            for child in body.children(&mut cursor) {
                if let Some(member_id) =
                    self.extract_class_member(&child, source, file_path, output, &mut scope)
                {
                    children.push(member_id);
                }
            }
        }

        let block = Block::new(name, BlockType::Class)
            .with_children(children)
            .with_location(self.node_location(node, file_path))
            .with_metadata(BlockMetadata {
                is_exported: Some(is_top_level),
                ..Default::default()
            });

        let id = output.add_block_auto(block);
        if is_top_level {
            output.add_root(id.clone());
        }
        Some(id)
    }

    fn extract_class_member(
        &self,
        node: &Node,
        source: &str,
        file_path: &str,
        output: &mut FlowMapOutput,
        scope: &mut Scope,
    ) -> Option<BlockId> {
        match node.kind() {
            "method_definition" => {
                let name = node
                    .child_by_field_name("name")
                    .map(|n| self.node_text(&n, source))
                    .unwrap_or_else(|| "method".to_string());

                let is_static = self.has_child_kind(node, "static");
                let is_async = self.has_child_kind(node, "async");
                let is_getter = self.has_child_kind(node, "get");
                let is_setter = self.has_child_kind(node, "set");

                let block_type = if name == "constructor" {
                    BlockType::Constructor
                } else if is_getter {
                    BlockType::Getter
                } else if is_setter {
                    BlockType::Setter
                } else if is_static {
                    BlockType::StaticMethod
                } else if is_async {
                    BlockType::AsyncMethod
                } else {
                    BlockType::Method
                };

                let params = self.extract_parameters(node, source, scope);
                let mut children = Vec::new();

                // Extract decorators
                let decorators = self.extract_decorators(node, source, output);
                children.extend(decorators);

                // Extract body
                if let Some(body) = node.child_by_field_name("body") {
                    if let Some(body_id) =
                        self.extract_node(&body, source, file_path, output, scope, false)
                    {
                        children.push(body_id);
                    }
                }

                let block = Block::new(name, block_type)
                    .with_produces(params.clone())
                    .with_children(children)
                    .with_location(self.node_location(node, file_path))
                    .with_metadata(BlockMetadata {
                        is_static: Some(is_static),
                        is_async: Some(is_async),
                        parameters: Some(params),
                        ..Default::default()
                    });

                Some(output.add_block_auto(block))
            }

            "field_definition" | "public_field_definition" => {
                let name = node
                    .child_by_field_name("name")
                    .or_else(|| node.child_by_field_name("property"))
                    .map(|n| self.node_text(&n, source))
                    .unwrap_or_else(|| "field".to_string());

                let is_static = self.has_child_kind(node, "static");
                let block_type = if is_static {
                    BlockType::StaticProperty
                } else {
                    BlockType::Property
                };

                let mut uses = Vec::new();
                if let Some(value) = node.child_by_field_name("value") {
                    uses = self.extract_used_identifiers(&value, source, scope);
                }

                let block = Block::new(name.clone(), block_type)
                    .with_uses(uses)
                    .with_produces(vec![name])
                    .with_location(self.node_location(node, file_path))
                    .with_code(self.node_text(node, source))
                    .with_metadata(BlockMetadata {
                        is_static: Some(is_static),
                        ..Default::default()
                    });

                Some(output.add_block_auto(block))
            }

            "decorator" => {
                let name = self.get_decorator_name(node, source);
                let block = Block::new(format!("@{}", name), BlockType::Decorator)
                    .with_location(self.node_location(node, file_path))
                    .with_code(self.node_text(node, source));
                Some(output.add_block_auto(block))
            }

            _ => None,
        }
    }

    fn extract_variable_declaration(
        &self,
        node: &Node,
        source: &str,
        file_path: &str,
        output: &mut FlowMapOutput,
        scope: &mut Scope,
        is_top_level: bool,
    ) -> Option<BlockId> {
        let kind_str = node
            .child(0)
            .map(|n| self.node_text(&n, source))
            .unwrap_or_default();
        let block_type = match kind_str.as_str() {
            "const" => BlockType::Const,
            "let" => BlockType::Let,
            _ => BlockType::Variable,
        };

        let mut declarators = Vec::new();
        let mut cursor = node.walk();

        for child in node.children(&mut cursor) {
            if child.kind() == "variable_declarator" {
                let name = child
                    .child_by_field_name("name")
                    .map(|n| self.extract_binding_pattern(&n, source, scope))
                    .unwrap_or_default();

                let mut uses = Vec::new();
                let mut value_children = Vec::new();

                if let Some(value) = child.child_by_field_name("value") {
                    uses = self.extract_used_identifiers(&value, source, scope);

                    // Extract complex expressions in value
                    if let Some(val_id) =
                        self.extract_node(&value, source, file_path, output, scope, false)
                    {
                        value_children.push(val_id);
                    }
                }

                for n in &name {
                    scope.define(n);
                }

                declarators.push((name, uses, value_children));
            }
        }

        // If single declarator, create one block
        if declarators.len() == 1 {
            let (names, uses, children) = declarators.into_iter().next().unwrap();
            let name = names.join(", ");

            let block = Block::new(name.clone(), block_type)
                .with_uses(uses)
                .with_produces(names)
                .with_children(children)
                .with_location(self.node_location(node, file_path))
                .with_code(self.node_text(node, source));

            let id = output.add_block_auto(block);
            if is_top_level {
                output.add_root(id.clone());
            }
            return Some(id);
        }

        // Multiple declarators: create parent block with children
        let mut children = Vec::new();
        for (names, uses, value_children) in declarators {
            let name = names.join(", ");

            let decl_block = Block::new(name.clone(), block_type.clone())
                .with_uses(uses)
                .with_produces(names)
                .with_children(value_children);

            children.push(output.add_block_auto(decl_block));
        }

        let block = Block::new("declarations", BlockType::Block)
            .with_children(children)
            .with_location(self.node_location(node, file_path));

        let id = output.add_block_auto(block);
        if is_top_level {
            output.add_root(id.clone());
        }
        Some(id)
    }

    fn extract_if_statement(
        &self,
        node: &Node,
        source: &str,
        file_path: &str,
        output: &mut FlowMapOutput,
        scope: &mut Scope,
    ) -> Option<BlockId> {
        let condition = node.child_by_field_name("condition");
        let condition_text = condition
            .map(|c| self.node_text(&c, source))
            .unwrap_or_default();

        let mut uses = condition
            .map(|c| self.extract_used_identifiers(&c, source, scope))
            .unwrap_or_default();
        let mut children = Vec::new();

        // Then branch
        if let Some(consequence) = node.child_by_field_name("consequence") {
            if let Some(then_id) =
                self.extract_node(&consequence, source, file_path, output, scope, false)
            {
                children.push(then_id);
            }
        }

        // Else branch
        if let Some(alternative) = node.child_by_field_name("alternative") {
            if alternative.kind() == "if_statement" {
                // else-if chain
                if let Some(else_if_id) =
                    self.extract_if_statement(&alternative, source, file_path, output, scope)
                {
                    children.push(else_if_id);
                }
            } else if alternative.kind() == "else_clause" {
                // else block
                if let Some(else_body) = alternative
                    .child_by_field_name("body")
                    .or_else(|| alternative.child(1))
                {
                    if let Some(else_id) =
                        self.extract_node(&else_body, source, file_path, output, scope, false)
                    {
                        let else_block = Block::new("else", BlockType::Else)
                            .with_children(vec![else_id])
                            .with_location(self.node_location(&alternative, file_path));
                        children.push(output.add_block_auto(else_block));
                    }
                }
            } else {
                if let Some(else_id) =
                    self.extract_node(&alternative, source, file_path, output, scope, false)
                {
                    let else_block = Block::new("else", BlockType::Else)
                        .with_children(vec![else_id])
                        .with_location(self.node_location(&alternative, file_path));
                    children.push(output.add_block_auto(else_block));
                }
            }
        }

        let block = Block::new(
            format!("if {}", self.truncate(&condition_text, 30)),
            BlockType::If,
        )
        .with_uses(uses)
        .with_children(children)
        .with_location(self.node_location(node, file_path))
        .with_metadata(BlockMetadata {
            condition: Some(condition_text),
            ..Default::default()
        });

        Some(output.add_block_auto(block))
    }

    fn extract_switch_statement(
        &self,
        node: &Node,
        source: &str,
        file_path: &str,
        output: &mut FlowMapOutput,
        scope: &mut Scope,
    ) -> Option<BlockId> {
        let value = node.child_by_field_name("value");
        let value_text = value
            .map(|v| self.node_text(&v, source))
            .unwrap_or_default();
        let uses = value
            .map(|v| self.extract_used_identifiers(&v, source, scope))
            .unwrap_or_default();

        let mut children = Vec::new();

        if let Some(body) = node.child_by_field_name("body") {
            let mut cursor = body.walk();
            for child in body.children(&mut cursor) {
                match child.kind() {
                    "switch_case" => {
                        let case_value = child
                            .child_by_field_name("value")
                            .map(|v| self.node_text(&v, source));

                        let mut case_children = Vec::new();
                        let mut case_cursor = child.walk();
                        for case_child in child.children(&mut case_cursor) {
                            if case_child.kind() != "case" && case_child.kind() != ":" {
                                if let Some(id) = self.extract_node(
                                    &case_child,
                                    source,
                                    file_path,
                                    output,
                                    scope,
                                    false,
                                ) {
                                    case_children.push(id);
                                }
                            }
                        }

                        let case_block = Block::new(
                            format!("case {}", case_value.unwrap_or_else(|| "?".to_string())),
                            BlockType::Case,
                        )
                        .with_children(case_children)
                        .with_location(self.node_location(&child, file_path));

                        children.push(output.add_block_auto(case_block));
                    }
                    "switch_default" => {
                        let mut default_children = Vec::new();
                        let mut default_cursor = child.walk();
                        for default_child in child.children(&mut default_cursor) {
                            if default_child.kind() != "default" && default_child.kind() != ":" {
                                if let Some(id) = self.extract_node(
                                    &default_child,
                                    source,
                                    file_path,
                                    output,
                                    scope,
                                    false,
                                ) {
                                    default_children.push(id);
                                }
                            }
                        }

                        let default_block = Block::new("default", BlockType::Default)
                            .with_children(default_children)
                            .with_location(self.node_location(&child, file_path));

                        children.push(output.add_block_auto(default_block));
                    }
                    _ => {}
                }
            }
        }

        let block = Block::new(
            format!("switch {}", self.truncate(&value_text, 30)),
            BlockType::Switch,
        )
        .with_uses(uses)
        .with_children(children)
        .with_location(self.node_location(node, file_path))
        .with_metadata(BlockMetadata {
            condition: Some(value_text),
            ..Default::default()
        });

        Some(output.add_block_auto(block))
    }

    fn extract_try_statement(
        &self,
        node: &Node,
        source: &str,
        file_path: &str,
        output: &mut FlowMapOutput,
        scope: &mut Scope,
    ) -> Option<BlockId> {
        let mut children = Vec::new();

        // Try block
        if let Some(body) = node.child_by_field_name("body") {
            if let Some(try_id) = self.extract_node(&body, source, file_path, output, scope, false)
            {
                let try_block = Block::new("try", BlockType::Try)
                    .with_children(vec![try_id])
                    .with_location(self.node_location(&body, file_path));
                children.push(output.add_block_auto(try_block));
            }
        }

        // Catch clause
        if let Some(handler) = node.child_by_field_name("handler") {
            let catch_param = handler
                .child_by_field_name("parameter")
                .map(|p| self.node_text(&p, source));

            let mut catch_children = Vec::new();
            if let Some(catch_body) = handler.child_by_field_name("body") {
                if let Some(catch_body_id) =
                    self.extract_node(&catch_body, source, file_path, output, scope, false)
                {
                    catch_children.push(catch_body_id);
                }
            }

            let catch_block = Block::new(
                format!(
                    "catch({})",
                    catch_param.clone().unwrap_or_else(|| "e".to_string())
                ),
                BlockType::Catch,
            )
            .with_produces(catch_param.into_iter().collect())
            .with_children(catch_children)
            .with_location(self.node_location(&handler, file_path));

            children.push(output.add_block_auto(catch_block));
        }

        // Finally clause
        if let Some(finalizer) = node.child_by_field_name("finalizer") {
            if let Some(finally_body) = finalizer.child_by_field_name("body") {
                if let Some(finally_id) =
                    self.extract_node(&finally_body, source, file_path, output, scope, false)
                {
                    let finally_block = Block::new("finally", BlockType::Finally)
                        .with_children(vec![finally_id])
                        .with_location(self.node_location(&finalizer, file_path));
                    children.push(output.add_block_auto(finally_block));
                }
            }
        }

        let block = Block::new("try...catch", BlockType::TryCatch)
            .with_children(children)
            .with_location(self.node_location(node, file_path));

        Some(output.add_block_auto(block))
    }

    fn extract_loop(
        &self,
        node: &Node,
        source: &str,
        file_path: &str,
        output: &mut FlowMapOutput,
        scope: &mut Scope,
        loop_type: BlockType,
    ) -> Option<BlockId> {
        let mut uses = Vec::new();
        let mut produces = Vec::new();
        let mut children = Vec::new();

        // Extract loop variable (for for-of/for-in)
        if let Some(left) = node.child_by_field_name("left") {
            let vars = self.extract_binding_pattern(&left, source, scope);
            produces.extend(vars.clone());
            for v in vars {
                scope.define(&v);
            }
        }

        // Extract initializer (for for loops)
        if let Some(init) = node.child_by_field_name("initializer") {
            if let Some(init_id) = self.extract_node(&init, source, file_path, output, scope, false)
            {
                children.push(init_id);
            }
        }

        // Extract condition
        if let Some(condition) = node.child_by_field_name("condition") {
            uses.extend(self.extract_used_identifiers(&condition, source, scope));
        }

        // Extract right side (for for-of/for-in)
        if let Some(right) = node.child_by_field_name("right") {
            uses.extend(self.extract_used_identifiers(&right, source, scope));
        }

        // Extract increment
        if let Some(increment) = node.child_by_field_name("increment") {
            uses.extend(self.extract_used_identifiers(&increment, source, scope));
        }

        // Extract body
        if let Some(body) = node.child_by_field_name("body") {
            if let Some(body_id) = self.extract_node(&body, source, file_path, output, scope, false)
            {
                children.push(body_id);
            }
        }

        let condition_text = node
            .child_by_field_name("condition")
            .map(|c| self.node_text(&c, source))
            .or_else(|| {
                node.child_by_field_name("right")
                    .map(|r| self.node_text(&r, source))
            });

        let name = match loop_type {
            BlockType::For => "for".to_string(),
            BlockType::ForIn => "for...in".to_string(),
            BlockType::ForOf => "for...of".to_string(),
            BlockType::ForAwait => "for await...of".to_string(),
            BlockType::While => format!("while {}", condition_text.clone().unwrap_or_default()),
            BlockType::DoWhile => {
                format!("do...while {}", condition_text.clone().unwrap_or_default())
            }
            _ => "loop".to_string(),
        };

        let block = Block::new(name, loop_type)
            .with_uses(uses)
            .with_produces(produces)
            .with_children(children)
            .with_location(self.node_location(node, file_path))
            .with_metadata(BlockMetadata {
                condition: condition_text,
                ..Default::default()
            });

        Some(output.add_block_auto(block))
    }

    fn extract_call_expression(
        &self,
        node: &Node,
        source: &str,
        file_path: &str,
        output: &mut FlowMapOutput,
        scope: &mut Scope,
    ) -> Option<BlockId> {
        let callee = node.child_by_field_name("function")?;
        let callee_text = self.node_text(&callee, source);

        let mut uses = Vec::new();
        let mut children = Vec::new();

        // Track callee as used data
        uses.extend(self.extract_used_identifiers(&callee, source, scope));

        // Extract arguments
        if let Some(args) = node.child_by_field_name("arguments") {
            let mut cursor = args.walk();
            for arg in args.children(&mut cursor) {
                if arg.kind() == "(" || arg.kind() == ")" || arg.kind() == "," {
                    continue;
                }
                uses.extend(self.extract_used_identifiers(&arg, source, scope));

                // Extract complex expressions in arguments
                if self.is_complex_expression(&arg) {
                    if let Some(arg_id) =
                        self.extract_node(&arg, source, file_path, output, scope, false)
                    {
                        children.push(arg_id);
                    }
                }
            }
        }

        let (block_type, name) = if callee.kind() == "member_expression" {
            let object = callee
                .child_by_field_name("object")
                .map(|o| self.node_text(&o, source))
                .unwrap_or_default();
            let method = callee
                .child_by_field_name("property")
                .map(|p| self.node_text(&p, source))
                .unwrap_or_default();
            (BlockType::MethodCall, format!("{}.{}", object, method))
        } else {
            (BlockType::Call, callee_text)
        };

        let block = Block::new(name, block_type)
            .with_uses(uses)
            .with_children(children)
            .with_location(self.node_location(node, file_path))
            .with_code(self.node_text(node, source));

        Some(output.add_block_auto(block))
    }

    fn extract_await_expression(
        &self,
        node: &Node,
        source: &str,
        file_path: &str,
        output: &mut FlowMapOutput,
        scope: &mut Scope,
    ) -> Option<BlockId> {
        let argument = node.child(1)?;
        let mut children = Vec::new();

        // Extract the awaited expression
        if let Some(arg_id) = self.extract_node(&argument, source, file_path, output, scope, false)
        {
            children.push(arg_id);
        }

        let uses = self.extract_used_identifiers(&argument, source, scope);
        let label = self.truncate(&self.node_text(&argument, source), 40);

        let block = Block::new(format!("await {}", label), BlockType::AwaitCall)
            .with_uses(uses)
            .with_children(children)
            .with_location(self.node_location(node, file_path));

        Some(output.add_block_auto(block))
    }

    fn extract_new_expression(
        &self,
        node: &Node,
        source: &str,
        file_path: &str,
        output: &mut FlowMapOutput,
        scope: &mut Scope,
    ) -> Option<BlockId> {
        let constructor = node.child_by_field_name("constructor")?;
        let constructor_text = self.node_text(&constructor, source);

        let mut uses = vec![constructor_text.clone()];
        let mut children = Vec::new();

        // Extract arguments
        if let Some(args) = node.child_by_field_name("arguments") {
            let mut cursor = args.walk();
            for arg in args.children(&mut cursor) {
                if arg.kind() == "(" || arg.kind() == ")" || arg.kind() == "," {
                    continue;
                }
                uses.extend(self.extract_used_identifiers(&arg, source, scope));

                if self.is_complex_expression(&arg) {
                    if let Some(arg_id) =
                        self.extract_node(&arg, source, file_path, output, scope, false)
                    {
                        children.push(arg_id);
                    }
                }
            }
        }

        let block = Block::new(format!("new {}", constructor_text), BlockType::New)
            .with_uses(uses)
            .with_children(children)
            .with_location(self.node_location(node, file_path))
            .with_code(self.node_text(node, source));

        Some(output.add_block_auto(block))
    }

    fn extract_assignment(
        &self,
        node: &Node,
        source: &str,
        file_path: &str,
        output: &mut FlowMapOutput,
        scope: &mut Scope,
    ) -> Option<BlockId> {
        let left = node.child_by_field_name("left")?;
        let right = node.child_by_field_name("right")?;

        let produces = self.extract_binding_pattern(&left, source, scope);
        let uses = self.extract_used_identifiers(&right, source, scope);

        let mut children = Vec::new();
        if self.is_complex_expression(&right) {
            if let Some(right_id) =
                self.extract_node(&right, source, file_path, output, scope, false)
            {
                children.push(right_id);
            }
        }

        for p in &produces {
            scope.define(p);
        }

        let left_text = self.node_text(&left, source);
        let block = Block::new(left_text, BlockType::Assignment)
            .with_uses(uses)
            .with_produces(produces)
            .with_children(children)
            .with_location(self.node_location(node, file_path))
            .with_code(self.node_text(node, source));

        Some(output.add_block_auto(block))
    }

    fn extract_return_statement(
        &self,
        node: &Node,
        source: &str,
        file_path: &str,
        output: &mut FlowMapOutput,
        scope: &mut Scope,
    ) -> Option<BlockId> {
        let mut uses = Vec::new();
        let mut children = Vec::new();

        // Find return value (everything after 'return' keyword)
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() != "return" && child.kind() != ";" {
                uses.extend(self.extract_used_identifiers(&child, source, scope));
                if self.is_complex_expression(&child) {
                    if let Some(child_id) =
                        self.extract_node(&child, source, file_path, output, scope, false)
                    {
                        children.push(child_id);
                    }
                }
            }
        }

        let block = Block::new("return", BlockType::Return)
            .with_uses(uses)
            .with_children(children)
            .with_location(self.node_location(node, file_path))
            .with_code(self.node_text(node, source));

        Some(output.add_block_auto(block))
    }

    fn extract_throw_statement(
        &self,
        node: &Node,
        source: &str,
        file_path: &str,
        output: &mut FlowMapOutput,
        scope: &mut Scope,
    ) -> Option<BlockId> {
        let mut uses = Vec::new();
        let mut children = Vec::new();

        // Find thrown expression
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() != "throw" && child.kind() != ";" {
                uses.extend(self.extract_used_identifiers(&child, source, scope));
                if let Some(child_id) =
                    self.extract_node(&child, source, file_path, output, scope, false)
                {
                    children.push(child_id);
                }
            }
        }

        let block = Block::new("throw", BlockType::Throw)
            .with_uses(uses)
            .with_children(children)
            .with_location(self.node_location(node, file_path))
            .with_code(self.node_text(node, source));

        Some(output.add_block_auto(block))
    }

    fn extract_enum(
        &self,
        node: &Node,
        source: &str,
        file_path: &str,
        output: &mut FlowMapOutput,
        is_top_level: bool,
    ) -> Option<BlockId> {
        let name = node
            .child_by_field_name("name")
            .map(|n| self.node_text(&n, source))
            .unwrap_or_else(|| "enum".to_string());

        let mut children = Vec::new();

        if let Some(body) = node.child_by_field_name("body") {
            let mut cursor = body.walk();
            for child in body.children(&mut cursor) {
                if child.kind() == "enum_assignment" || child.kind() == "property_identifier" {
                    let member_name = child
                        .child_by_field_name("name")
                        .or_else(|| Some(child))
                        .map(|n| self.node_text(&n, source))
                        .unwrap_or_default();

                    let member_block = Block::new(member_name, BlockType::EnumMember)
                        .with_location(self.node_location(&child, file_path));
                    children.push(output.add_block_auto(member_block));
                }
            }
        }

        let block = Block::new(name, BlockType::Enum)
            .with_children(children)
            .with_location(self.node_location(node, file_path))
            .with_code(self.node_text(node, source));

        let id = output.add_block_auto(block);
        if is_top_level {
            output.add_root(id.clone());
        }
        Some(id)
    }

    // Helper methods

    fn extract_parameters(&self, node: &Node, source: &str, scope: &mut Scope) -> Vec<String> {
        let mut params = Vec::new();

        if let Some(params_node) = node.child_by_field_name("parameters") {
            let mut cursor = params_node.walk();
            for child in params_node.children(&mut cursor) {
                match child.kind() {
                    "required_parameter" | "optional_parameter" | "rest_parameter"
                    | "identifier" => {
                        let names = self.extract_binding_pattern(&child, source, scope);
                        for name in names {
                            scope.define(&name);
                            params.push(name);
                        }
                    }
                    _ => {}
                }
            }
        }

        params
    }

    fn extract_binding_pattern(&self, node: &Node, source: &str, scope: &mut Scope) -> Vec<String> {
        let mut names = Vec::new();

        match node.kind() {
            "identifier" | "shorthand_property_identifier_pattern" | "property_identifier" => {
                names.push(self.node_text(node, source));
            }
            "object_pattern" | "array_pattern" => {
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    names.extend(self.extract_binding_pattern(&child, source, scope));
                }
            }
            "assignment_pattern" => {
                if let Some(left) = node.child_by_field_name("left") {
                    names.extend(self.extract_binding_pattern(&left, source, scope));
                }
            }
            "rest_pattern" | "rest_element" => {
                if let Some(arg) = node.child(1) {
                    names.extend(self.extract_binding_pattern(&arg, source, scope));
                }
            }
            "pair_pattern" | "pair" => {
                if let Some(value) = node.child_by_field_name("value") {
                    names.extend(self.extract_binding_pattern(&value, source, scope));
                }
            }
            "required_parameter" | "optional_parameter" => {
                if let Some(pattern) = node.child_by_field_name("pattern") {
                    names.extend(self.extract_binding_pattern(&pattern, source, scope));
                }
            }
            "rest_parameter" => {
                if let Some(pattern) = node.child(1) {
                    names.extend(self.extract_binding_pattern(&pattern, source, scope));
                }
            }
            _ => {}
        }

        names
    }

    fn extract_used_identifiers(
        &self,
        node: &Node,
        source: &str,
        scope: &mut Scope,
    ) -> Vec<String> {
        let mut identifiers = Vec::new();

        match node.kind() {
            "identifier" => {
                let name = self.node_text(node, source);
                // Skip built-in globals
                if !self.is_builtin(&name) && !scope.is_defined(&name) {
                    identifiers.push(name);
                }
            }
            "member_expression" => {
                // Only extract the root object
                if let Some(object) = node.child_by_field_name("object") {
                    identifiers.extend(self.extract_used_identifiers(&object, source, scope));
                }
            }
            "this" => {
                identifiers.push("this".to_string());
            }
            _ => {
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    identifiers.extend(self.extract_used_identifiers(&child, source, scope));
                }
            }
        }

        // Deduplicate
        let mut seen = HashSet::new();
        identifiers.retain(|x| seen.insert(x.clone()));

        identifiers
    }

    fn extract_import_names(&self, node: &Node, source: &str) -> Vec<String> {
        let mut names = Vec::new();

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "import_clause" => {
                    let mut clause_cursor = child.walk();
                    for clause_child in child.children(&mut clause_cursor) {
                        match clause_child.kind() {
                            "identifier" => {
                                names.push(self.node_text(&clause_child, source));
                            }
                            "named_imports" => {
                                let mut imports_cursor = clause_child.walk();
                                for import_child in clause_child.children(&mut imports_cursor) {
                                    if import_child.kind() == "import_specifier" {
                                        if let Some(alias) =
                                            import_child.child_by_field_name("alias")
                                        {
                                            names.push(self.node_text(&alias, source));
                                        } else if let Some(name_node) =
                                            import_child.child_by_field_name("name")
                                        {
                                            names.push(self.node_text(&name_node, source));
                                        }
                                    }
                                }
                            }
                            "namespace_import" => {
                                if let Some(name) = clause_child.child(2) {
                                    names.push(self.node_text(&name, source));
                                }
                            }
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        }

        names
    }

    fn get_import_source(&self, node: &Node, source: &str) -> Option<String> {
        node.child_by_field_name("source").map(|s| {
            let text = self.node_text(&s, source);
            text.trim_matches(|c| c == '"' || c == '\'').to_string()
        })
    }

    fn extract_decorators(
        &self,
        node: &Node,
        source: &str,
        output: &mut FlowMapOutput,
    ) -> Vec<BlockId> {
        let mut decorators = Vec::new();

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "decorator" {
                let name = self.get_decorator_name(&child, source);
                let block = Block::new(format!("@{}", name), BlockType::Decorator)
                    .with_code(self.node_text(&child, source));
                decorators.push(output.add_block_auto(block));
            }
        }

        decorators
    }

    fn get_decorator_name(&self, node: &Node, source: &str) -> String {
        if let Some(expr) = node.child(1) {
            match expr.kind() {
                "identifier" => self.node_text(&expr, source),
                "call_expression" => {
                    if let Some(callee) = expr.child_by_field_name("function") {
                        self.node_text(&callee, source)
                    } else {
                        "decorator".to_string()
                    }
                }
                _ => self.node_text(&expr, source),
            }
        } else {
            "decorator".to_string()
        }
    }

    fn has_child_kind(&self, node: &Node, kind: &str) -> bool {
        let mut cursor = node.walk();
        node.children(&mut cursor).any(|c| c.kind() == kind)
    }

    fn is_complex_expression(&self, node: &Node) -> bool {
        matches!(
            node.kind(),
            "call_expression"
                | "await_expression"
                | "new_expression"
                | "arrow_function"
                | "function_expression"
                | "object"
                | "array"
                | "ternary_expression"
        )
    }

    fn is_significant_node(&self, kind: &str) -> bool {
        matches!(
            kind,
            "function_declaration"
                | "class_declaration"
                | "method_definition"
                | "export_statement"
                | "import_statement"
        )
    }

    fn is_builtin(&self, name: &str) -> bool {
        matches!(
            name,
            "console"
                | "window"
                | "document"
                | "global"
                | "globalThis"
                | "process"
                | "require"
                | "module"
                | "exports"
                | "__dirname"
                | "__filename"
                | "Promise"
                | "Array"
                | "Object"
                | "String"
                | "Number"
                | "Boolean"
                | "Date"
                | "Math"
                | "JSON"
                | "Error"
                | "TypeError"
                | "RangeError"
                | "Map"
                | "Set"
                | "WeakMap"
                | "WeakSet"
                | "Symbol"
                | "BigInt"
                | "Reflect"
                | "Proxy"
                | "Intl"
                | "ArrayBuffer"
                | "DataView"
                | "Uint8Array"
                | "Int32Array"
                | "Float64Array"
                | "setTimeout"
                | "setInterval"
                | "clearTimeout"
                | "clearInterval"
                | "fetch"
                | "URL"
                | "URLSearchParams"
                | "Headers"
                | "Request"
                | "Response"
                | "undefined"
                | "null"
                | "NaN"
                | "Infinity"
        )
    }

    fn node_text(&self, node: &Node, source: &str) -> String {
        source[node.byte_range()].to_string()
    }

    fn node_location(&self, node: &Node, file_path: &str) -> Location {
        let start = node.start_position();
        let end = node.end_position();
        Location {
            file: file_path.to_string(),
            start_line: start.row as u32 + 1,
            end_line: end.row as u32 + 1,
            start_col: Some(start.column as u32),
            end_col: Some(end.column as u32),
        }
    }

    fn truncate(&self, s: &str, max: usize) -> String {
        if s.chars().count() > max {
            let truncated: String = s.chars().take(max).collect();
            format!("{}...", truncated)
        } else {
            s.to_string()
        }
    }
}

impl Default for BlockExtractor {
    fn default() -> Self {
        Self::new().expect("Failed to create BlockExtractor")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_function() {
        let mut extractor = BlockExtractor::new().unwrap();
        let source = r#"
function add(a, b) {
    return a + b;
}
"#;

        let output = extractor.parse_file(source, "test.ts").unwrap();
        assert!(output.block_count() > 0);
        assert!(!output.root.is_empty());

        let json = serde_json::to_string_pretty(&output).unwrap();
        assert!(json.contains("add"));
        assert!(json.contains("function"));
    }

    #[test]
    fn test_class_extraction() {
        let mut extractor = BlockExtractor::new().unwrap();
        let source = r#"
class UserService {
    constructor(private db: Database) {}

    async getUser(id: string) {
        const user = await this.db.findById(id);
        return user;
    }
}
"#;

        let output = extractor.parse_file(source, "test.ts").unwrap();
        let json = serde_json::to_string_pretty(&output).unwrap();

        assert!(json.contains("UserService"));
        assert!(json.contains("getUser"));
        assert!(json.contains("constructor"));
    }

    #[test]
    fn test_data_flow() {
        let mut extractor = BlockExtractor::new().unwrap();
        let source = r#"
function createOrder(userId, items) {
    const user = getUser(userId);
    const total = calculatePrice(user, items);
    if (total > 1000) {
        applyDiscount(total, user);
    }
    return { user, total };
}
"#;

        let output = extractor.parse_file(source, "test.ts").unwrap();

        // Check that we're tracking data flow
        let mut found_uses_user = false;
        let mut found_produces_user = false;

        for block in output.library.values() {
            if block.uses_data.contains(&"userId".to_string()) {
                found_uses_user = true;
            }
            if block.produces_data.contains(&"user".to_string()) {
                found_produces_user = true;
            }
        }

        assert!(
            found_uses_user || found_produces_user,
            "Data flow should be tracked"
        );
    }
}
