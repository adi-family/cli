use lib_flowmap_core::{Block, BlockId, BlockMetadata, BlockType, FlowMapOutput, Location};
use std::collections::HashSet;
use tree_sitter::{Node, Parser};

/// Block-based extractor for Java
pub struct JavaBlockExtractor {
    parser: Parser,
}

struct Scope {
    defined: HashSet<String>,
}

struct Modifiers {
    visibility: Option<String>,
    is_static: bool,
}

impl Scope {
    fn new() -> Self {
        Self {
            defined: HashSet::new(),
        }
    }

    fn define(&mut self, name: &str) {
        self.defined.insert(name.to_string());
    }

    fn is_defined(&self, name: &str) -> bool {
        self.defined.contains(name)
    }
}

impl JavaBlockExtractor {
    pub fn new() -> crate::Result<Self> {
        let mut parser = Parser::new();
        let language = tree_sitter_java::LANGUAGE;
        parser
            .set_language(&language.into())
            .map_err(|e| crate::ParseError::TreeSitter(e.to_string()))?;

        Ok(Self { parser })
    }

    pub fn parse_file(&mut self, source: &str, file_path: &str) -> crate::Result<FlowMapOutput> {
        let tree =
            self.parser
                .parse(source, None)
                .ok_or_else(|| crate::ParseError::ParseFailed {
                    path: file_path.to_string(),
                })?;

        let mut output = FlowMapOutput::new()
            .with_file(file_path.to_string())
            .with_language("java");

        let root = tree.root_node();
        let mut scope = Scope::new();

        self.extract_node(&root, source, file_path, &mut output, &mut scope, true);

        Ok(output)
    }

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
            "program" => {
                let mut children = Vec::new();
                let mut cursor = node.walk();

                for child in node.children(&mut cursor) {
                    if let Some(child_id) =
                        self.extract_node(&child, source, file_path, output, scope, true)
                    {
                        children.push(child_id);
                    }
                }

                if children.is_empty() {
                    return None;
                }

                let block = Block::new("module", BlockType::Module)
                    .with_children(children.clone())
                    .with_location(self.node_location(node, file_path));

                let id = output.add_block_auto(block);
                output.add_root(id.clone());
                Some(id)
            }

            // Package declaration
            "package_declaration" => {
                let package_name = self.get_scoped_identifier(node, source);
                let block = Block::new(format!("package {}", package_name), BlockType::Statement)
                    .with_location(self.node_location(node, file_path))
                    .with_code(self.node_text(node, source));
                Some(output.add_block_auto(block))
            }

            "import_declaration" => {
                let import_path = self.get_scoped_identifier(node, source);
                let imported_name = import_path
                    .split('.')
                    .last()
                    .unwrap_or(&import_path)
                    .to_string();

                if imported_name != "*" {
                    scope.define(&imported_name);
                }

                let block = Block::new(format!("import {}", import_path), BlockType::Import)
                    .with_produces(if imported_name == "*" {
                        vec![]
                    } else {
                        vec![imported_name]
                    })
                    .with_location(self.node_location(node, file_path))
                    .with_code(self.node_text(node, source));
                Some(output.add_block_auto(block))
            }

            // Class declarations
            "class_declaration" => {
                self.extract_class(node, source, file_path, output, is_top_level)
            }

            // Interface declarations
            "interface_declaration" => {
                self.extract_interface(node, source, file_path, output, is_top_level)
            }

            "enum_declaration" => self.extract_enum(node, source, file_path, output, is_top_level),

            // Record declarations (Java 16+)
            "record_declaration" => {
                self.extract_record(node, source, file_path, output, is_top_level)
            }

            // Annotation type declarations
            "annotation_type_declaration" => {
                let name = node
                    .child_by_field_name("name")
                    .map(|n| self.node_text(&n, source))
                    .unwrap_or_else(|| "annotation".to_string());

                let block = Block::new(format!("@interface {}", name), BlockType::Interface)
                    .with_location(self.node_location(node, file_path))
                    .with_code(self.node_text(node, source));

                let id = output.add_block_auto(block);
                if is_top_level {
                    output.add_root(id.clone());
                }
                Some(id)
            }

            "method_declaration" => self.extract_method(node, source, file_path, output, scope),

            // Constructor declarations
            "constructor_declaration" => {
                self.extract_constructor(node, source, file_path, output, scope)
            }

            // Field declarations
            "field_declaration" => self.extract_field(node, source, file_path, output, scope),

            // Local variable declarations
            "local_variable_declaration" => {
                self.extract_local_variable(node, source, file_path, output, scope)
            }

            // If statements
            "if_statement" => self.extract_if_statement(node, source, file_path, output, scope),

            // Switch statements/expressions
            "switch_statement" | "switch_expression" => {
                self.extract_switch(node, source, file_path, output, scope)
            }

            // While loops
            "while_statement" => self.extract_while_loop(node, source, file_path, output, scope),

