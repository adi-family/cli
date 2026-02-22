use lib_flowmap_core::{Block, BlockId, BlockMetadata, BlockType, FlowMapOutput, Location};
use std::collections::HashSet;
use tree_sitter::{Node, Parser};

/// Block-based extractor for Python
pub struct PythonBlockExtractor {
    parser: Parser,
}

struct Scope {
    defined: HashSet<String>,
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

    fn is_defined(&self, name: &str) -> bool {
        self.defined.contains(name)
    }
}

impl PythonBlockExtractor {
    pub fn new() -> crate::Result<Self> {
        let mut parser = Parser::new();
        let language = tree_sitter_python::LANGUAGE;
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
            .with_language("python");

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
            "module" => {
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

            "import_statement" => {
                let imports = self.extract_import_names(node, source);
                for name in &imports {
                    scope.define(name);
                }

                let block = Block::new(format!("import {}", imports.join(", ")), BlockType::Import)
                    .with_produces(imports)
                    .with_location(self.node_location(node, file_path))
                    .with_code(self.node_text(node, source));

                let id = output.add_block_auto(block);
                if is_top_level {
                    output.add_root(id.clone());
                }
                Some(id)
            }

            "import_from_statement" => {
                let imports = self.extract_import_names(node, source);
                let from_module = node
                    .child_by_field_name("module_name")
                    .map(|n| self.node_text(&n, source));

                for name in &imports {
                    scope.define(name);
                }

                let block = Block::new(
                    format!("from {} import", from_module.as_deref().unwrap_or("?")),
                    BlockType::Import,
                )
                .with_produces(imports)
                .with_location(self.node_location(node, file_path))
                .with_code(self.node_text(node, source))
                .with_metadata(BlockMetadata {
                    target: from_module,
                    ..Default::default()
                });

                let id = output.add_block_auto(block);
                if is_top_level {
                    output.add_root(id.clone());
                }
                Some(id)
            }

            "function_definition" => {
                self.extract_function(node, source, file_path, output, false, is_top_level)
            }

            "async_function_definition" => {
                self.extract_function(node, source, file_path, output, true, is_top_level)
            }

            // Class definitions
            "class_definition" => self.extract_class(node, source, file_path, output, is_top_level),

            // Decorated definitions
            "decorated_definition" => {
                self.extract_decorated(node, source, file_path, output, scope, is_top_level)
            }

            // If statements
            "if_statement" => self.extract_if_statement(node, source, file_path, output, scope),

            // While loops
            "while_statement" => self.extract_while_loop(node, source, file_path, output, scope),

            // For loops
            "for_statement" => self.extract_for_loop(node, source, file_path, output, scope),

            // Try statements
            "try_statement" => self.extract_try_statement(node, source, file_path, output, scope),

            // With statements
            "with_statement" => self.extract_with_statement(node, source, file_path, output, scope),

            // Match statements (Python 3.10+)
            "match_statement" => {
                self.extract_match_statement(node, source, file_path, output, scope)
            }

            // Expression statements
            "expression_statement" => {
                if let Some(expr) = node.child(0) {
                    self.extract_node(&expr, source, file_path, output, scope, false)
                } else {
                    None
                }
            }

            // Assignments
            "assignment" | "augmented_assignment" => {
                self.extract_assignment(node, source, file_path, output, scope)
            }

            // Call expressions
            "call" => self.extract_call(node, source, file_path, output, scope),

            // Await expressions
            "await" => self.extract_await(node, source, file_path, output, scope),

            // Return statements
            "return_statement" => self.extract_return(node, source, file_path, output, scope),

            // Raise statements
            "raise_statement" => self.extract_raise(node, source, file_path, output, scope),

            // Yield expressions
            "yield" | "yield_from" => {
                let is_from = kind == "yield_from";
                let uses = self.extract_used_identifiers(node, source, scope);
                let block_type = if is_from {
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

            // Assert statements
            "assert_statement" => {
                let uses = self.extract_used_identifiers(node, source, scope);
                let block = Block::new("assert", BlockType::Expression)
                    .with_uses(uses)
                    .with_location(self.node_location(node, file_path))
                    .with_code(self.node_text(node, source));
                Some(output.add_block_auto(block))
            }

            // Pass, break, continue
            "pass_statement" => {
                let block = Block::new("pass", BlockType::Statement)
                    .with_location(self.node_location(node, file_path));
                Some(output.add_block_auto(block))
            }
            "break_statement" => {
                let block = Block::new("break", BlockType::Break)
                    .with_location(self.node_location(node, file_path));
                Some(output.add_block_auto(block))
            }
            "continue_statement" => {
                let block = Block::new("continue", BlockType::Continue)
                    .with_location(self.node_location(node, file_path));
                Some(output.add_block_auto(block))
            }

            // List/dict/set comprehensions
            "list_comprehension"
            | "dictionary_comprehension"
            | "set_comprehension"
            | "generator_expression" => {
                let uses = self.extract_used_identifiers(node, source, scope);
                let block_type = match kind {
                    "list_comprehension" => BlockType::Array,
                    "dictionary_comprehension" => BlockType::Object,
                    "set_comprehension" => BlockType::Object,
                    _ => BlockType::Generator,
                };

                let block = Block::new(kind.replace('_', " "), block_type)
                    .with_uses(uses)
                    .with_location(self.node_location(node, file_path))
                    .with_code(self.node_text(node, source));
                Some(output.add_block_auto(block))
            }

            // Lambda expressions
            "lambda" => {
                let mut scope = Scope::new();
                let params = self.extract_parameters(node, source, &mut scope);

                let uses = if let Some(body) = node.child_by_field_name("body") {
                    self.extract_used_identifiers(&body, source, &scope)
                } else {
                    Vec::new()
                };

                let block = Block::new("lambda", BlockType::Arrow)
                    .with_uses(uses)
                    .with_produces(params.clone())
                    .with_location(self.node_location(node, file_path))
                    .with_code(self.node_text(node, source))
                    .with_metadata(BlockMetadata {
                        parameters: Some(params),
                        ..Default::default()
                    });
                Some(output.add_block_auto(block))
            }

            // Conditional expression (ternary)
            "conditional_expression" => {
                let uses = self.extract_used_identifiers(node, source, scope);
                let block = Block::new("if...else", BlockType::Ternary)
                    .with_uses(uses)
                    .with_location(self.node_location(node, file_path))
                    .with_code(self.node_text(node, source));
                Some(output.add_block_auto(block))
            }

            // Binary/comparison operations
            "binary_operator" | "comparison_operator" | "boolean_operator" => {
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

            // Unary operations
            "unary_operator" | "not_operator" => {
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

            // Attribute access
            "attribute" => {
                let uses = self.extract_used_identifiers(node, source, scope);
                let block = Block::new(self.node_text(node, source), BlockType::Member)
                    .with_uses(uses)
                    .with_location(self.node_location(node, file_path));
                Some(output.add_block_auto(block))
            }

            // Subscript access
            "subscript" => {
                let uses = self.extract_used_identifiers(node, source, scope);
                let block = Block::new(
                    self.truncate(&self.node_text(node, source), 50),
                    BlockType::Index,
                )
                .with_uses(uses)
                .with_location(self.node_location(node, file_path));
                Some(output.add_block_auto(block))
            }

            // Dictionary
            "dictionary" => {
                let mut uses = Vec::new();
                let mut produces = Vec::new();

                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    if child.kind() == "pair" {
                        if let Some(key) = child.child_by_field_name("key") {
                            if key.kind() == "string" {
                                let key_text = self.node_text(&key, source);
                                produces.push(
                                    key_text.trim_matches(|c| c == '"' || c == '\'').to_string(),
                                );
                            }
                        }
                        if let Some(value) = child.child_by_field_name("value") {
                            uses.extend(self.extract_used_identifiers(&value, source, scope));
                        }
                    } else if child.kind() == "dictionary_splat" {
                        uses.extend(self.extract_used_identifiers(&child, source, scope));
                    }
                }

                let block = Block::new("dict", BlockType::Object)
                    .with_uses(uses)
                    .with_produces(produces)
                    .with_location(self.node_location(node, file_path));
                Some(output.add_block_auto(block))
            }

            // List
            "list" => {
                let uses = self.extract_used_identifiers(node, source, scope);
                let block = Block::new("list", BlockType::Array)
                    .with_uses(uses)
                    .with_location(self.node_location(node, file_path));
                Some(output.add_block_auto(block))
            }

            // Tuple
            "tuple" => {
                let uses = self.extract_used_identifiers(node, source, scope);
                let block = Block::new("tuple", BlockType::Array)
                    .with_uses(uses)
                    .with_location(self.node_location(node, file_path));
                Some(output.add_block_auto(block))
            }

            // Set
            "set" => {
                let uses = self.extract_used_identifiers(node, source, scope);
                let block = Block::new("set", BlockType::Object)
                    .with_uses(uses)
                    .with_location(self.node_location(node, file_path));
                Some(output.add_block_auto(block))
            }

            // F-string
            "string"
                if self.node_text(node, source).starts_with("f\"")
                    || self.node_text(node, source).starts_with("f'") =>
            {
                let uses = self.extract_used_identifiers(node, source, scope);
                let block = Block::new("f-string", BlockType::Template)
                    .with_uses(uses)
                    .with_location(self.node_location(node, file_path))
                    .with_code(self.node_text(node, source));
                Some(output.add_block_auto(block))
            }

            // Block (suite in Python)
            "block" => {
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
                } else if children.len() == 1 {
                    Some(children.into_iter().next().unwrap())
                } else {
                    let block = Block::new("block", BlockType::Block)
                        .with_children(children)
                        .with_location(self.node_location(node, file_path));
                    Some(output.add_block_auto(block))
                }
            }

            // Skip these (comments don't contribute to flow)
            "comment" | "identifier" | "integer" | "float" | "string" | "true" | "false"
            | "none" => None,
            ":" | "," | "(" | ")" | "[" | "]" | "{" | "}" | "=" | "->" => None,

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

    fn extract_function(
        &self,
        node: &Node,
        source: &str,
        file_path: &str,
        output: &mut FlowMapOutput,
        is_async: bool,
        is_top_level: bool,
    ) -> Option<BlockId> {
        let name = node
            .child_by_field_name("name")
            .map(|n| self.node_text(&n, source))
            .unwrap_or_else(|| "anonymous".to_string());

        let mut scope = Scope::new();
        let params = self.extract_parameters(node, source, &mut scope);
        let mut children = Vec::new();

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

        let block_type = if is_async {
            BlockType::AsyncFunction
        } else {
            BlockType::Function
        };

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

        // Extract class body
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
            .with_location(self.node_location(node, file_path));

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
            "function_definition" => {
                let name = node
                    .child_by_field_name("name")
                    .map(|n| self.node_text(&n, source))
                    .unwrap_or_else(|| "method".to_string());

                let block_type = if name == "__init__" {
                    BlockType::Constructor
                } else if name.starts_with("__") && name.ends_with("__") {
                    BlockType::Method // Dunder methods
                } else {
                    BlockType::Method
                };

                let params = self.extract_parameters(node, source, scope);
                let mut method_scope = Scope::new();
                for p in &params {
                    method_scope.define(p);
                }

                let mut children = Vec::new();
                if let Some(body) = node.child_by_field_name("body") {
                    if let Some(body_id) = self.extract_node(
                        &body,
                        source,
                        file_path,
                        output,
                        &mut method_scope,
                        false,
                    ) {
                        children.push(body_id);
                    }
                }

                let block = Block::new(name, block_type)
                    .with_produces(params.clone())
                    .with_children(children)
                    .with_location(self.node_location(node, file_path))
                    .with_metadata(BlockMetadata {
                        parameters: Some(params),
                        ..Default::default()
                    });

                Some(output.add_block_auto(block))
            }

            "async_function_definition" => {
                let name = node
                    .child_by_field_name("name")
                    .map(|n| self.node_text(&n, source))
                    .unwrap_or_else(|| "method".to_string());

                let params = self.extract_parameters(node, source, scope);
                let mut method_scope = Scope::new();
                for p in &params {
                    method_scope.define(p);
                }

                let mut children = Vec::new();
                if let Some(body) = node.child_by_field_name("body") {
                    if let Some(body_id) = self.extract_node(
                        &body,
                        source,
                        file_path,
                        output,
                        &mut method_scope,
                        false,
                    ) {
                        children.push(body_id);
                    }
                }

                let block = Block::new(name, BlockType::AsyncMethod)
                    .with_produces(params.clone())
                    .with_children(children)
                    .with_location(self.node_location(node, file_path))
                    .with_metadata(BlockMetadata {
                        is_async: Some(true),
                        parameters: Some(params),
                        ..Default::default()
                    });

                Some(output.add_block_auto(block))
            }

            "decorated_definition" => {
                self.extract_decorated(node, source, file_path, output, scope, false)
            }

            "expression_statement" => {
                // Class-level assignments (class attributes)
                if let Some(expr) = node.child(0) {
                    if expr.kind() == "assignment" {
                        return self.extract_assignment(&expr, source, file_path, output, scope);
                    }
                }
                None
            }

            _ => None,
        }
    }

    fn extract_decorated(
        &self,
        node: &Node,
        source: &str,
        file_path: &str,
        output: &mut FlowMapOutput,
        scope: &mut Scope,
        is_top_level: bool,
    ) -> Option<BlockId> {
        let mut decorators = Vec::new();
        let mut definition_node = None;

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "decorator" {
                let decorator_name = self.get_decorator_name(&child, source);
                let dec_block = Block::new(format!("@{}", decorator_name), BlockType::Decorator)
                    .with_location(self.node_location(&child, file_path))
                    .with_code(self.node_text(&child, source));
                decorators.push(output.add_block_auto(dec_block));
            } else {
                definition_node = Some(child);
            }
        }

        if let Some(def) = definition_node {
            if let Some(def_id) =
                self.extract_node(&def, source, file_path, output, scope, is_top_level)
            {
                // Add decorators as children of the definition
                if let Some(block) = output.get_block_mut(&def_id) {
                    let mut new_children = decorators;
                    new_children.extend(block.children.clone());
                    block.children = new_children;
                }
                return Some(def_id);
            }
        }

        None
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

        // Then branch (consequence)
        if let Some(consequence) = node.child_by_field_name("consequence") {
            if let Some(then_id) =
                self.extract_node(&consequence, source, file_path, output, scope, false)
            {
                children.push(then_id);
            }
        }

        // Elif clauses and else
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "elif_clause" => {
                    let elif_cond = child.child_by_field_name("condition");
                    let elif_cond_text = elif_cond
                        .map(|c| self.node_text(&c, source))
                        .unwrap_or_default();

                    let mut elif_children = Vec::new();
                    if let Some(elif_body) = child.child_by_field_name("consequence") {
                        if let Some(body_id) =
                            self.extract_node(&elif_body, source, file_path, output, scope, false)
                        {
                            elif_children.push(body_id);
                        }
                    }

                    let elif_block = Block::new(
                        format!("elif {}", self.truncate(&elif_cond_text, 30)),
                        BlockType::ElseIf,
                    )
                    .with_uses(
                        elif_cond
                            .map(|c| self.extract_used_identifiers(&c, source, scope))
                            .unwrap_or_default(),
                    )
                    .with_children(elif_children)
                    .with_location(self.node_location(&child, file_path))
                    .with_metadata(BlockMetadata {
                        condition: Some(elif_cond_text),
                        ..Default::default()
                    });

                    children.push(output.add_block_auto(elif_block));
                }
                "else_clause" => {
                    if let Some(else_body) = child.child_by_field_name("body") {
                        if let Some(else_id) =
                            self.extract_node(&else_body, source, file_path, output, scope, false)
                        {
                            let else_block = Block::new("else", BlockType::Else)
                                .with_children(vec![else_id])
                                .with_location(self.node_location(&child, file_path));
                            children.push(output.add_block_auto(else_block));
                        }
                    }
                }
                _ => {}
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

        // Handle else clause (Python while...else)
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "else_clause" {
                if let Some(else_body) = child.child_by_field_name("body") {
                    if let Some(else_id) =
                        self.extract_node(&else_body, source, file_path, output, scope, false)
                    {
                        let else_block = Block::new("else", BlockType::Else)
                            .with_children(vec![else_id])
                            .with_location(self.node_location(&child, file_path));
                        children.push(output.add_block_auto(else_block));
                    }
                }
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

    fn extract_for_loop(
        &self,
        node: &Node,
        source: &str,
        file_path: &str,
        output: &mut FlowMapOutput,
        scope: &mut Scope,
    ) -> Option<BlockId> {
        let left = node.child_by_field_name("left");
        let right = node.child_by_field_name("right");

        let mut produces = Vec::new();
        if let Some(l) = left {
            produces = self.extract_binding_pattern(&l, source, scope);
            for p in &produces {
                scope.define(p);
            }
        }

        let uses = right
            .map(|r| self.extract_used_identifiers(&r, source, scope))
            .unwrap_or_default();

        let mut children = Vec::new();

        if let Some(body) = node.child_by_field_name("body") {
            if let Some(body_id) = self.extract_node(&body, source, file_path, output, scope, false)
            {
                children.push(body_id);
            }
        }

        // Handle else clause (Python for...else)
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "else_clause" {
                if let Some(else_body) = child.child_by_field_name("body") {
                    if let Some(else_id) =
                        self.extract_node(&else_body, source, file_path, output, scope, false)
                    {
                        let else_block = Block::new("else", BlockType::Else)
                            .with_children(vec![else_id])
                            .with_location(self.node_location(&child, file_path));
                        children.push(output.add_block_auto(else_block));
                    }
                }
            }
        }

        let right_text = right
            .map(|r| self.node_text(&r, source))
            .unwrap_or_default();
        let block = Block::new(
            format!("for in {}", self.truncate(&right_text, 30)),
            BlockType::ForOf,
        )
        .with_uses(uses)
        .with_produces(produces)
        .with_children(children)
        .with_location(self.node_location(node, file_path));

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

        // Except handlers
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "except_clause" | "except_group_clause" => {
                    let except_type = child
                        .child_by_field_name("type")
                        .or_else(|| child.child(1))
                        .map(|t| self.node_text(&t, source));

                    let except_name = child
                        .child_by_field_name("name")
                        .map(|n| self.node_text(&n, source));

                    let mut except_children = Vec::new();
                    // Find the block/body in the except clause
                    let mut except_cursor = child.walk();
                    for except_child in child.children(&mut except_cursor) {
                        if except_child.kind() == "block" {
                            if let Some(body_id) = self.extract_node(
                                &except_child,
                                source,
                                file_path,
                                output,
                                scope,
                                false,
                            ) {
                                except_children.push(body_id);
                            }
                        }
                    }

                    let label = match (&except_type, &except_name) {
                        (Some(t), Some(n)) => format!("except {} as {}", t, n),
                        (Some(t), None) => format!("except {}", t),
                        _ => "except".to_string(),
                    };

                    let except_block = Block::new(label, BlockType::Catch)
                        .with_produces(except_name.into_iter().collect())
                        .with_children(except_children)
                        .with_location(self.node_location(&child, file_path));

                    children.push(output.add_block_auto(except_block));
                }
                "else_clause" => {
                    if let Some(else_body) = child.child_by_field_name("body") {
                        if let Some(else_id) =
                            self.extract_node(&else_body, source, file_path, output, scope, false)
                        {
                            let else_block = Block::new("else", BlockType::Else)
                                .with_children(vec![else_id])
                                .with_location(self.node_location(&child, file_path));
                            children.push(output.add_block_auto(else_block));
                        }
                    }
                }
                "finally_clause" => {
                    if let Some(finally_body) = child.child_by_field_name("body") {
                        if let Some(finally_id) = self.extract_node(
                            &finally_body,
                            source,
                            file_path,
                            output,
                            scope,
                            false,
                        ) {
                            let finally_block = Block::new("finally", BlockType::Finally)
                                .with_children(vec![finally_id])
                                .with_location(self.node_location(&child, file_path));
                            children.push(output.add_block_auto(finally_block));
                        }
                    }
                }
                _ => {}
            }
        }

        let block = Block::new("try...except", BlockType::TryCatch)
            .with_children(children)
            .with_location(self.node_location(node, file_path));

        Some(output.add_block_auto(block))
    }

    fn extract_with_statement(
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

        // Extract with items
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "with_clause" {
                let mut clause_cursor = child.walk();
                for item in child.children(&mut clause_cursor) {
                    if item.kind() == "with_item" {
                        if let Some(value) = item.child_by_field_name("value") {
                            uses.extend(self.extract_used_identifiers(&value, source, scope));
                        }
                        if let Some(alias) = item.child_by_field_name("alias") {
                            let names = self.extract_binding_pattern(&alias, source, scope);
                            for n in &names {
                                scope.define(n);
                            }
                            produces.extend(names);
                        }
                    }
                }
            }
        }

        // Extract body
        if let Some(body) = node.child_by_field_name("body") {
            if let Some(body_id) = self.extract_node(&body, source, file_path, output, scope, false)
            {
                children.push(body_id);
            }
        }

        let block = Block::new("with", BlockType::Block)
            .with_uses(uses)
            .with_produces(produces)
            .with_children(children)
            .with_location(self.node_location(node, file_path));

        Some(output.add_block_auto(block))
    }

