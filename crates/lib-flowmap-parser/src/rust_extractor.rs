use lib_flowmap_core::{Block, BlockId, BlockMetadata, BlockType, FlowMapOutput, Location};
use std::collections::HashSet;
use tree_sitter::{Node, Parser};

/// Block-based extractor for Rust
/// Parses complete AST into flat block library with data flow tracking
pub struct RustBlockExtractor {
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

impl RustBlockExtractor {
    pub fn new() -> crate::Result<Self> {
        let mut parser = Parser::new();
        let language = tree_sitter_rust::LANGUAGE;
        parser
            .set_language(&language.into())
            .map_err(|e| crate::ParseError::TreeSitter(e.to_string()))?;

        Ok(Self { parser })
    }

    /// Parse a file and return the block-based output
    pub fn parse_file(&mut self, source: &str, file_path: &str) -> crate::Result<FlowMapOutput> {
        let tree = self
            .parser
            .parse(source, None)
            .ok_or_else(|| crate::ParseError::ParseFailed {
                path: file_path.to_string(),
            })?;

        let mut output = FlowMapOutput::new()
            .with_file(file_path.to_string())
            .with_language("rust".to_string());

        let root = tree.root_node();
        let mut scope = Scope::new();

        // Extract all top-level blocks
        self.extract_node(&root, source, file_path, &mut output, &mut scope, true);

        Ok(output)
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
            // Source file (root)
            "source_file" => {
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

            // Use declarations (imports)
            "use_declaration" => {
                let imports = self.extract_use_names(node, source);
                let target = self.get_use_path(node, source);

                for name in &imports {
                    scope.define(name);
                }

                let block = Block::new(
                    format!("use {}", target.as_deref().unwrap_or("?")),
                    BlockType::Import,
                )
                .with_produces(imports)
                .with_location(self.node_location(node, file_path))
                .with_code(self.node_text(node, source))
                .with_metadata(BlockMetadata {
                    target,
                    ..Default::default()
                });

                let id = output.add_block_auto(block);
                if is_top_level {
                    output.add_root(id.clone());
                }
                Some(id)
            }

            // Module declarations
            "mod_item" => {
                let name = node
                    .child_by_field_name("name")
                    .map(|n| self.node_text(&n, source))
                    .unwrap_or_else(|| "mod".to_string());

                let mut children = Vec::new();

                // Extract module body if inline
                if let Some(body) = node.child_by_field_name("body") {
                    let mut cursor = body.walk();
                    for child in body.children(&mut cursor) {
                        if let Some(child_id) =
                            self.extract_node(&child, source, file_path, output, scope, false)
                        {
                            children.push(child_id);
                        }
                    }
                }

                let block = Block::new(name, BlockType::Module)
                    .with_children(children)
                    .with_location(self.node_location(node, file_path))
                    .with_code(self.node_text(node, source));

                let id = output.add_block_auto(block);
                if is_top_level {
                    output.add_root(id.clone());
                }
                Some(id)
            }

            // Function declarations
            "function_item" => self.extract_function(node, source, file_path, output, is_top_level),

            // Impl blocks
            "impl_item" => self.extract_impl(node, source, file_path, output, is_top_level),

            // Trait definitions
            "trait_item" => self.extract_trait(node, source, file_path, output, is_top_level),

            // Struct definitions
            "struct_item" => self.extract_struct(node, source, file_path, output, is_top_level),

            // Enum definitions
            "enum_item" => self.extract_enum(node, source, file_path, output, is_top_level),

            // Type alias
            "type_item" => {
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

            // Const items
            "const_item" => {
                let name = node
                    .child_by_field_name("name")
                    .map(|n| self.node_text(&n, source))
                    .unwrap_or_else(|| "const".to_string());

                scope.define(&name);

                let mut uses = Vec::new();
                if let Some(value) = node.child_by_field_name("value") {
                    uses = self.extract_used_identifiers(&value, source, scope);
                }

                let block = Block::new(name.clone(), BlockType::Const)
                    .with_uses(uses)
                    .with_produces(vec![name])
                    .with_location(self.node_location(node, file_path))
                    .with_code(self.node_text(node, source));

                let id = output.add_block_auto(block);
                if is_top_level {
                    output.add_root(id.clone());
                }
                Some(id)
            }

            // Static items
            "static_item" => {
                let name = node
                    .child_by_field_name("name")
                    .map(|n| self.node_text(&n, source))
                    .unwrap_or_else(|| "static".to_string());

                scope.define(&name);

                let mut uses = Vec::new();
                if let Some(value) = node.child_by_field_name("value") {
                    uses = self.extract_used_identifiers(&value, source, scope);
                }

                let is_mutable = self.has_child_kind(node, "mutable_specifier");

                let block = Block::new(name.clone(), BlockType::StaticProperty)
                    .with_uses(uses)
                    .with_produces(vec![name])
                    .with_location(self.node_location(node, file_path))
                    .with_code(self.node_text(node, source))
                    .with_metadata(BlockMetadata {
                        is_static: Some(true),
                        visibility: if is_mutable {
                            Some("mut".to_string())
                        } else {
                            None
                        },
                        ..Default::default()
                    });

                let id = output.add_block_auto(block);
                if is_top_level {
                    output.add_root(id.clone());
                }
                Some(id)
            }

            // Let declarations
            "let_declaration" => {
                self.extract_let_declaration(node, source, file_path, output, scope)
            }

            // Expression statements
            "expression_statement" => {
                if let Some(expr) = node.child(0) {
                    self.extract_node(&expr, source, file_path, output, scope, false)
                } else {
                    None
                }
            }

            // If expressions
            "if_expression" => self.extract_if_expression(node, source, file_path, output, scope),

            // Match expressions
            "match_expression" => {
                self.extract_match_expression(node, source, file_path, output, scope)
            }

            // Loop expressions
            "loop_expression" => {
                self.extract_loop(node, source, file_path, output, scope, BlockType::While)
            }
            "while_expression" => {
                self.extract_loop(node, source, file_path, output, scope, BlockType::While)
            }
            "for_expression" => {
                self.extract_for_loop(node, source, file_path, output, scope)
            }

            // Call expressions
            "call_expression" => {
                self.extract_call_expression(node, source, file_path, output, scope)
            }

            // Method call expressions
            "method_call_expression" => {
                self.extract_method_call(node, source, file_path, output, scope)
            }

            // Await expressions
            "await_expression" => {
                self.extract_await_expression(node, source, file_path, output, scope)
            }

            // Try expressions (?)
            "try_expression" => {
                let mut children = Vec::new();
                if let Some(inner) = node.child(0) {
                    if let Some(inner_id) =
                        self.extract_node(&inner, source, file_path, output, scope, false)
                    {
                        children.push(inner_id);
                    }
                }

                let uses = self.extract_used_identifiers(node, source, scope);

                let block = Block::new("?", BlockType::Expression)
                    .with_uses(uses)
                    .with_children(children)
                    .with_location(self.node_location(node, file_path))
                    .with_code(self.node_text(node, source));

                Some(output.add_block_auto(block))
            }

            // Return expressions
            "return_expression" => {
                self.extract_return_expression(node, source, file_path, output, scope)
            }

            // Break expressions
            "break_expression" => {
                let label = node
                    .child_by_field_name("label")
                    .map(|n| self.node_text(&n, source));

                let mut uses = Vec::new();
                if let Some(value) = node.child_by_field_name("value") {
                    uses = self.extract_used_identifiers(&value, source, scope);
                }

                let block =
                    Block::new(label.unwrap_or_else(|| "break".to_string()), BlockType::Break)
                        .with_uses(uses)
                        .with_location(self.node_location(node, file_path));

                Some(output.add_block_auto(block))
            }

            // Continue expressions
            "continue_expression" => {
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

            // Closure expressions
            "closure_expression" => {
                self.extract_closure(node, source, file_path, output, scope)
            }

            // Macro invocations
            "macro_invocation" => {
                self.extract_macro_invocation(node, source, file_path, output, scope)
            }

            // Assignment expressions
            "assignment_expression" | "compound_assignment_expr" => {
                self.extract_assignment(node, source, file_path, output, scope)
            }

            // Block expressions
            "block" => {
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
                    Some(children.into_iter().next().unwrap())
                } else {
                    let block = Block::new("block", BlockType::Block)
                        .with_children(children)
                        .with_location(self.node_location(node, file_path));
                    Some(output.add_block_auto(block))
                }
            }

            // Unsafe block
            "unsafe_block" => {
                let mut children = Vec::new();

                if let Some(body) = node.child_by_field_name("body") {
                    if let Some(body_id) =
                        self.extract_node(&body, source, file_path, output, scope, false)
                    {
                        children.push(body_id);
                    }
                }

                let block = Block::new("unsafe", BlockType::Block)
                    .with_children(children)
                    .with_location(self.node_location(node, file_path));

                Some(output.add_block_auto(block))
            }

            // Async block
            "async_block" => {
                let mut children = Vec::new();

                if let Some(body) = node.child_by_field_name("body") {
                    if let Some(body_id) =
                        self.extract_node(&body, source, file_path, output, scope, false)
                    {
                        children.push(body_id);
                    }
                }

                let block = Block::new("async", BlockType::Block)
                    .with_children(children)
                    .with_location(self.node_location(node, file_path))
                    .with_metadata(BlockMetadata {
                        is_async: Some(true),
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
            "unary_expression" => {
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

            // Reference expressions
            "reference_expression" => {
                let uses = self.extract_used_identifiers(node, source, scope);
                let block = Block::new(
                    self.truncate(&self.node_text(node, source), 50),
                    BlockType::Expression,
                )
                .with_uses(uses)
                .with_location(self.node_location(node, file_path))
                .with_code(self.node_text(node, source));
                Some(output.add_block_auto(block))
            }

            // Dereference expressions
            "dereference_expression" => {
                let uses = self.extract_used_identifiers(node, source, scope);
                let block = Block::new(
                    self.truncate(&self.node_text(node, source), 50),
                    BlockType::Expression,
                )
                .with_uses(uses)
                .with_location(self.node_location(node, file_path))
                .with_code(self.node_text(node, source));
                Some(output.add_block_auto(block))
            }

            // Struct expressions (instantiation)
            "struct_expression" => {
                let name = node
                    .child_by_field_name("name")
                    .map(|n| self.node_text(&n, source))
                    .unwrap_or_else(|| "struct".to_string());

                let uses = self.extract_used_identifiers(node, source, scope);

                let block = Block::new(format!("{} {{ }}", name), BlockType::New)
                    .with_uses(uses)
                    .with_location(self.node_location(node, file_path))
                    .with_code(self.node_text(node, source));

                Some(output.add_block_auto(block))
            }

            // Tuple expressions
            "tuple_expression" => {
                let uses = self.extract_used_identifiers(node, source, scope);
                let block = Block::new("tuple", BlockType::Array)
                    .with_uses(uses)
                    .with_location(self.node_location(node, file_path))
                    .with_code(self.node_text(node, source));
                Some(output.add_block_auto(block))
            }

            // Array expressions
            "array_expression" => {
                let uses = self.extract_used_identifiers(node, source, scope);
                let block = Block::new("array", BlockType::Array)
                    .with_uses(uses)
                    .with_location(self.node_location(node, file_path))
                    .with_code(self.node_text(node, source));
                Some(output.add_block_auto(block))
            }

            // Field expressions
            "field_expression" => {
                let uses = self.extract_used_identifiers(node, source, scope);
                let text = self.node_text(node, source);
                let block = Block::new(self.truncate(&text, 50), BlockType::Member)
                    .with_uses(uses)
                    .with_location(self.node_location(node, file_path))
                    .with_code(text);
                Some(output.add_block_auto(block))
            }

            // Index expressions
            "index_expression" => {
                let uses = self.extract_used_identifiers(node, source, scope);
                let text = self.node_text(node, source);
                let block = Block::new(self.truncate(&text, 50), BlockType::Index)
                    .with_uses(uses)
                    .with_location(self.node_location(node, file_path))
                    .with_code(text);
                Some(output.add_block_auto(block))
            }

            // Range expressions
            "range_expression" => {
                let uses = self.extract_used_identifiers(node, source, scope);
                let block = Block::new(
                    self.truncate(&self.node_text(node, source), 50),
                    BlockType::Expression,
                )
                .with_uses(uses)
                .with_location(self.node_location(node, file_path))
                .with_code(self.node_text(node, source));
                Some(output.add_block_auto(block))
            }

            // Attributes (decorators)
            "attribute_item" | "inner_attribute_item" => {
                let attr_text = self.node_text(node, source);
                let name = self.extract_attribute_name(node, source);

                let block = Block::new(format!("#[{}]", name), BlockType::Decorator)
                    .with_location(self.node_location(node, file_path))
                    .with_code(attr_text)
                    .with_metadata(BlockMetadata {
                        target: Some(name),
                        ..Default::default()
                    });

                Some(output.add_block_auto(block))
            }

            // Skip these node types (comments don't contribute to flow)
            "line_comment"
            | "block_comment"
            | "{"
            | "}"
            | "("
            | ")"
            | "["
            | "]"
            | ","
            | ";"
            | ":"
            | "::"
            | "->"
            | "=>"
            | "." => None,
            "identifier"
            | "field_identifier"
            | "type_identifier"
            | "primitive_type"
            | "string_literal"
            | "raw_string_literal"
            | "char_literal"
            | "integer_literal"
            | "float_literal"
            | "boolean_literal" => None,
            "visibility_modifier" | "lifetime" | "generic_type" | "type_parameters"
            | "type_arguments" | "where_clause" => None,

            // Default: recursively extract children
            _ => {
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
        let name = node
            .child_by_field_name("name")
            .map(|n| self.node_text(&n, source))
            .unwrap_or_else(|| "fn".to_string());

        let is_async = self.has_modifier(node, "async");
        let is_unsafe = self.has_modifier(node, "unsafe");
        let is_const = self.has_modifier(node, "const");

        let block_type = if is_async {
            BlockType::AsyncFunction
        } else {
            BlockType::Function
        };

        let mut scope = Scope::new();
        let params = self.extract_parameters(node, source, &mut scope);
        let mut children = Vec::new();

        // Extract attributes
        let attributes = self.extract_attributes(node, source, output);
        children.extend(attributes);

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

        let visibility = self.extract_visibility(node, source);

        let block = Block::new(name, block_type)
            .with_uses(scope.used_external.into_iter().collect())
            .with_produces(params.clone())
            .with_children(children)
            .with_location(self.node_location(node, file_path))
            .with_metadata(BlockMetadata {
                is_async: Some(is_async),
                parameters: Some(params),
                return_type,
                visibility: if visibility.is_empty() {
                    None
                } else {
                    Some(visibility)
                },
                decorators: if is_unsafe || is_const {
                    Some(
                        [
                            if is_unsafe {
                                Some("unsafe".to_string())
                            } else {
                                None
                            },
                            if is_const {
                                Some("const".to_string())
                            } else {
                                None
                            },
                        ]
                        .into_iter()
                        .flatten()
                        .collect(),
                    )
                } else {
                    None
                },
                ..Default::default()
            });

        let id = output.add_block_auto(block);
        if is_top_level {
            output.add_root(id.clone());
        }
        Some(id)
    }

    fn extract_impl(
        &self,
        node: &Node,
        source: &str,
        file_path: &str,
        output: &mut FlowMapOutput,
        is_top_level: bool,
    ) -> Option<BlockId> {
        let type_name = node
            .child_by_field_name("type")
            .map(|n| self.node_text(&n, source))
            .unwrap_or_else(|| "impl".to_string());

        let trait_name = node
            .child_by_field_name("trait")
            .map(|n| self.node_text(&n, source));

        let name = if let Some(trait_ref) = &trait_name {
            format!("impl {} for {}", trait_ref, type_name)
        } else {
            format!("impl {}", type_name)
        };

        let mut children = Vec::new();
        let mut scope = Scope::new();

        // Extract impl body
        if let Some(body) = node.child_by_field_name("body") {
            let mut cursor = body.walk();
            for child in body.children(&mut cursor) {
                if let Some(member_id) =
                    self.extract_impl_member(&child, source, file_path, output, &mut scope)
                {
                    children.push(member_id);
                }
            }
        }

        let block = Block::new(name, BlockType::Class)
            .with_children(children)
            .with_location(self.node_location(node, file_path))
            .with_metadata(BlockMetadata {
                target: trait_name,
                ..Default::default()
            });

        let id = output.add_block_auto(block);
        if is_top_level {
            output.add_root(id.clone());
        }
        Some(id)
    }

    fn extract_impl_member(
        &self,
        node: &Node,
        source: &str,
        file_path: &str,
        output: &mut FlowMapOutput,
        scope: &mut Scope,
    ) -> Option<BlockId> {
        match node.kind() {
            "function_item" => {
                let name = node
                    .child_by_field_name("name")
                    .map(|n| self.node_text(&n, source))
                    .unwrap_or_else(|| "method".to_string());

                let is_async = self.has_modifier(node, "async");
                let params = self.extract_parameters(node, source, scope);

                // Check if it's a static method (no self parameter)
                let is_static = !params.iter().any(|p| p == "self" || p.contains("self"));

                let block_type = if is_static {
                    BlockType::StaticMethod
                } else if is_async {
                    BlockType::AsyncMethod
                } else {
                    BlockType::Method
                };

                let mut children = Vec::new();

                // Extract attributes
                let attributes = self.extract_attributes(node, source, output);
                children.extend(attributes);

                // Extract body
                if let Some(body) = node.child_by_field_name("body") {
                    if let Some(body_id) =
                        self.extract_node(&body, source, file_path, output, scope, false)
                    {
                        children.push(body_id);
                    }
                }

                let return_type = node
                    .child_by_field_name("return_type")
                    .map(|n| self.node_text(&n, source));

                let visibility = self.extract_visibility(node, source);

                let block = Block::new(name, block_type)
                    .with_produces(params.clone())
                    .with_children(children)
                    .with_location(self.node_location(node, file_path))
                    .with_metadata(BlockMetadata {
                        is_static: Some(is_static),
                        is_async: Some(is_async),
                        parameters: Some(params),
                        return_type,
                        visibility: if visibility.is_empty() {
                            None
                        } else {
                            Some(visibility)
                        },
                        ..Default::default()
                    });

                Some(output.add_block_auto(block))
            }

            "const_item" => {
                let name = node
                    .child_by_field_name("name")
                    .map(|n| self.node_text(&n, source))
                    .unwrap_or_else(|| "const".to_string());

                let mut uses = Vec::new();
                if let Some(value) = node.child_by_field_name("value") {
                    uses = self.extract_used_identifiers(&value, source, scope);
                }

                let block = Block::new(name.clone(), BlockType::StaticProperty)
                    .with_uses(uses)
                    .with_produces(vec![name])
                    .with_location(self.node_location(node, file_path))
                    .with_code(self.node_text(node, source));

                Some(output.add_block_auto(block))
            }

            "type_item" => {
                let name = node
                    .child_by_field_name("name")
                    .map(|n| self.node_text(&n, source))
                    .unwrap_or_else(|| "type".to_string());

                let block = Block::new(name, BlockType::TypeAlias)
                    .with_location(self.node_location(node, file_path))
                    .with_code(self.node_text(node, source));

                Some(output.add_block_auto(block))
            }

            "attribute_item" => {
                let name = self.extract_attribute_name(node, source);
                let block = Block::new(format!("#[{}]", name), BlockType::Decorator)
                    .with_location(self.node_location(node, file_path))
                    .with_code(self.node_text(node, source));
                Some(output.add_block_auto(block))
            }

            _ => None,
        }
    }

    fn extract_trait(
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
            .unwrap_or_else(|| "trait".to_string());

        let mut children = Vec::new();
        let mut scope = Scope::new();

        // Extract attributes
        let attributes = self.extract_attributes(node, source, output);
        children.extend(attributes);

        // Extract trait body
        if let Some(body) = node.child_by_field_name("body") {
            let mut cursor = body.walk();
            for child in body.children(&mut cursor) {
                if let Some(member_id) =
                    self.extract_trait_member(&child, source, file_path, output, &mut scope)
                {
                    children.push(member_id);
                }
            }
        }

        let visibility = self.extract_visibility(node, source);

        let block = Block::new(name, BlockType::Interface)
            .with_children(children)
            .with_location(self.node_location(node, file_path))
            .with_metadata(BlockMetadata {
                visibility: if visibility.is_empty() {
                    None
                } else {
                    Some(visibility)
                },
                ..Default::default()
            });

        let id = output.add_block_auto(block);
        if is_top_level {
            output.add_root(id.clone());
        }
        Some(id)
    }

    fn extract_trait_member(
        &self,
        node: &Node,
        source: &str,
        file_path: &str,
        output: &mut FlowMapOutput,
        scope: &mut Scope,
    ) -> Option<BlockId> {
        match node.kind() {
            "function_signature_item" => {
                let name = node
                    .child_by_field_name("name")
                    .map(|n| self.node_text(&n, source))
                    .unwrap_or_else(|| "method".to_string());

                let params = self.extract_parameters(node, source, scope);
                let return_type = node
                    .child_by_field_name("return_type")
                    .map(|n| self.node_text(&n, source));

                let block = Block::new(name, BlockType::Method)
                    .with_produces(params.clone())
                    .with_location(self.node_location(node, file_path))
                    .with_metadata(BlockMetadata {
                        parameters: Some(params),
                        return_type,
                        ..Default::default()
                    });

                Some(output.add_block_auto(block))
            }

            "function_item" => {
                self.extract_impl_member(node, source, file_path, output, scope)
            }

            "type_item" | "associated_type" => {
                let name = node
                    .child_by_field_name("name")
                    .map(|n| self.node_text(&n, source))
                    .unwrap_or_else(|| "type".to_string());

                let block = Block::new(name, BlockType::TypeAlias)
                    .with_location(self.node_location(node, file_path))
                    .with_code(self.node_text(node, source));

                Some(output.add_block_auto(block))
            }

            "const_item" => {
                let name = node
                    .child_by_field_name("name")
                    .map(|n| self.node_text(&n, source))
                    .unwrap_or_else(|| "const".to_string());

                let block = Block::new(name, BlockType::Const)
                    .with_location(self.node_location(node, file_path))
                    .with_code(self.node_text(node, source));

                Some(output.add_block_auto(block))
            }

            _ => None,
        }
    }

    fn extract_struct(
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
            .unwrap_or_else(|| "struct".to_string());

        let mut children = Vec::new();

        // Extract attributes
        let attributes = self.extract_attributes(node, source, output);
        children.extend(attributes);

        // Extract fields
        if let Some(body) = node.child_by_field_name("body") {
            let mut cursor = body.walk();
            for child in body.children(&mut cursor) {
                if child.kind() == "field_declaration" {
                    let field_name = child
                        .child_by_field_name("name")
                        .map(|n| self.node_text(&n, source))
                        .unwrap_or_else(|| "field".to_string());

                    let field_type = child
                        .child_by_field_name("type")
                        .map(|n| self.node_text(&n, source));

                    let visibility = self.extract_visibility(&child, source);

                    let field_block = Block::new(field_name.clone(), BlockType::Property)
                        .with_produces(vec![field_name])
                        .with_location(self.node_location(&child, file_path))
                        .with_metadata(BlockMetadata {
                            return_type: field_type,
                            visibility: if visibility.is_empty() {
                                None
                            } else {
                                Some(visibility)
                            },
                            ..Default::default()
                        });

                    children.push(output.add_block_auto(field_block));
                }
            }
        }

        let visibility = self.extract_visibility(node, source);

        let block = Block::new(name, BlockType::Class)
            .with_children(children)
            .with_location(self.node_location(node, file_path))
            .with_code(self.node_text(node, source))
            .with_metadata(BlockMetadata {
                visibility: if visibility.is_empty() {
                    None
                } else {
                    Some(visibility)
                },
                ..Default::default()
            });

        let id = output.add_block_auto(block);
        if is_top_level {
            output.add_root(id.clone());
        }
        Some(id)
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

        // Extract attributes
        let attributes = self.extract_attributes(node, source, output);
        children.extend(attributes);

        // Extract variants
        if let Some(body) = node.child_by_field_name("body") {
            let mut cursor = body.walk();
            for child in body.children(&mut cursor) {
                if child.kind() == "enum_variant" {
                    let variant_name = child
                        .child_by_field_name("name")
                        .map(|n| self.node_text(&n, source))
                        .unwrap_or_else(|| "variant".to_string());

                    let variant_block = Block::new(variant_name, BlockType::EnumMember)
                        .with_location(self.node_location(&child, file_path))
                        .with_code(self.node_text(&child, source));

                    children.push(output.add_block_auto(variant_block));
                }
            }
        }

        let visibility = self.extract_visibility(node, source);

        let block = Block::new(name, BlockType::Enum)
            .with_children(children)
            .with_location(self.node_location(node, file_path))
            .with_metadata(BlockMetadata {
                visibility: if visibility.is_empty() {
                    None
                } else {
                    Some(visibility)
                },
                ..Default::default()
            });

        let id = output.add_block_auto(block);
        if is_top_level {
            output.add_root(id.clone());
        }
        Some(id)
    }

    fn extract_let_declaration(
        &self,
        node: &Node,
        source: &str,
        file_path: &str,
        output: &mut FlowMapOutput,
        scope: &mut Scope,
    ) -> Option<BlockId> {
        let pattern = node.child_by_field_name("pattern");
        let names = pattern
            .map(|p| self.extract_pattern(&p, source, scope))
            .unwrap_or_default();

        for name in &names {
            scope.define(name);
        }

        let mut uses = Vec::new();
        let mut children = Vec::new();

        if let Some(value) = node.child_by_field_name("value") {
            uses = self.extract_used_identifiers(&value, source, scope);

            if self.is_complex_expression(&value) {
                if let Some(value_id) =
                    self.extract_node(&value, source, file_path, output, scope, false)
                {
                    children.push(value_id);
                }
            }
        }

        let is_mutable = self.has_child_kind(node, "mutable_specifier");
        let block_type = if is_mutable {
            BlockType::Let
        } else {
            BlockType::Const
        };

        let name_str = names.join(", ");

        let block = Block::new(name_str, block_type)
            .with_uses(uses)
            .with_produces(names)
            .with_children(children)
            .with_location(self.node_location(node, file_path))
            .with_code(self.node_text(node, source));

        Some(output.add_block_auto(block))
    }

    fn extract_if_expression(
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

        let uses = condition
            .map(|c| self.extract_used_identifiers(&c, source, scope))
            .unwrap_or_default();
        let mut children = Vec::new();

        // Consequence (then branch)
        if let Some(consequence) = node.child_by_field_name("consequence") {
            if let Some(then_id) =
                self.extract_node(&consequence, source, file_path, output, scope, false)
            {
                children.push(then_id);
            }
        }

        // Alternative (else branch)
        if let Some(alternative) = node.child_by_field_name("alternative") {
            // Could be else_clause or another if_expression (else if)
            match alternative.kind() {
                "else_clause" => {
                    if let Some(body) = alternative.child(1) {
                        if let Some(else_id) =
                            self.extract_node(&body, source, file_path, output, scope, false)
                        {
                            let else_block = Block::new("else", BlockType::Else)
                                .with_children(vec![else_id])
                                .with_location(self.node_location(&alternative, file_path));
                            children.push(output.add_block_auto(else_block));
                        }
                    }
                }
                "if_expression" => {
                    if let Some(else_if_id) =
                        self.extract_if_expression(&alternative, source, file_path, output, scope)
                    {
                        children.push(else_if_id);
                    }
                }
                _ => {
                    if let Some(alt_id) =
                        self.extract_node(&alternative, source, file_path, output, scope, false)
                    {
                        let else_block = Block::new("else", BlockType::Else)
                            .with_children(vec![alt_id])
                            .with_location(self.node_location(&alternative, file_path));
                        children.push(output.add_block_auto(else_block));
                    }
                }
            }
        }

        let block =
            Block::new(format!("if {}", self.truncate(&condition_text, 30)), BlockType::If)
                .with_uses(uses)
                .with_children(children)
                .with_location(self.node_location(node, file_path))
                .with_metadata(BlockMetadata {
                    condition: Some(condition_text),
                    ..Default::default()
                });

        Some(output.add_block_auto(block))
    }

    fn extract_match_expression(
        &self,
        node: &Node,
        source: &str,
        file_path: &str,
        output: &mut FlowMapOutput,
        scope: &mut Scope,
    ) -> Option<BlockId> {
        let value = node.child_by_field_name("value");
        let value_text = value.map(|v| self.node_text(&v, source)).unwrap_or_default();
        let uses = value
            .map(|v| self.extract_used_identifiers(&v, source, scope))
            .unwrap_or_default();

        let mut children = Vec::new();

        if let Some(body) = node.child_by_field_name("body") {
            let mut cursor = body.walk();
            for child in body.children(&mut cursor) {
                if child.kind() == "match_arm" {
                    let pattern = child
                        .child_by_field_name("pattern")
                        .map(|p| self.node_text(&p, source))
                        .unwrap_or_else(|| "_".to_string());

                    let mut arm_children = Vec::new();
                    if let Some(value_node) = child.child_by_field_name("value") {
                        if let Some(value_id) =
                            self.extract_node(&value_node, source, file_path, output, scope, false)
                        {
                            arm_children.push(value_id);
                        }
                    }

                    let arm_block = Block::new(pattern, BlockType::Case)
                        .with_children(arm_children)
                        .with_location(self.node_location(&child, file_path));

                    children.push(output.add_block_auto(arm_block));
                }
            }
        }

        let block = Block::new(
            format!("match {}", self.truncate(&value_text, 30)),
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
        let mut children = Vec::new();

        // Extract condition (for while loops)
        let condition = node.child_by_field_name("condition");
        if let Some(cond) = condition {
            uses.extend(self.extract_used_identifiers(&cond, source, scope));
        }

        // Extract body
        if let Some(body) = node.child_by_field_name("body") {
            if let Some(body_id) =
                self.extract_node(&body, source, file_path, output, scope, false)
            {
                children.push(body_id);
            }
        }

        let condition_text = condition.map(|c| self.node_text(&c, source));
        let name = match loop_type {
            BlockType::While => {
                format!("while {}", condition_text.clone().unwrap_or_default())
            }
            _ => "loop".to_string(),
        };

        let block = Block::new(name, loop_type)
            .with_uses(uses)
            .with_children(children)
            .with_location(self.node_location(node, file_path))
            .with_metadata(BlockMetadata {
                condition: condition_text,
                ..Default::default()
            });

        Some(output.add_block_auto(block))
    }

    fn extract_for_loop(
        &self,
        node: &Node,
        source: &str,
        file_path: &str,
        output: &mut FlowMapOutput,
        scope: &mut Scope,
    ) -> Option<BlockId> {
        let mut uses = Vec::new();
        let mut produces = Vec::new();
        let mut children = Vec::new();

        // Extract pattern (loop variable)
        if let Some(pattern) = node.child_by_field_name("pattern") {
            let vars = self.extract_pattern(&pattern, source, scope);
            produces.extend(vars.clone());
            for v in vars {
                scope.define(&v);
            }
        }

        // Extract value (iterator)
        if let Some(value) = node.child_by_field_name("value") {
            uses.extend(self.extract_used_identifiers(&value, source, scope));
        }

        // Extract body
        if let Some(body) = node.child_by_field_name("body") {
            if let Some(body_id) =
                self.extract_node(&body, source, file_path, output, scope, false)
            {
                children.push(body_id);
            }
        }

        let value_text = node
            .child_by_field_name("value")
            .map(|v| self.node_text(&v, source));

        let block = Block::new("for...in", BlockType::ForIn)
            .with_uses(uses)
            .with_produces(produces)
            .with_children(children)
            .with_location(self.node_location(node, file_path))
            .with_metadata(BlockMetadata {
                condition: value_text,
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
        let function = node.child_by_field_name("function")?;

        let mut uses = Vec::new();
        let mut children = Vec::new();

        // If the function is a field_expression (method call chain), extract the receiver chain first
        if function.kind() == "field_expression" {
            // The value of the field_expression is the receiver - could be another call
            if let Some(receiver) = function.child_by_field_name("value") {
                if receiver.kind() == "call_expression" {
                    if let Some(recv_id) =
                        self.extract_call_expression(&receiver, source, file_path, output, scope)
                    {
                        children.push(recv_id);
                    }
                }
            }
        }

        // Track function identifiers as used
        uses.extend(self.extract_used_identifiers(&function, source, scope));

        // Extract arguments - including closures
        if let Some(args) = node.child_by_field_name("arguments") {
            let mut cursor = args.walk();
            for arg in args.children(&mut cursor) {
                if arg.kind() == "(" || arg.kind() == ")" || arg.kind() == "," {
                    continue;
                }
                uses.extend(self.extract_used_identifiers(&arg, source, scope));

                // Extract closures and other complex expressions
                if self.is_complex_expression(&arg) || arg.kind() == "closure_expression" {
                    if let Some(arg_id) =
                        self.extract_node(&arg, source, file_path, output, scope, false)
                    {
                        children.push(arg_id);
                    }
                }
            }
        }

        // Get the method/function name (just the last part, not the full chain)
        let function_name = if function.kind() == "field_expression" {
            function
                .child_by_field_name("field")
                .map(|f| self.node_text(&f, source))
                .unwrap_or_else(|| self.node_text(&function, source))
        } else {
            self.node_text(&function, source)
        };

        let block = Block::new(function_name, BlockType::Call)
            .with_uses(uses)
            .with_children(children)
            .with_location(self.node_location(node, file_path))
            .with_code(self.node_text(node, source));

        Some(output.add_block_auto(block))
    }

    fn extract_method_call(
        &self,
        node: &Node,
        source: &str,
        file_path: &str,
        output: &mut FlowMapOutput,
        scope: &mut Scope,
    ) -> Option<BlockId> {
        let receiver = node
            .child_by_field_name("value")
            .map(|n| self.node_text(&n, source))
            .unwrap_or_default();
        let method = node
            .child_by_field_name("name")
            .map(|n| self.node_text(&n, source))
            .unwrap_or_else(|| "method".to_string());

        let mut uses = Vec::new();
        let mut children = Vec::new();

        // Track receiver as used
        if let Some(recv) = node.child_by_field_name("value") {
            uses.extend(self.extract_used_identifiers(&recv, source, scope));
        }

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

        let block = Block::new(format!("{}.{}", receiver, method), BlockType::MethodCall)
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
        let mut children = Vec::new();

        // The awaited expression is the first child
        if let Some(inner) = node.child(0) {
            if let Some(inner_id) =
                self.extract_node(&inner, source, file_path, output, scope, false)
            {
                children.push(inner_id);
            }
        }

        let uses = self.extract_used_identifiers(node, source, scope);
        let label = self.truncate(&self.node_text(node, source), 40);

        let block = Block::new(format!("{}.await", label), BlockType::AwaitCall)
            .with_uses(uses)
            .with_children(children)
            .with_location(self.node_location(node, file_path));

        Some(output.add_block_auto(block))
    }

    fn extract_return_expression(
        &self,
        node: &Node,
        source: &str,
        file_path: &str,
        output: &mut FlowMapOutput,
        scope: &mut Scope,
    ) -> Option<BlockId> {
        let mut uses = Vec::new();
        let mut children = Vec::new();

        // Find return value
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() != "return" {
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

    fn extract_closure(
        &self,
        node: &Node,
        source: &str,
        file_path: &str,
        output: &mut FlowMapOutput,
        scope: &mut Scope,
    ) -> Option<BlockId> {
        let mut closure_scope = Scope::new();
        let params = self.extract_closure_parameters(node, source, &mut closure_scope);
        let mut children = Vec::new();

        // Extract body
        if let Some(body) = node.child_by_field_name("body") {
            if let Some(body_id) =
                self.extract_node(&body, source, file_path, output, &mut closure_scope, false)
            {
                children.push(body_id);
            }
        }

        let is_async = self.has_modifier(node, "async");
        let is_move = self.has_modifier(node, "move");

        let block = Block::new("closure", BlockType::Arrow)
            .with_uses(closure_scope.used_external.into_iter().collect())
            .with_produces(params.clone())
            .with_children(children)
            .with_location(self.node_location(node, file_path))
            .with_code(self.node_text(node, source))
            .with_metadata(BlockMetadata {
                is_async: Some(is_async),
                parameters: Some(params),
                decorators: if is_move {
                    Some(vec!["move".to_string()])
                } else {
                    None
                },
                ..Default::default()
            });

        Some(output.add_block_auto(block))
    }

    fn extract_macro_invocation(
        &self,
        node: &Node,
        source: &str,
        file_path: &str,
        output: &mut FlowMapOutput,
        scope: &mut Scope,
    ) -> Option<BlockId> {
        let macro_name = node
            .child_by_field_name("macro")
            .map(|n| self.node_text(&n, source))
            .unwrap_or_else(|| "macro".to_string());

        let uses = self.extract_used_identifiers(node, source, scope);

        let block = Block::new(format!("{}!", macro_name), BlockType::Call)
            .with_uses(uses)
            .with_location(self.node_location(node, file_path))
            .with_code(self.node_text(node, source))
            .with_metadata(BlockMetadata {
                target: Some(macro_name),
                ..Default::default()
            });

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

        let produces = self.extract_pattern(&left, source, scope);
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

    // Helper methods

    fn extract_parameters(&self, node: &Node, source: &str, scope: &mut Scope) -> Vec<String> {
        let mut params = Vec::new();

        if let Some(params_node) = node.child_by_field_name("parameters") {
            let mut cursor = params_node.walk();
            for child in params_node.children(&mut cursor) {
                match child.kind() {
                    "parameter" => {
                        if let Some(pattern) = child.child_by_field_name("pattern") {
                            let names = self.extract_pattern(&pattern, source, scope);
                            for name in names {
                                scope.define(&name);
                                params.push(name);
                            }
                        }
                    }
                    "self_parameter" => {
                        params.push("self".to_string());
                        scope.define("self");
                    }
                    _ => {}
                }
            }
        }

        params
    }

    fn extract_closure_parameters(
        &self,
        node: &Node,
        source: &str,
        scope: &mut Scope,
    ) -> Vec<String> {
        let mut params = Vec::new();

        if let Some(params_node) = node.child_by_field_name("parameters") {
            let mut cursor = params_node.walk();
            for child in params_node.children(&mut cursor) {
                if child.kind() == "closure_parameter" || child.kind() == "parameter" {
                    if let Some(pattern) = child.child_by_field_name("pattern") {
                        let names = self.extract_pattern(&pattern, source, scope);
                        for name in names {
                            scope.define(&name);
                            params.push(name);
                        }
                    } else {
                        // Simple identifier parameter
                        let name = self.node_text(&child, source);
                        if !name.is_empty()
                            && name != "|"
                            && name != ","
                            && !name.starts_with(':')
                        {
                            scope.define(&name);
                            params.push(name);
                        }
                    }
                }
            }
        }

        params
    }

    fn extract_pattern(&self, node: &Node, source: &str, scope: &mut Scope) -> Vec<String> {
        let mut names = Vec::new();

        match node.kind() {
            "identifier" => {
                let name = self.node_text(node, source);
                if name != "_" {
                    names.push(name);
                }
            }
            "tuple_pattern" | "slice_pattern" => {
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    names.extend(self.extract_pattern(&child, source, scope));
                }
            }
            "struct_pattern" => {
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    if child.kind() == "field_pattern" {
                        if let Some(name_node) = child.child_by_field_name("name") {
                            names.push(self.node_text(&name_node, source));
                        }
                    }
                }
            }
            "ref_pattern" | "mut_pattern" => {
                if let Some(inner) = node.child(1) {
                    names.extend(self.extract_pattern(&inner, source, scope));
                }
            }
            "or_pattern" => {
                // Just extract from first alternative for variable names
                if let Some(first) = node.child(0) {
                    names.extend(self.extract_pattern(&first, source, scope));
                }
            }
            _ => {}
        }

        names
    }

    fn extract_used_identifiers(&self, node: &Node, source: &str, scope: &mut Scope) -> Vec<String> {
        let mut identifiers = Vec::new();

        match node.kind() {
            "identifier" => {
                let name = self.node_text(node, source);
                if !self.is_builtin(&name) && !scope.is_defined(&name) && name != "_" {
                    identifiers.push(name);
                }
            }
            "field_expression" => {
                // Only extract the root object
                if let Some(value) = node.child_by_field_name("value") {
                    identifiers.extend(self.extract_used_identifiers(&value, source, scope));
                }
            }
            "self" => {
                identifiers.push("self".to_string());
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

    fn extract_use_names(&self, node: &Node, source: &str) -> Vec<String> {
        let mut names = Vec::new();

        fn collect_use_names(node: &Node, source: &str, names: &mut Vec<String>, extractor: &RustBlockExtractor) {
            match node.kind() {
                "identifier" => {
                    names.push(extractor.node_text(node, source));
                }
                "use_as_clause" => {
                    if let Some(alias) = node.child_by_field_name("alias") {
                        names.push(extractor.node_text(&alias, source));
                    } else if let Some(path) = node.child_by_field_name("path") {
                        collect_use_names(&path, source, names, extractor);
                    }
                }
                "use_list" | "use_group" => {
                    let mut cursor = node.walk();
                    for child in node.children(&mut cursor) {
                        collect_use_names(&child, source, names, extractor);
                    }
                }
                "scoped_use_list" => {
                    if let Some(list) = node.child_by_field_name("list") {
                        collect_use_names(&list, source, names, extractor);
                    }
                }
                "use_wildcard" => {
                    names.push("*".to_string());
                }
                "scoped_identifier" => {
                    if let Some(name) = node.child_by_field_name("name") {
                        names.push(extractor.node_text(&name, source));
                    }
                }
                _ => {
                    let mut cursor = node.walk();
                    for child in node.children(&mut cursor) {
                        collect_use_names(&child, source, names, extractor);
                    }
                }
            }
        }

        if let Some(argument) = node.child_by_field_name("argument") {
            collect_use_names(&argument, source, &mut names, self);
        }

        names
    }

    fn get_use_path(&self, node: &Node, source: &str) -> Option<String> {
        node.child_by_field_name("argument")
            .map(|a| self.node_text(&a, source))
    }

    fn extract_attributes(&self, node: &Node, source: &str, output: &mut FlowMapOutput) -> Vec<BlockId> {
        let mut attributes = Vec::new();

        // Look for attribute_item siblings before this node
        if let Some(parent) = node.parent() {
            let mut cursor = parent.walk();
            let mut found_self = false;

            for child in parent.children(&mut cursor) {
                if child.id() == node.id() {
                    found_self = true;
                    break;
                }

                if child.kind() == "attribute_item" && !found_self {
                    let name = self.extract_attribute_name(&child, source);
                    let block = Block::new(format!("#[{}]", name), BlockType::Decorator)
                        .with_code(self.node_text(&child, source));
                    attributes.push(output.add_block_auto(block));
                }
            }
        }

        attributes
    }

    fn extract_attribute_name(&self, node: &Node, source: &str) -> String {
        // Find the attribute inside the brackets
        fn find_attr_name(node: &Node, source: &str, extractor: &RustBlockExtractor) -> String {
            match node.kind() {
                "attribute" | "meta_item" => {
                    if let Some(path) = node.child_by_field_name("path") {
                        return extractor.node_text(&path, source);
                    }
                    // Check for identifier child
                    let mut cursor = node.walk();
                    for child in node.children(&mut cursor) {
                        if child.kind() == "identifier" || child.kind() == "scoped_identifier" {
                            return extractor.node_text(&child, source);
                        }
                    }
                }
                "identifier" | "scoped_identifier" => {
                    return extractor.node_text(node, source);
                }
                _ => {
                    let mut cursor = node.walk();
                    for child in node.children(&mut cursor) {
                        let result = find_attr_name(&child, source, extractor);
                        if !result.is_empty() {
                            return result;
                        }
                    }
                }
            }
            String::new()
        }

        let result = find_attr_name(node, source, self);
        if result.is_empty() {
            "attr".to_string()
        } else {
            result
        }
    }

    fn extract_visibility(&self, node: &Node, source: &str) -> String {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "visibility_modifier" {
                return self.node_text(&child, source);
            }
        }
        String::new()
    }

    fn has_child_kind(&self, node: &Node, kind: &str) -> bool {
        let mut cursor = node.walk();
        node.children(&mut cursor).any(|c| c.kind() == kind)
    }

    /// Check for a modifier (async, unsafe, const) in function_modifiers or visibility_modifier
    fn has_modifier(&self, node: &Node, modifier: &str) -> bool {
        // First check direct children
        if self.has_child_kind(node, modifier) {
            return true;
        }
        // Check inside function_modifiers
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "function_modifiers" {
                if self.has_child_kind(&child, modifier) {
                    return true;
                }
            }
        }
        false
    }

    fn is_complex_expression(&self, node: &Node) -> bool {
        matches!(
            node.kind(),
            "call_expression"
                | "method_call_expression"
                | "await_expression"
                | "closure_expression"
                | "struct_expression"
                | "match_expression"
                | "if_expression"
                | "block"
                | "async_block"
                | "unsafe_block"
        )
    }

    fn is_significant_node(&self, kind: &str) -> bool {
        matches!(
            kind,
            "function_item"
                | "impl_item"
                | "trait_item"
                | "struct_item"
                | "enum_item"
                | "mod_item"
                | "use_declaration"
        )
    }

    fn is_builtin(&self, name: &str) -> bool {
        matches!(
            name,
            "Some"
                | "None"
                | "Ok"
                | "Err"
                | "true"
                | "false"
                | "Self"
                | "super"
                | "crate"
                | "std"
                | "core"
                | "alloc"
                | "Vec"
                | "String"
                | "Box"
                | "Rc"
                | "Arc"
                | "Cell"
                | "RefCell"
                | "Option"
                | "Result"
                | "HashMap"
                | "HashSet"
                | "BTreeMap"
                | "BTreeSet"
                | "println"
                | "print"
                | "eprintln"
                | "eprint"
                | "format"
                | "vec"
                | "panic"
                | "assert"
                | "assert_eq"
                | "assert_ne"
                | "debug_assert"
                | "todo"
                | "unimplemented"
                | "unreachable"
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

impl Default for RustBlockExtractor {
    fn default() -> Self {
        Self::new().expect("Failed to create RustBlockExtractor")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_function() {
        let mut extractor = RustBlockExtractor::new().unwrap();
        let source = r#"
fn add(a: i32, b: i32) -> i32 {
    a + b
}
"#;

        let output = extractor.parse_file(source, "test.rs").unwrap();
        assert!(output.block_count() > 0);
        assert!(!output.root.is_empty());

        let json = serde_json::to_string_pretty(&output).unwrap();
        assert!(json.contains("add"));
        assert!(json.contains("function"));
    }

    #[test]
    fn test_struct_and_impl() {
        let mut extractor = RustBlockExtractor::new().unwrap();
        let source = r#"
pub struct User {
    pub name: String,
    age: u32,
}

impl User {
    pub fn new(name: String, age: u32) -> Self {
        User { name, age }
    }

    pub fn greet(&self) {
        println!("Hello, {}!", self.name);
    }
}
"#;

        let output = extractor.parse_file(source, "test.rs").unwrap();
        let json = serde_json::to_string_pretty(&output).unwrap();

        assert!(json.contains("User"));
        assert!(json.contains("new"));
        assert!(json.contains("greet"));
        assert!(json.contains("name"));
    }

    #[test]
    fn test_enum_extraction() {
        let mut extractor = RustBlockExtractor::new().unwrap();
        let source = r#"
#[derive(Debug)]
pub enum Status {
    Active,
    Inactive,
    Pending(String),
}
"#;

        let output = extractor.parse_file(source, "test.rs").unwrap();
        let json = serde_json::to_string_pretty(&output).unwrap();

        assert!(json.contains("Status"));
        assert!(json.contains("Active"));
        assert!(json.contains("Inactive"));
        assert!(json.contains("Pending"));
        assert!(json.contains("enum"));
    }

    #[test]
    fn test_trait_extraction() {
        let mut extractor = RustBlockExtractor::new().unwrap();
        let source = r#"
pub trait Display {
    fn fmt(&self) -> String;

    fn default_method(&self) -> String {
        "default".to_string()
    }
}
"#;

        let output = extractor.parse_file(source, "test.rs").unwrap();
        let json = serde_json::to_string_pretty(&output).unwrap();

        assert!(json.contains("Display"));
        assert!(json.contains("fmt"));
        assert!(json.contains("default_method"));
    }

    #[test]
    fn test_async_function() {
        let mut extractor = RustBlockExtractor::new().unwrap();
        let source = r#"
async fn fetch_data(url: &str) -> Result<String, Error> {
    let response = client.get(url).await?;
    let body = response.text().await?;
    Ok(body)
}
"#;

        let output = extractor.parse_file(source, "test.rs").unwrap();
        let json = serde_json::to_string_pretty(&output).unwrap();

        assert!(json.contains("fetch_data"));
        assert!(json.contains("async_function"));
        assert!(json.contains("await"));
    }

    #[test]
    fn test_match_expression() {
        let mut extractor = RustBlockExtractor::new().unwrap();
        let source = r#"
fn process(status: Status) -> &'static str {
    match status {
        Status::Active => "active",
        Status::Inactive => "inactive",
        Status::Pending(msg) => "pending",
    }
}
"#;

        let output = extractor.parse_file(source, "test.rs").unwrap();
        let json = serde_json::to_string_pretty(&output).unwrap();

        assert!(json.contains("match"));
        assert!(json.contains("switch"));
        assert!(json.contains("case"));
    }

    #[test]
    fn test_data_flow() {
        let mut extractor = RustBlockExtractor::new().unwrap();
        let source = r#"
fn create_order(user_id: u64, items: Vec<Item>) -> Order {
    let user = get_user(user_id);
    let total = calculate_price(&user, &items);
    if total > 1000.0 {
        apply_discount(&mut total, &user);
    }
    Order { user, total, items }
}
"#;

        let output = extractor.parse_file(source, "test.rs").unwrap();

        // Check that we're tracking data flow
        let mut found_uses = false;
        let mut found_produces = false;

        for block in output.library.values() {
            if block.uses_data.contains(&"user_id".to_string()) {
                found_uses = true;
            }
            if block.produces_data.contains(&"user".to_string()) {
                found_produces = true;
            }
        }

        assert!(
            found_uses || found_produces,
            "Data flow should be tracked"
        );
    }

    #[test]
    fn test_closure_extraction() {
        let mut extractor = RustBlockExtractor::new().unwrap();
        let source = r#"
fn filter_users(users: Vec<User>) -> Vec<User> {
    users.into_iter()
        .filter(|u| u.age > 18)
        .map(|u| transform(u))
        .collect()
}
"#;

        let output = extractor.parse_file(source, "test.rs").unwrap();
        let json = serde_json::to_string_pretty(&output).unwrap();

        assert!(json.contains("closure") || json.contains("arrow"));
    }

    #[test]
    fn test_use_declarations() {
        let mut extractor = RustBlockExtractor::new().unwrap();
        let source = r#"
use std::collections::HashMap;
use crate::models::{User, Order};
use super::utils::*;
"#;

        let output = extractor.parse_file(source, "test.rs").unwrap();
        let json = serde_json::to_string_pretty(&output).unwrap();

        assert!(json.contains("import"));
        assert!(json.contains("HashMap") || json.contains("std::collections"));
    }
}