            // Do-while loops
            "do_statement" => self.extract_do_while(node, source, file_path, output, scope),

            // For loops
            "for_statement" => self.extract_for_loop(node, source, file_path, output, scope),

            // Enhanced for (for-each)
            "enhanced_for_statement" => {
                self.extract_enhanced_for(node, source, file_path, output, scope)
            }

            // Try statements
            "try_statement" | "try_with_resources_statement" => {
                self.extract_try(node, source, file_path, output, scope)
            }

            // Synchronized blocks
            "synchronized_statement" => {
                self.extract_synchronized(node, source, file_path, output, scope)
            }

            // Expression statements
            "expression_statement" => {
                if let Some(expr) = node.child(0) {
                    self.extract_node(&expr, source, file_path, output, scope, false)
                } else {
                    None
                }
            }

            "method_invocation" => {
                self.extract_method_invocation(node, source, file_path, output, scope)
            }

            // Object creation
            "object_creation_expression" => {
                self.extract_object_creation(node, source, file_path, output, scope)
            }

            // Assignment expressions
            "assignment_expression" => {
                self.extract_assignment(node, source, file_path, output, scope)
            }

            // Return statements
            "return_statement" => self.extract_return(node, source, file_path, output, scope),

            // Throw statements
            "throw_statement" => self.extract_throw(node, source, file_path, output, scope),

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

            // Yield statements (switch expressions)
            "yield_statement" => {
                let uses = self.extract_used_identifiers(node, source, scope);
                let block = Block::new("yield", BlockType::Yield)
                    .with_uses(uses)
                    .with_location(self.node_location(node, file_path))
                    .with_code(self.node_text(node, source));
                Some(output.add_block_auto(block))
            }

            // Assert statements
            "assert_statement" => {
                let uses = self.extract_used_identifiers(node, source, scope);
                let block = Block::new("assert", BlockType::Expression)
                    .with_uses(uses)
                    .with_location(self.node_location(node, file_path))
                    .with_code(self.node_text(node, source));
                Some(output.add_block_auto(block))
            }

            // Lambda expressions
            "lambda_expression" => self.extract_lambda(node, source, file_path, output, scope),

            // Ternary expressions
            "ternary_expression" => {
                let uses = self.extract_used_identifiers(node, source, scope);
                let condition = node
                    .child_by_field_name("condition")
                    .map(|c| self.node_text(&c, source));

                let block = Block::new("? :", BlockType::Ternary)
                    .with_uses(uses)
                    .with_location(self.node_location(node, file_path))
                    .with_code(self.node_text(node, source))
                    .with_metadata(BlockMetadata {
                        condition,
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
                .with_location(self.node_location(node, file_path));
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
                .with_location(self.node_location(node, file_path));
                Some(output.add_block_auto(block))
            }

            // Cast expressions
            "cast_expression" => {
                let uses = self.extract_used_identifiers(node, source, scope);
                let block = Block::new("cast", BlockType::Expression)
                    .with_uses(uses)
                    .with_location(self.node_location(node, file_path))
                    .with_code(self.node_text(node, source));
                Some(output.add_block_auto(block))
            }

            // Field access
            "field_access" => {
                let uses = self.extract_used_identifiers(node, source, scope);
                let block = Block::new(self.node_text(node, source), BlockType::Member)
                    .with_uses(uses)
                    .with_location(self.node_location(node, file_path));
                Some(output.add_block_auto(block))
            }

            // Array access
            "array_access" => {
                let uses = self.extract_used_identifiers(node, source, scope);
                let block = Block::new(
                    self.truncate(&self.node_text(node, source), 50),
                    BlockType::Index,
                )
                .with_uses(uses)
                .with_location(self.node_location(node, file_path));
                Some(output.add_block_auto(block))
            }

            // Array creation
            "array_creation_expression" => {
                let uses = self.extract_used_identifiers(node, source, scope);
                let block = Block::new("new array", BlockType::Array)
                    .with_uses(uses)
                    .with_location(self.node_location(node, file_path))
                    .with_code(self.node_text(node, source));
                Some(output.add_block_auto(block))
            }

            // Array initializer
            "array_initializer" => {
                let uses = self.extract_used_identifiers(node, source, scope);
                let block = Block::new("array initializer", BlockType::Array)
                    .with_uses(uses)
                    .with_location(self.node_location(node, file_path));
                Some(output.add_block_auto(block))
            }

            // Block
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

            // Annotations
            "annotation" | "marker_annotation" => {
                let name = self.get_annotation_name(node, source);
                let block = Block::new(format!("@{}", name), BlockType::Decorator)
                    .with_location(self.node_location(node, file_path))
                    .with_code(self.node_text(node, source));
                Some(output.add_block_auto(block))
            }

            // Skip these (comments don't contribute to flow)
            "comment" | "line_comment" | "block_comment" => None,
            "identifier" | "type_identifier" | "scoped_identifier" | "scoped_type_identifier" => {
                None
            }
            "integral_type" | "floating_point_type" | "boolean_type" | "void_type" => None,
            "decimal_integer_literal" | "decimal_floating_point_literal" | "string_literal" => None,
            "true" | "false" | "null_literal" | "character_literal" => None,
            "{" | "}" | "(" | ")" | "[" | "]" | "," | ";" | "." | "=" => None,
            "modifiers" | "dimensions" | "type_parameters" | "type_arguments" => None,

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

                if children.is_empty() {
                    None
                } else {
                    let block = Block::new(kind.to_string(), BlockType::Statement)
                        .with_children(children)
                        .with_location(self.node_location(node, file_path));
                    Some(output.add_block_auto(block))
                }
            }
        }
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