    fn extract_match_statement(
        &self,
        node: &Node,
        source: &str,
        file_path: &str,
        output: &mut FlowMapOutput,
        scope: &mut Scope,
    ) -> Option<BlockId> {
        let subject = node.child_by_field_name("subject");
        let subject_text = subject
            .map(|s| self.node_text(&s, source))
            .unwrap_or_default();
        let uses = subject
            .map(|s| self.extract_used_identifiers(&s, source, scope))
            .unwrap_or_default();

        let mut children = Vec::new();

        // Extract case clauses
        if let Some(body) = node.child_by_field_name("body") {
            let mut cursor = body.walk();
            for child in body.children(&mut cursor) {
                if child.kind() == "case_clause" {
                    let pattern = child
                        .child_by_field_name("pattern")
                        .map(|p| self.node_text(&p, source))
                        .unwrap_or_else(|| "_".to_string());

                    let mut case_children = Vec::new();
                    let mut case_cursor = child.walk();
                    for case_child in child.children(&mut case_cursor) {
                        if case_child.kind() == "block" {
                            if let Some(body_id) = self.extract_node(
                                &case_child,
                                source,
                                file_path,
                                output,
                                scope,
                                false,
                            ) {
                                case_children.push(body_id);
                            }
                        }
                    }

                    let case_block = Block::new(format!("case {}", pattern), BlockType::Case)
                        .with_children(case_children)
                        .with_location(self.node_location(&child, file_path));

                    children.push(output.add_block_auto(case_block));
                }
            }
        }

        let block = Block::new(
            format!("match {}", self.truncate(&subject_text, 30)),
            BlockType::Switch,
        )
        .with_uses(uses)
        .with_children(children)
        .with_location(self.node_location(node, file_path))
        .with_metadata(BlockMetadata {
            condition: Some(subject_text),
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

    fn extract_call(
        &self,
        node: &Node,
        source: &str,
        file_path: &str,
        output: &mut FlowMapOutput,
        scope: &mut Scope,
    ) -> Option<BlockId> {
        let function = node.child_by_field_name("function")?;
        let function_text = self.node_text(&function, source);

        let mut uses = self.extract_used_identifiers(&function, source, scope);
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

        let (block_type, name) = if function.kind() == "attribute" {
            let object = function
                .child_by_field_name("object")
                .map(|o| self.node_text(&o, source))
                .unwrap_or_default();
            let method = function
                .child_by_field_name("attribute")
                .map(|a| self.node_text(&a, source))
                .unwrap_or_default();
            (BlockType::MethodCall, format!("{}.{}", object, method))
        } else {
            (BlockType::Call, function_text)
        };

        let block = Block::new(name, block_type)
            .with_uses(uses)
            .with_children(children)
            .with_location(self.node_location(node, file_path))
            .with_code(self.node_text(node, source));

        Some(output.add_block_auto(block))
    }

    fn extract_await(
        &self,
        node: &Node,
        source: &str,
        file_path: &str,
        output: &mut FlowMapOutput,
        scope: &mut Scope,
    ) -> Option<BlockId> {
        let argument = node.child(1)?;
        let mut children = Vec::new();

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

    fn extract_raise(
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
            if child.kind() != "raise" {
                uses.extend(self.extract_used_identifiers(&child, source, scope));
                if let Some(child_id) =
                    self.extract_node(&child, source, file_path, output, scope, false)
                {
                    children.push(child_id);
                }
            }
        }

        let block = Block::new("raise", BlockType::Throw)
            .with_uses(uses)
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
                    "identifier" => {
                        let name = self.node_text(&child, source);
                        scope.define(&name);
                        params.push(name);
                    }
                    "default_parameter" | "typed_default_parameter" => {
                        if let Some(name) = child.child_by_field_name("name") {
                            let n = self.node_text(&name, source);
                            scope.define(&n);
                            params.push(n);
                        }
                    }
                    "typed_parameter" => {
                        if let Some(name) = child.child(0) {
                            if name.kind() == "identifier" {
                                let n = self.node_text(&name, source);
                                scope.define(&n);
                                params.push(n);
                            }
                        }
                    }
                    "list_splat_pattern" | "dictionary_splat_pattern" => {
                        if let Some(name) = child.child(1) {
                            let n = self.node_text(&name, source);
                            scope.define(&n);
                            params.push(if child.kind() == "list_splat_pattern" {
                                format!("*{}", n)
                            } else {
                                format!("**{}", n)
                            });
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
            "identifier" => {
                names.push(self.node_text(node, source));
            }
            "pattern_list" | "tuple_pattern" | "list_pattern" => {
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    names.extend(self.extract_binding_pattern(&child, source, scope));
                }
            }
            "subscript" | "attribute" => {
                // For x.y = z or x[i] = z, the "target" is the full expression
                names.push(self.node_text(node, source));
            }
            _ => {}
        }

        names
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
            "attribute" => {
                // Only extract the root object
                if let Some(object) = node.child_by_field_name("object") {
                    identifiers.extend(self.extract_used_identifiers(&object, source, scope));
                }
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
                "dotted_name" => {
                    // import foo.bar -> imports "foo"
                    if let Some(first) = child.child(0) {
                        names.push(self.node_text(&first, source));
                    }
                }
                "aliased_import" => {
                    if let Some(alias) = child.child_by_field_name("alias") {
                        names.push(self.node_text(&alias, source));
                    } else if let Some(name) = child.child_by_field_name("name") {
                        names.push(self.node_text(&name, source));
                    }
                }
                "import_from_statement" => {
                    // from x import y
                }
                "wildcard_import" => {
                    names.push("*".to_string());
                }
                _ => {}
            }
        }

        // For import_from_statement, extract the imported names
        if node.kind() == "import_from_statement" {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                match child.kind() {
                    "dotted_name" => {
                        names.push(self.node_text(&child, source));
                    }
                    "aliased_import" => {
                        if let Some(alias) = child.child_by_field_name("alias") {
                            names.push(self.node_text(&alias, source));
                        } else if let Some(name) = child.child_by_field_name("name") {
                            names.push(self.node_text(&name, source));
                        }
                    }
                    "wildcard_import" => {
                        names.push("*".to_string());
                    }
                    _ => {}
                }
            }
        }

        names
    }

    fn get_decorator_name(&self, node: &Node, source: &str) -> String {
        if let Some(expr) = node.child(1) {
            match expr.kind() {
                "identifier" => self.node_text(&expr, source),
                "call" => {
                    if let Some(func) = expr.child_by_field_name("function") {
                        self.node_text(&func, source)
                    } else {
                        "decorator".to_string()
                    }
                }
                "attribute" => self.node_text(&expr, source),
                _ => self.node_text(&expr, source),
            }
        } else {
            "decorator".to_string()
        }
    }

    fn is_complex_expression(&self, node: &Node) -> bool {
        matches!(
            node.kind(),
            "call"
                | "await"
                | "lambda"
                | "dictionary"
                | "list"
                | "set"
                | "list_comprehension"
                | "dictionary_comprehension"
                | "set_comprehension"
                | "generator_expression"
        )
    }

    fn is_builtin(&self, name: &str) -> bool {
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
                | "round"
                | "sorted"
                | "reversed"
                | "list"
                | "dict"
                | "set"
                | "tuple"
                | "str"
                | "int"
                | "float"
                | "bool"
                | "type"
                | "isinstance"
                | "issubclass"
                | "hasattr"
                | "getattr"
                | "setattr"
                | "delattr"
                | "open"
                | "input"
                | "id"
                | "hash"
                | "repr"
                | "format"
                | "iter"
                | "next"
                | "super"
                | "object"
                | "staticmethod"
                | "classmethod"
                | "property"
                | "True"
                | "False"
                | "None"
                | "self"
                | "cls"
                | "__name__"
                | "__file__"
                | "__doc__"
                | "__all__"
                | "Exception"
                | "BaseException"
                | "ValueError"
                | "TypeError"
                | "KeyError"
                | "IndexError"
                | "AttributeError"
                | "RuntimeError"
                | "StopIteration"
                | "NotImplementedError"
                | "FileNotFoundError"
                | "IOError"
                | "OSError"
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

impl Default for PythonBlockExtractor {
    fn default() -> Self {
        Self::new().expect("Failed to create PythonBlockExtractor")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_function() {
        let mut extractor = PythonBlockExtractor::new().unwrap();
        let source = r#"
def add(a, b):
    return a + b
"#;

        let output = extractor.parse_file(source, "test.py").unwrap();
        assert!(output.block_count() > 0);

        let json = serde_json::to_string_pretty(&output).unwrap();
        assert!(json.contains("add"));
        assert!(json.contains("function"));
    }

    #[test]
    fn test_class_extraction() {
        let mut extractor = PythonBlockExtractor::new().unwrap();
        let source = r#"
class UserService:
    def __init__(self, db):
        self.db = db

    async def get_user(self, user_id: str):
        user = await self.db.find_by_id(user_id)
        return user
"#;

        let output = extractor.parse_file(source, "test.py").unwrap();
        let json = serde_json::to_string_pretty(&output).unwrap();

        assert!(json.contains("UserService"));
        assert!(json.contains("get_user"));
        assert!(json.contains("__init__"));
    }

    #[test]
    fn test_decorator_extraction() {
        let mut extractor = PythonBlockExtractor::new().unwrap();
        let source = r#"
@app.route("/users")
@login_required
def get_users():
    return users
"#;

        let output = extractor.parse_file(source, "test.py").unwrap();
        let json = serde_json::to_string_pretty(&output).unwrap();

        assert!(json.contains("@app.route"));
        assert!(json.contains("@login_required"));
    }
}