        // Extract annotations
        children.extend(self.extract_annotations(node, source, output));

        // Extract class body
        if let Some(body) = node.child_by_field_name("body") {
            let mut cursor = body.walk();
            for child in body.children(&mut cursor) {
                if let Some(member_id) =
                    self.extract_node(&child, source, file_path, output, &mut scope, false)
                {
                    children.push(member_id);
                }
            }
        }

        let modifiers = self.get_modifiers(node, source);

        let block = Block::new(name, BlockType::Class)
            .with_children(children)
            .with_location(self.node_location(node, file_path))
            .with_metadata(BlockMetadata {
                visibility: modifiers.visibility,
                is_static: Some(modifiers.is_static),
                ..Default::default()
            });

        let id = output.add_block_auto(block);
        if is_top_level {
            output.add_root(id.clone());
        }
        Some(id)
    }

    fn extract_interface(
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
            .unwrap_or_else(|| "interface".to_string());

        let mut children = Vec::new();
        let mut scope = Scope::new();

        // Extract annotations
        children.extend(self.extract_annotations(node, source, output));

        // Extract interface body
        if let Some(body) = node.child_by_field_name("body") {
            let mut cursor = body.walk();
            for child in body.children(&mut cursor) {
                if let Some(member_id) =
                    self.extract_node(&child, source, file_path, output, &mut scope, false)
                {
                    children.push(member_id);
                }
            }
        }

        let block = Block::new(name, BlockType::Interface)
            .with_children(children)
            .with_location(self.node_location(node, file_path));

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
        let mut scope = Scope::new();

        // Extract annotations
        children.extend(self.extract_annotations(node, source, output));

        // Extract enum body
        if let Some(body) = node.child_by_field_name("body") {
            let mut cursor = body.walk();
            for child in body.children(&mut cursor) {
                match child.kind() {
                    "enum_constant" => {
                        let const_name = child
                            .child_by_field_name("name")
                            .map(|n| self.node_text(&n, source))
                            .unwrap_or_else(|| "constant".to_string());

                        let const_block = Block::new(const_name, BlockType::EnumMember)
                            .with_location(self.node_location(&child, file_path));
                        children.push(output.add_block_auto(const_block));
                    }
                    _ => {
                        if let Some(member_id) =
                            self.extract_node(&child, source, file_path, output, &mut scope, false)
                        {
                            children.push(member_id);
                        }
                    }
                }
            }
        }

        let block = Block::new(name, BlockType::Enum)
            .with_children(children)
            .with_location(self.node_location(node, file_path));

        let id = output.add_block_auto(block);
        if is_top_level {
            output.add_root(id.clone());
        }
        Some(id)
    }

    fn extract_record(
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
            .unwrap_or_else(|| "record".to_string());

        let mut children = Vec::new();
        let mut scope = Scope::new();
        let mut produces = Vec::new();

        // Extract record components (parameters)
        if let Some(params) = node.child_by_field_name("parameters") {
            produces = self.extract_formal_parameters(&params, source, &mut scope);
        }

        // Extract annotations
        children.extend(self.extract_annotations(node, source, output));

        // Extract record body
        if let Some(body) = node.child_by_field_name("body") {
            let mut cursor = body.walk();
            for child in body.children(&mut cursor) {
                if let Some(member_id) =
                    self.extract_node(&child, source, file_path, output, &mut scope, false)
                {
                    children.push(member_id);
                }
            }
        }

        let block = Block::new(name, BlockType::Class)
            .with_produces(produces)
            .with_children(children)
            .with_location(self.node_location(node, file_path));

        let id = output.add_block_auto(block);
        if is_top_level {
            output.add_root(id.clone());
        }
        Some(id)
    }

    fn extract_method(
        &self,
        node: &Node,
        source: &str,
        file_path: &str,
        output: &mut FlowMapOutput,
        _parent_scope: &mut Scope,
    ) -> Option<BlockId> {
        let name = node
            .child_by_field_name("name")
            .map(|n| self.node_text(&n, source))
            .unwrap_or_else(|| "method".to_string());

        let mut scope = Scope::new();
        let params = node
            .child_by_field_name("parameters")
            .map(|p| self.extract_formal_parameters(&p, source, &mut scope))
            .unwrap_or_default();

        let mut children = Vec::new();

        // Extract annotations
        children.extend(self.extract_annotations(node, source, output));

        // Extract body
        if let Some(body) = node.child_by_field_name("body") {
            if let Some(body_id) =
                self.extract_node(&body, source, file_path, output, &mut scope, false)
            {
                children.push(body_id);
            }
        }

        let modifiers = self.get_modifiers(node, source);
        let return_type = node
            .child_by_field_name("type")
            .map(|t| self.node_text(&t, source));

        let block_type = if modifiers.is_static {
            BlockType::StaticMethod
        } else {
            BlockType::Method
        };

        let block = Block::new(name, block_type)
            .with_produces(params.clone())
            .with_children(children)
            .with_location(self.node_location(node, file_path))
            .with_metadata(BlockMetadata {
                visibility: modifiers.visibility,
                is_static: Some(modifiers.is_static),
                parameters: Some(params),
                return_type,
                ..Default::default()
            });

        Some(output.add_block_auto(block))
    }

    fn extract_constructor(
        &self,
        node: &Node,
        source: &str,
        file_path: &str,
        output: &mut FlowMapOutput,
        _parent_scope: &mut Scope,
    ) -> Option<BlockId> {
        let name = node
            .child_by_field_name("name")
            .map(|n| self.node_text(&n, source))
            .unwrap_or_else(|| "constructor".to_string());

        let mut scope = Scope::new();
        let params = node
            .child_by_field_name("parameters")
            .map(|p| self.extract_formal_parameters(&p, source, &mut scope))
            .unwrap_or_default();

        let mut children = Vec::new();

        // Extract annotations
        children.extend(self.extract_annotations(node, source, output));

        // Extract body
        if let Some(body) = node.child_by_field_name("body") {
            if let Some(body_id) =
                self.extract_node(&body, source, file_path, output, &mut scope, false)
            {
                children.push(body_id);
            }
        }

        let modifiers = self.get_modifiers(node, source);

        let block = Block::new(name, BlockType::Constructor)
            .with_produces(params.clone())
            .with_children(children)
            .with_location(self.node_location(node, file_path))
            .with_metadata(BlockMetadata {
                visibility: modifiers.visibility,
                parameters: Some(params),
                ..Default::default()
            });

        Some(output.add_block_auto(block))
    }

    fn extract_field(
        &self,
        node: &Node,
        source: &str,
        file_path: &str,
        output: &mut FlowMapOutput,
        scope: &mut Scope,
    ) -> Option<BlockId> {
        let mut produces = Vec::new();
        let mut uses = Vec::new();
        let mut children = Vec::new();

        // Find variable declarators
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "variable_declarator" {
                if let Some(name_node) = child.child_by_field_name("name") {
                    let name = self.node_text(&name_node, source);
                    scope.define(&name);
                    produces.push(name);
                }
                if let Some(value) = child.child_by_field_name("value") {
                    uses.extend(self.extract_used_identifiers(&value, source, scope));
                    if self.is_complex_expression(&value) {
                        if let Some(val_id) =
                            self.extract_node(&value, source, file_path, output, scope, false)
                        {
                            children.push(val_id);
                        }
                    }
                }
            }
        }

        let modifiers = self.get_modifiers(node, source);
        let block_type = if modifiers.is_static {
            BlockType::StaticProperty
        } else {
            BlockType::Property
        };

        let name = produces.join(", ");

        // Extract annotations
        let annotations = self.extract_annotations(node, source, output);
        let mut all_children = annotations;
        all_children.extend(children);

        let block = Block::new(name.clone(), block_type)
            .with_uses(uses)
            .with_produces(produces)
            .with_children(all_children)
            .with_location(self.node_location(node, file_path))
            .with_code(self.node_text(node, source))
            .with_metadata(BlockMetadata {
                visibility: modifiers.visibility,
                is_static: Some(modifiers.is_static),
                ..Default::default()
            });

        Some(output.add_block_auto(block))
    }

    fn extract_local_variable(
        &self,
        node: &Node,
        source: &str,
        file_path: &str,
        output: &mut FlowMapOutput,
        scope: &mut Scope,
    ) -> Option<BlockId> {
        let mut produces = Vec::new();
        let mut uses = Vec::new();
        let mut children = Vec::new();

        // Find variable declarators
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "variable_declarator" {
                if let Some(name_node) = child.child_by_field_name("name") {
                    let name = self.node_text(&name_node, source);
                    scope.define(&name);
                    produces.push(name);
                }
                if let Some(value) = child.child_by_field_name("value") {
                    uses.extend(self.extract_used_identifiers(&value, source, scope));
                    if self.is_complex_expression(&value) {
                        if let Some(val_id) =
                            self.extract_node(&value, source, file_path, output, scope, false)
                        {
                            children.push(val_id);
                        }
                    }
                }
            }
        }

        let name = produces.join(", ");

        let block = Block::new(name.clone(), BlockType::Variable)
            .with_uses(uses)
            .with_produces(produces)
            .with_children(children)
            .with_location(self.node_location(node, file_path))
            .with_code(self.node_text(node, source));

        Some(output.add_block_auto(block))
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
        let uses = condition
            .map(|c| self.extract_used_identifiers(&c, source, scope))
            .unwrap_or_default();

        let mut children = Vec::new();

        // Then branch (consequence)
        if let Some(consequence) = node.child_by_field_name("consequence") {
            if let Some(then_id) =
                self.extract_node(&consequence, source, file_path, output, scope, false)
            {
                children.push(then_id);
            }
        }

        // Else branch (alternative)
        if let Some(alternative) = node.child_by_field_name("alternative") {
            if alternative.kind() == "if_statement" {
                // else-if chain
                if let Some(else_if_id) =
                    self.extract_if_statement(&alternative, source, file_path, output, scope)
                {
                    let else_block = Block::new("else", BlockType::Else)
                        .with_children(vec![else_if_id])
                        .with_location(self.node_location(&alternative, file_path));
                    children.push(output.add_block_auto(else_block));
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

    fn extract_switch(
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

        // Extract switch body
        if let Some(body) = node.child_by_field_name("body") {
            let mut cursor = body.walk();
            for child in body.children(&mut cursor) {
                match child.kind() {
                    "switch_block_statement_group" | "switch_rule" => {
                        // Extract case label
                        let label_text = self.get_case_label(&child, source);

                        let mut case_children = Vec::new();
                        let mut case_cursor = child.walk();
                        for case_child in child.children(&mut case_cursor) {
                            if case_child.kind() != "switch_label" {
                                if let Some(stmt_id) = self.extract_node(
                                    &case_child,
                                    source,
                                    file_path,
                                    output,
                                    scope,
                                    false,
                                ) {
                                    case_children.push(stmt_id);
                                }
                            }
                        }

                        let block_type = if label_text == "default" {
                            BlockType::Default
                        } else {
                            BlockType::Case
                        };
                        let case_block = Block::new(label_text, block_type)
                            .with_children(case_children)
                            .with_location(self.node_location(&child, file_path));

                        children.push(output.add_block_auto(case_block));
                    }
                    _ => {}
                }
            }
        }

        let block = Block::new(
            format!("switch {}", self.truncate(&condition_text, 30)),
            BlockType::Switch,
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

    fn extract_while_loop(
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

        if let Some(body) = node.child_by_field_name("body") {
            if let Some(body_id) = self.extract_node(&body, source, file_path, output, scope, false)
            {
                children.push(body_id);
            }
        }

        let block = Block::new(
            format!("while {}", self.truncate(&condition_text, 30)),
            BlockType::While,
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

    fn extract_do_while(
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

        if let Some(body) = node.child_by_field_name("body") {
            if let Some(body_id) = self.extract_node(&body, source, file_path, output, scope, false)
            {
                children.push(body_id);
            }
        }

        let block = Block::new(
            format!("do...while {}", self.truncate(&condition_text, 30)),
            BlockType::DoWhile,
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

        // Extract init
        if let Some(init) = node.child_by_field_name("init") {
            if let Some(init_id) = self.extract_node(&init, source, file_path, output, scope, false)
            {
                children.push(init_id);
            }
            // Extract variable declarations from init
            let mut cursor = init.walk();
            for child in init.children(&mut cursor) {
                if child.kind() == "variable_declarator" {
                    if let Some(name) = child.child_by_field_name("name") {
                        let n = self.node_text(&name, source);
                        scope.define(&n);
                        produces.push(n);
                    }
                }
            }
        }

        // Extract condition
        if let Some(condition) = node.child_by_field_name("condition") {
            uses.extend(self.extract_used_identifiers(&condition, source, scope));
        }

        // Extract update
        if let Some(update) = node.child_by_field_name("update") {
            uses.extend(self.extract_used_identifiers(&update, source, scope));
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
            .map(|c| self.node_text(&c, source));

        let block = Block::new("for", BlockType::For)
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

    fn extract_enhanced_for(
        &self,
        node: &Node,
        source: &str,
        file_path: &str,
        output: &mut FlowMapOutput,
        scope: &mut Scope,
    ) -> Option<BlockId> {
        let mut produces = Vec::new();

        // Extract loop variable
        if let Some(var_name) = node.child_by_field_name("name") {
            let name = self.node_text(&var_name, source);
            scope.define(&name);
            produces.push(name);
        }

        // Extract iterable
        let value = node.child_by_field_name("value");
        let value_text = value
            .map(|v| self.node_text(&v, source))
            .unwrap_or_default();
        let uses = value
            .map(|v| self.extract_used_identifiers(&v, source, scope))
            .unwrap_or_default();

        let mut children = Vec::new();

        if let Some(body) = node.child_by_field_name("body") {
            if let Some(body_id) = self.extract_node(&body, source, file_path, output, scope, false)
            {
                children.push(body_id);
            }
        }

        let block = Block::new(
            format!("for : {}", self.truncate(&value_text, 30)),
            BlockType::ForOf,
        )
        .with_uses(uses)
        .with_produces(produces)
        .with_children(children)
        .with_location(self.node_location(node, file_path));

        Some(output.add_block_auto(block))
    }

    fn extract_try(
        &self,
        node: &Node,
        source: &str,
        file_path: &str,
        output: &mut FlowMapOutput,
        scope: &mut Scope,
    ) -> Option<BlockId> {
        let mut children = Vec::new();

        // Extract resources (try-with-resources)
        if let Some(resources) = node.child_by_field_name("resources") {
            if let Some(res_id) =
                self.extract_node(&resources, source, file_path, output, scope, false)
            {
                children.push(res_id);
            }
        }

        // Try body
        if let Some(body) = node.child_by_field_name("body") {
            if let Some(try_id) = self.extract_node(&body, source, file_path, output, scope, false)
            {
                let try_block = Block::new("try", BlockType::Try)
                    .with_children(vec![try_id])
                    .with_location(self.node_location(&body, file_path));
                children.push(output.add_block_auto(try_block));
            }
        }

        // Catch clauses
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "catch_clause" {
                let catch_param = child
                    .child_by_field_name("parameter")
                    .map(|p| self.get_catch_parameter(&p, source));

                let mut catch_children = Vec::new();
                if let Some(catch_body) = child.child_by_field_name("body") {
                    if let Some(body_id) =
                        self.extract_node(&catch_body, source, file_path, output, scope, false)
                    {
                        catch_children.push(body_id);
                    }
                }

                let label = catch_param
                    .as_ref()
                    .map(|(t, n)| format!("catch ({} {})", t, n))
                    .unwrap_or_else(|| "catch".to_string());

                let catch_block = Block::new(label, BlockType::Catch)
                    .with_produces(catch_param.map(|(_, n)| vec![n]).unwrap_or_default())
                    .with_children(catch_children)
                    .with_location(self.node_location(&child, file_path));

                children.push(output.add_block_auto(catch_block));
            }
        }

        // Finally clause
        if let Some(finally) = node.child_by_field_name("finally_clause") {
            if let Some(finally_body) = finally.child(1) {
                if let Some(finally_id) =
                    self.extract_node(&finally_body, source, file_path, output, scope, false)
                {
                    let finally_block = Block::new("finally", BlockType::Finally)
                        .with_children(vec![finally_id])
                        .with_location(self.node_location(&finally, file_path));
                    children.push(output.add_block_auto(finally_block));
                }
            }
        }

        let block = Block::new("try...catch", BlockType::TryCatch)
            .with_children(children)
            .with_location(self.node_location(node, file_path));

        Some(output.add_block_auto(block))
    }

    fn extract_synchronized(
        &self,
        node: &Node,
        source: &str,
        file_path: &str,
        output: &mut FlowMapOutput,
        scope: &mut Scope,
    ) -> Option<BlockId> {
        let lock = node.child(2); // synchronized ( expr )
        let lock_text = lock.map(|l| self.node_text(&l, source)).unwrap_or_default();
        let uses = lock
            .map(|l| self.extract_used_identifiers(&l, source, scope))
            .unwrap_or_default();

        let mut children = Vec::new();

        if let Some(body) = node.child_by_field_name("body") {
            if let Some(body_id) = self.extract_node(&body, source, file_path, output, scope, false)
            {
                children.push(body_id);
            }
        }

        let block = Block::new(
            format!("synchronized({})", self.truncate(&lock_text, 20)),
            BlockType::Block,
        )
        .with_uses(uses)
        .with_children(children)
        .with_location(self.node_location(node, file_path));

        Some(output.add_block_auto(block))
    }

    fn extract_method_invocation(
        &self,
        node: &Node,
        source: &str,
        file_path: &str,
        output: &mut FlowMapOutput,
        scope: &mut Scope,
    ) -> Option<BlockId> {
        let object = node.child_by_field_name("object");
        let name = node
            .child_by_field_name("name")
            .map(|n| self.node_text(&n, source))
            .unwrap_or_else(|| "method".to_string());

        let mut uses = Vec::new();
        let mut children = Vec::new();

        // Track object
        if let Some(obj) = object {
            uses.extend(self.extract_used_identifiers(&obj, source, scope));
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

        let (block_type, label) = if let Some(obj) = object {
            let obj_text = self.node_text(&obj, source);
            (BlockType::MethodCall, format!("{}.{}", obj_text, name))
        } else {
            (BlockType::Call, name)
        };

        let block = Block::new(label, block_type)
            .with_uses(uses)
            .with_children(children)
            .with_location(self.node_location(node, file_path))
            .with_code(self.node_text(node, source));

        Some(output.add_block_auto(block))
    }

    fn extract_object_creation(
        &self,
        node: &Node,
        source: &str,
        file_path: &str,
        output: &mut FlowMapOutput,
        scope: &mut Scope,
    ) -> Option<BlockId> {
        let type_name = node
            .child_by_field_name("type")
            .map(|t| self.node_text(&t, source))
            .unwrap_or_else(|| "Object".to_string());

        let mut uses = vec![type_name.clone()];
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

        // Anonymous class body
        if let Some(body) = node.child_by_field_name("body") {
            if let Some(body_id) = self.extract_node(&body, source, file_path, output, scope, false)
            {
                children.push(body_id);
            }
        }

        let block = Block::new(format!("new {}", type_name), BlockType::New)
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

        let left_text = self.node_text(&left, source);
        let uses = self.extract_used_identifiers(&right, source, scope);

        let mut children = Vec::new();
        if self.is_complex_expression(&right) {
            if let Some(right_id) =
                self.extract_node(&right, source, file_path, output, scope, false)
            {
                children.push(right_id);
            }
        }

        let block = Block::new(left_text.clone(), BlockType::Assignment)
            .with_uses(uses)
            .with_produces(vec![left_text])
            .with_children(children)
            .with_location(self.node_location(node, file_path))
            .with_code(self.node_text(node, source));

        Some(output.add_block_auto(block))
    }

    fn extract_return(
        &self,
        node: &Node,
        source: &str,
        file_path: &str,
        output: &mut FlowMapOutput,
        scope: &mut Scope,
    ) -> Option<BlockId> {
        let mut uses = Vec::new();
        let mut children = Vec::new();

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

    fn extract_throw(
        &self,
        node: &Node,
        source: &str,
        file_path: &str,
        output: &mut FlowMapOutput,
        scope: &mut Scope,
    ) -> Option<BlockId> {
        let mut uses = Vec::new();
        let mut children = Vec::new();

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

    fn extract_lambda(
        &self,
        node: &Node,
        source: &str,
        file_path: &str,
        output: &mut FlowMapOutput,
        _parent_scope: &mut Scope,
    ) -> Option<BlockId> {
        let mut scope = Scope::new();
        let params = node
            .child_by_field_name("parameters")
            .map(|p| self.extract_lambda_parameters(&p, source, &mut scope))
            .unwrap_or_default();

        let mut children = Vec::new();

        if let Some(body) = node.child_by_field_name("body") {
            if let Some(body_id) =
                self.extract_node(&body, source, file_path, output, &mut scope, false)
            {
                children.push(body_id);
            }
        }

        let block = Block::new("lambda", BlockType::Arrow)
            .with_produces(params.clone())
            .with_children(children)
            .with_location(self.node_location(node, file_path))
            .with_code(self.node_text(node, source))
            .with_metadata(BlockMetadata {
                parameters: Some(params),
                ..Default::default()
            });

        Some(output.add_block_auto(block))
    }

    // Helper methods

    fn extract_formal_parameters(
        &self,
        node: &Node,
        source: &str,
        scope: &mut Scope,
    ) -> Vec<String> {
        let mut params = Vec::new();

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "formal_parameter" || child.kind() == "spread_parameter" {
                if let Some(name) = child.child_by_field_name("name") {
                    let n = self.node_text(&name, source);
                    scope.define(&n);
                    params.push(n);
                }
            }
        }

        params
    }

    fn extract_lambda_parameters(
        &self,
        node: &Node,
        source: &str,
        scope: &mut Scope,
    ) -> Vec<String> {
        let mut params = Vec::new();

        match node.kind() {
            "identifier" => {
                let name = self.node_text(node, source);
                scope.define(&name);
                params.push(name);
            }
            "inferred_parameters" | "formal_parameters" => {
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    if child.kind() == "identifier" || child.kind() == "formal_parameter" {
                        if let Some(name) = child.child_by_field_name("name").or(Some(child)) {
                            if name.kind() == "identifier" {
                                let n = self.node_text(&name, source);
                                scope.define(&n);
                                params.push(n);
                            }
                        }
                    }
                }
            }
            _ => {}
        }

        params
    }

    fn extract_used_identifiers(&self, node: &Node, source: &str, scope: &Scope) -> Vec<String> {
        let mut identifiers = Vec::new();

        match node.kind() {
            "identifier" => {
                let name = self.node_text(node, source);
                if !self.is_builtin(&name) && !scope.is_defined(&name) {
                    identifiers.push(name);
                }
            }
            "field_access" | "method_invocation" => {
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

    fn extract_annotations(
        &self,
        node: &Node,
        source: &str,
        output: &mut FlowMapOutput,
    ) -> Vec<BlockId> {
        let mut annotations = Vec::new();

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "modifiers" {
                let mut mod_cursor = child.walk();
                for mod_child in child.children(&mut mod_cursor) {
                    if mod_child.kind() == "annotation" || mod_child.kind() == "marker_annotation" {
                        let name = self.get_annotation_name(&mod_child, source);
                        let ann_block = Block::new(format!("@{}", name), BlockType::Decorator)
                            .with_code(self.node_text(&mod_child, source));
                        annotations.push(output.add_block_auto(ann_block));
                    }
                }
            }
        }

        annotations
    }

    fn get_annotation_name(&self, node: &Node, source: &str) -> String {
        if let Some(name) = node.child_by_field_name("name") {
            self.node_text(&name, source)
        } else {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.kind() == "identifier" || child.kind() == "scoped_identifier" {
                    return self.node_text(&child, source);
                }
            }
            "annotation".to_string()
        }
    }

    fn get_scoped_identifier(&self, node: &Node, source: &str) -> String {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "scoped_identifier" || child.kind() == "identifier" {
                return self.node_text(&child, source);
            }
        }
        self.node_text(node, source)
    }

    fn get_case_label(&self, node: &Node, source: &str) -> String {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "switch_label" {
                let text = self.node_text(&child, source);
                if text.starts_with("case ") {
                    return format!("case {}", text[5..].trim_end_matches(':').trim());
                } else if text.starts_with("default") {
                    return "default".to_string();
                }
            }
        }
        "case".to_string()
    }

    fn get_catch_parameter(&self, node: &Node, source: &str) -> (String, String) {
        let type_name = node
            .child_by_field_name("type")
            .or_else(|| node.child(0))
            .map(|t| self.node_text(&t, source))
            .unwrap_or_else(|| "Exception".to_string());

        let param_name = node
            .child_by_field_name("name")
            .map(|n| self.node_text(&n, source))
            .unwrap_or_else(|| "e".to_string());

        (type_name, param_name)
    }

    fn get_modifiers(&self, node: &Node, source: &str) -> Modifiers {
        let mut visibility = None;
        let mut is_static = false;

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "modifiers" {
                let text = self.node_text(&child, source);
                if text.contains("public") {
                    visibility = Some("public".to_string());
                } else if text.contains("private") {
                    visibility = Some("private".to_string());
                } else if text.contains("protected") {
                    visibility = Some("protected".to_string());
                }
                if text.contains("static") {
                    is_static = true;
                }
            }
        }

        Modifiers {
            visibility,
            is_static,
        }
    }

    fn is_complex_expression(&self, node: &Node) -> bool {
        matches!(
            node.kind(),
            "method_invocation"
                | "object_creation_expression"
                | "lambda_expression"
                | "array_creation_expression"
                | "array_initializer"
                | "ternary_expression"
        )
    }

    fn is_builtin(&self, name: &str) -> bool {
        matches!(
            name,
            "System"
                | "String"
                | "Integer"
                | "Long"
                | "Double"
                | "Float"
                | "Boolean"
                | "Object"
                | "Class"
                | "Exception"
                | "RuntimeException"
                | "Error"
                | "Throwable"
                | "Thread"
                | "Runnable"
                | "Callable"
                | "List"
                | "ArrayList"
                | "LinkedList"
                | "Map"
                | "HashMap"
                | "TreeMap"
                | "Set"
                | "HashSet"
                | "TreeSet"
                | "Collection"
                | "Collections"
                | "Arrays"
                | "Optional"
                | "Stream"
                | "Collectors"
                | "Math"
                | "Random"
                | "Date"
                | "Calendar"
                | "LocalDate"
                | "LocalDateTime"
                | "null"
                | "true"
                | "false"
                | "this"
                | "super"
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

impl Default for JavaBlockExtractor {
    fn default() -> Self {
        Self::new().expect("Failed to create JavaBlockExtractor")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_class() {
        let mut extractor = JavaBlockExtractor::new().unwrap();
        let source = r#"
public class Calculator {
    public int add(int a, int b) {
        return a + b;
    }
}
"#;

        let output = extractor.parse_file(source, "Calculator.java").unwrap();
        assert!(output.block_count() > 0);

        let json = serde_json::to_string_pretty(&output).unwrap();
        assert!(json.contains("Calculator"));
        assert!(json.contains("add"));
    }

    #[test]
    fn test_spring_controller() {
        let mut extractor = JavaBlockExtractor::new().unwrap();
        let source = r#"
@RestController
@RequestMapping("/api/users")
public class UserController {
    @Autowired
    private UserService userService;

    @GetMapping("/{id}")
    public User getUser(@PathVariable Long id) {
        return userService.findById(id);
    }
}
"#;

        let output = extractor.parse_file(source, "UserController.java").unwrap();
        let json = serde_json::to_string_pretty(&output).unwrap();

        assert!(json.contains("@RestController"));
        assert!(json.contains("@GetMapping"));
        assert!(json.contains("getUser"));
    }
}
