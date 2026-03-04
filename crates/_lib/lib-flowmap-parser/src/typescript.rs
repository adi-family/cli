use lib_flowmap_core::*;
use tree_sitter::{Node, Parser};

pub struct TypeScriptExtractor {
    parser: Parser,
    next_node_id: u64,
    next_edge_id: u64,
    next_flow_id: u64,
}

impl TypeScriptExtractor {
    pub fn new() -> crate::Result<Self> {
        let mut parser = Parser::new();
        let language = tree_sitter_typescript::LANGUAGE_TYPESCRIPT;
        parser
            .set_language(&language.into())
            .map_err(|e| crate::ParseError::TreeSitter(e.to_string()))?;

        Ok(Self {
            parser,
            next_node_id: 1,
            next_edge_id: 1,
            next_flow_id: 1,
        })
    }

    fn alloc_node_id(&mut self) -> u64 {
        let id = self.next_node_id;
        self.next_node_id += 1;
        id
    }

    fn alloc_edge_id(&mut self) -> u64 {
        let id = self.next_edge_id;
        self.next_edge_id += 1;
        id
    }

    fn alloc_flow_id(&mut self) -> u64 {
        let id = self.next_flow_id;
        self.next_flow_id += 1;
        id
    }

    pub fn parse_file(&mut self, source: &str, file_path: &str) -> crate::Result<Vec<FlowGraph>> {
        let tree = self
            .parser
            .parse(source, None)
            .ok_or_else(|| crate::ParseError::ParseFailed {
                path: file_path.to_string(),
            })?;

        let mut flows = Vec::new();
        let root = tree.root_node();

        self.extract_flows(&root, source, file_path, &mut flows);

        Ok(flows)
    }

    fn extract_flows(
        &mut self,
        node: &Node,
        source: &str,
        file_path: &str,
        flows: &mut Vec<FlowGraph>,
    ) {
        // Look for exported functions, HTTP handlers, etc.
        match node.kind() {
            "export_statement" => {
                if let Some(decl) = node.child_by_field_name("declaration") {
                    if let Some(flow) = self.extract_exported_function(&decl, source, file_path) {
                        flows.push(flow);
                    }
                }
            }
            "function_declaration" | "arrow_function" | "function_expression" => {
                // Check if it's a route handler or exported
                if let Some(flow) = self.try_extract_handler(node, source, file_path) {
                    flows.push(flow);
                }
            }
            "call_expression" => {
                // Check for app.get(), router.post(), etc.
                if let Some(flow) = self.try_extract_route_handler(node, source, file_path) {
                    flows.push(flow);
                }
            }
            _ => {}
        }

        // Recurse into children
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.extract_flows(&child, source, file_path, flows);
        }
    }

    fn extract_exported_function(
        &mut self,
        node: &Node,
        source: &str,
        file_path: &str,
    ) -> Option<FlowGraph> {
        if node.kind() != "function_declaration" && node.kind() != "lexical_declaration" {
            return None;
        }

        let name = self.get_function_name(node, source)?;
        let flow_id = self.alloc_flow_id();

        let mut flow = FlowGraph::new(
            flow_id,
            &name,
            file_path,
            EntryPointKind::ExportedFunction,
        );

        // Create entry node
        let entry_id = self.alloc_node_id();
        let entry_node = FlowNode::new(
            entry_id,
            NodeKind::ExportedFunction,
            &format!("Function: {}", name),
            &format!("export function {}()", name),
            self.node_location(node, file_path),
        );
        flow.add_node(entry_node);

        // Extract body nodes
        if let Some(body) = self.find_function_body(node) {
            self.extract_body_nodes(&body, source, file_path, &mut flow, NodeId(entry_id), PinId(1));
        }

        Some(flow)
    }

    fn try_extract_handler(
        &mut self,
        _node: &Node,
        _source: &str,
        _file_path: &str,
    ) -> Option<FlowGraph> {
        // Placeholder for standalone function detection
        None
    }

    fn try_extract_route_handler(
        &mut self,
        node: &Node,
        source: &str,
        file_path: &str,
    ) -> Option<FlowGraph> {
        // Look for patterns like: app.get('/path', handler)
        let callee = node.child_by_field_name("function")?;

        if callee.kind() != "member_expression" {
            return None;
        }

        let property = callee.child_by_field_name("property")?;
        let method = self.node_text(&property, source);

        let http_methods = ["get", "post", "put", "delete", "patch", "all", "use"];
        if !http_methods.contains(&method.to_lowercase().as_str()) {
            return None;
        }

        // Get the path argument
        let args = node.child_by_field_name("arguments")?;
        let mut args_cursor = args.walk();
        let args_children: Vec<_> = args.children(&mut args_cursor).collect();

        let path = args_children.first().map(|n| {
            let text = self.node_text(n, source);
            text.trim_matches(|c| c == '"' || c == '\'' || c == '`')
                .to_string()
        })?;

        // Find the handler function
        let handler = args_children.get(1)?;

        let flow_id = self.alloc_flow_id();
        let name = format!("{} {}", method.to_uppercase(), path);

        let mut flow = FlowGraph::new(
            flow_id,
            &name,
            file_path,
            EntryPointKind::HttpHandler {
                method: method.to_uppercase(),
                path: path.clone(),
            },
        );

        // Create entry node
        let entry_id = self.alloc_node_id();
        let entry_node = FlowNode::new(
            entry_id,
            NodeKind::HttpHandler {
                method: method.to_uppercase(),
                path: path.clone(),
            },
            &name,
            &format!("app.{}('{}')", method, path),
            self.node_location(node, file_path),
        );
        flow.add_node(entry_node);

        // Extract handler body
        if let Some(body) = self.find_function_body(handler) {
            self.extract_body_nodes(&body, source, file_path, &mut flow, NodeId(entry_id), PinId(1));
        }

        Some(flow)
    }

    fn extract_body_nodes(
        &mut self,
        body: &Node,
        source: &str,
        file_path: &str,
        flow: &mut FlowGraph,
        prev_node: NodeId,
        prev_pin: PinId,
    ) -> Option<(NodeId, PinId)> {
        let mut current_node = prev_node;
        let mut current_pin = prev_pin;

        let mut cursor = body.walk();
        for child in body.children(&mut cursor) {
            if let Some((new_node, new_pin)) =
                self.extract_statement(&child, source, file_path, flow, current_node, current_pin)
            {
                current_node = new_node;
                current_pin = new_pin;
            }
        }

        Some((current_node, current_pin))
    }

    fn extract_statement(
        &mut self,
        node: &Node,
        source: &str,
        file_path: &str,
        flow: &mut FlowGraph,
        prev_node: NodeId,
        prev_pin: PinId,
    ) -> Option<(NodeId, PinId)> {
        match node.kind() {
            "if_statement" => self.extract_if_statement(node, source, file_path, flow, prev_node, prev_pin),
            "try_statement" => self.extract_try_statement(node, source, file_path, flow, prev_node, prev_pin),
            "for_statement" | "for_in_statement" | "while_statement" => {
                self.extract_loop_statement(node, source, file_path, flow, prev_node, prev_pin)
            }
            "return_statement" => self.extract_return_statement(node, source, file_path, flow, prev_node, prev_pin),
            "throw_statement" => self.extract_throw_statement(node, source, file_path, flow, prev_node, prev_pin),
            "expression_statement" => {
                if let Some(expr) = node.child(0) {
                    self.extract_expression(&expr, source, file_path, flow, prev_node, prev_pin)
                } else {
                    None
                }
            }
            "lexical_declaration" | "variable_declaration" => {
                self.extract_declaration(node, source, file_path, flow, prev_node, prev_pin)
            }
            _ => None,
        }
    }

    fn extract_declaration(
        &mut self,
        node: &Node,
        source: &str,
        file_path: &str,
        flow: &mut FlowGraph,
        prev_node: NodeId,
        prev_pin: PinId,
    ) -> Option<(NodeId, PinId)> {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "variable_declarator" {
                if let Some(value) = child.child_by_field_name("value") {
                    return self.extract_expression(&value, source, file_path, flow, prev_node, prev_pin);
                }
            }
        }
        None
    }

    fn extract_expression(
        &mut self,
        node: &Node,
        source: &str,
        file_path: &str,
        flow: &mut FlowGraph,
        prev_node: NodeId,
        prev_pin: PinId,
    ) -> Option<(NodeId, PinId)> {
        match node.kind() {
            "await_expression" => self.extract_await(node, source, file_path, flow, prev_node, prev_pin),
            "call_expression" => self.extract_call(node, source, file_path, flow, prev_node, prev_pin),
            _ => None,
        }
    }

    fn extract_if_statement(
        &mut self,
        node: &Node,
        source: &str,
        file_path: &str,
        flow: &mut FlowGraph,
        prev_node: NodeId,
        prev_pin: PinId,
    ) -> Option<(NodeId, PinId)> {
        let condition = node.child_by_field_name("condition")?;
        let condition_text = self.node_text(&condition, source);

        let cond_node_id = self.alloc_node_id();
        let if_node = FlowNode::new(
            cond_node_id,
            NodeKind::Condition {
                expression: condition_text.to_string(),
            },
            &format!("If: {}", self.truncate(&condition_text, 30)),
            &format!("if ({})", condition_text),
            self.node_location(node, file_path),
        );
        flow.add_node(if_node);

        // Connect from previous
        flow.add_edge(FlowEdge {
            id: EdgeId(self.alloc_edge_id()),
            from_node: prev_node,
            from_pin: prev_pin,
            to_node: NodeId(cond_node_id),
            to_pin: PinId(1),
            kind: EdgeKind::Execution,
            label: None,
        });

        // Process true branch (consequence)
        let true_end = if let Some(consequence) = node.child_by_field_name("consequence") {
            self.extract_body_nodes(
                &consequence,
                source,
                file_path,
                flow,
                NodeId(cond_node_id),
                PinId(2), // true pin
            )
        } else {
            // No body = flow continues directly from condition's true pin
            Some((NodeId(cond_node_id), PinId(2)))
        };

        // Process false branch (alternative / else)
        let false_end = if let Some(alternative) = node.child_by_field_name("alternative") {
            // Handle else-if chain: alternative might be another if_statement
            if alternative.kind() == "if_statement" {
                self.extract_if_statement(
                    &alternative,
                    source,
                    file_path,
                    flow,
                    NodeId(cond_node_id),
                    PinId(3), // false pin
                )
            } else {
                self.extract_body_nodes(
                    &alternative,
                    source,
                    file_path,
                    flow,
                    NodeId(cond_node_id),
                    PinId(3), // false pin
                )
            }
        } else {
            // No else = flow continues directly from condition's false pin
            Some((NodeId(cond_node_id), PinId(3)))
        };

        // Create merge node if both branches have non-terminal endpoints
        match (true_end, false_end) {
            (Some((true_node, true_pin)), Some((false_node, false_pin))) => {
                // Both branches continue - create merge point
                let merge_id = self.alloc_node_id();
                let merge_node = FlowNode::new(
                    merge_id,
                    NodeKind::Merge,
                    "Merge",
                    "// branches merge",
                    self.node_location(node, file_path),
                );
                flow.add_node(merge_node);

                // Connect true branch to merge (pin 1)
                flow.add_edge(FlowEdge {
                    id: EdgeId(self.alloc_edge_id()),
                    from_node: true_node,
                    from_pin: true_pin,
                    to_node: NodeId(merge_id),
                    to_pin: PinId(1),
                    kind: EdgeKind::Execution,
                    label: None,
                });

                // Connect false branch to merge (pin 2)
                flow.add_edge(FlowEdge {
                    id: EdgeId(self.alloc_edge_id()),
                    from_node: false_node,
                    from_pin: false_pin,
                    to_node: NodeId(merge_id),
                    to_pin: PinId(2),
                    kind: EdgeKind::Execution,
                    label: None,
                });

                // Return merge node's output for subsequent statements
                Some((NodeId(merge_id), PinId(3)))
            }
            (Some(end), None) => Some(end), // Only true branch continues
            (None, Some(end)) => Some(end), // Only false branch continues
            (None, None) => None,           // Both branches terminate
        }
    }

    fn extract_try_statement(
        &mut self,
        node: &Node,
        source: &str,
        file_path: &str,
        flow: &mut FlowGraph,
        prev_node: NodeId,
        prev_pin: PinId,
    ) -> Option<(NodeId, PinId)> {
        let try_node_id = self.alloc_node_id();
        let try_node = FlowNode::new(
            try_node_id,
            NodeKind::TryCatch,
            "Try/Catch",
            "try { ... } catch { ... }",
            self.node_location(node, file_path),
        );
        flow.add_node(try_node);

        // Connect from previous
        flow.add_edge(FlowEdge {
            id: EdgeId(self.alloc_edge_id()),
            from_node: prev_node,
            from_pin: prev_pin,
            to_node: NodeId(try_node_id),
            to_pin: PinId(1),
            kind: EdgeKind::Execution,
            label: None,
        });

        // Process try body (success path)
        let try_end = if let Some(body) = node.child_by_field_name("body") {
            self.extract_body_nodes(&body, source, file_path, flow, NodeId(try_node_id), PinId(2))
        } else {
            Some((NodeId(try_node_id), PinId(2)))
        };

        // Process catch (error path)
        let catch_end = if let Some(handler) = node.child_by_field_name("handler") {
            if let Some(catch_body) = handler.child_by_field_name("body") {
                self.extract_body_nodes(
                    &catch_body,
                    source,
                    file_path,
                    flow,
                    NodeId(try_node_id),
                    PinId(3), // error pin
                )
            } else {
                Some((NodeId(try_node_id), PinId(3)))
            }
        } else {
            None // No catch clause
        };

        // Create merge if both paths continue
        match (try_end, catch_end) {
            (Some((try_node, try_pin)), Some((catch_node, catch_pin))) => {
                let merge_id = self.alloc_node_id();
                let merge_node = FlowNode::new(
                    merge_id,
                    NodeKind::Merge,
                    "Merge",
                    "// try/catch merge",
                    self.node_location(node, file_path),
                );
                flow.add_node(merge_node);

                // Connect try end to merge
                flow.add_edge(FlowEdge {
                    id: EdgeId(self.alloc_edge_id()),
                    from_node: try_node,
                    from_pin: try_pin,
                    to_node: NodeId(merge_id),
                    to_pin: PinId(1),
                    kind: EdgeKind::Execution,
                    label: None,
                });

                // Connect catch end to merge
                flow.add_edge(FlowEdge {
                    id: EdgeId(self.alloc_edge_id()),
                    from_node: catch_node,
                    from_pin: catch_pin,
                    to_node: NodeId(merge_id),
                    to_pin: PinId(2),
                    kind: EdgeKind::Execution,
                    label: None,
                });

                Some((NodeId(merge_id), PinId(3)))
            }
            (Some(end), None) => Some(end),
            (None, Some(end)) => Some(end),
            (None, None) => None,
        }
    }

    fn extract_loop_statement(
        &mut self,
        node: &Node,
        source: &str,
        file_path: &str,
        flow: &mut FlowGraph,
        prev_node: NodeId,
        prev_pin: PinId,
    ) -> Option<(NodeId, PinId)> {
        let loop_kind = match node.kind() {
            "for_statement" => LoopKind::For,
            "for_in_statement" => LoopKind::ForIn,
            "while_statement" => LoopKind::While,
            _ => LoopKind::For,
        };

        let node_id = self.alloc_node_id();
        let label = match loop_kind {
            LoopKind::For => "For Loop",
            LoopKind::ForIn | LoopKind::ForOf => "For Each",
            LoopKind::While | LoopKind::DoWhile => "While Loop",
        };

        let loop_node = FlowNode::new(
            node_id,
            NodeKind::Loop { kind: loop_kind },
            label,
            &self.node_text(node, source)[..50.min(self.node_text(node, source).len())],
            self.node_location(node, file_path),
        );
        flow.add_node(loop_node);

        // Connect from previous
        flow.add_edge(FlowEdge {
            id: EdgeId(self.alloc_edge_id()),
            from_node: prev_node,
            from_pin: prev_pin,
            to_node: NodeId(node_id),
            to_pin: PinId(1),
            kind: EdgeKind::Execution,
            label: None,
        });

        // Process body
        if let Some(body) = node.child_by_field_name("body") {
            self.extract_body_nodes(&body, source, file_path, flow, NodeId(node_id), PinId(2));
        }

        Some((NodeId(node_id), PinId(3))) // done pin
    }

    fn extract_return_statement(
        &mut self,
        node: &Node,
        source: &str,
        file_path: &str,
        flow: &mut FlowGraph,
        prev_node: NodeId,
        prev_pin: PinId,
    ) -> Option<(NodeId, PinId)> {
        let has_value = node.child_count() > 1;

        // First, extract expressions inside the return (await, calls, etc.)
        let (current_node, current_pin) = if has_value {
            // Get the return value expression (skip 'return' keyword)
            if let Some(expr) = node.child(1) {
                // Recursively extract the expression chain
                if let Some((n, p)) = self.extract_expression_deep(&expr, source, file_path, flow, prev_node, prev_pin) {
                    (n, p)
                } else {
                    (prev_node, prev_pin)
                }
            } else {
                (prev_node, prev_pin)
            }
        } else {
            (prev_node, prev_pin)
        };

        // Then add the return node at the end
        let node_id = self.alloc_node_id();
        let return_node = FlowNode::new(
            node_id,
            NodeKind::Return { has_value },
            if has_value { "Return" } else { "Return void" },
            "return",
            self.node_location(node, file_path),
        );
        flow.add_node(return_node);

        flow.add_edge(FlowEdge {
            id: EdgeId(self.alloc_edge_id()),
            from_node: current_node,
            from_pin: current_pin,
            to_node: NodeId(node_id),
            to_pin: PinId(1),
            kind: EdgeKind::Execution,
            label: None,
        });

        None // Return is terminal
    }

    /// Recursively extract expressions, handling chains like (await foo()).map()
    fn extract_expression_deep(
        &mut self,
        node: &Node,
        source: &str,
        file_path: &str,
        flow: &mut FlowGraph,
        prev_node: NodeId,
        prev_pin: PinId,
    ) -> Option<(NodeId, PinId)> {
        match node.kind() {
            "await_expression" => {
                self.extract_await(node, source, file_path, flow, prev_node, prev_pin)
            }
            "call_expression" => {
                let callee = node.child_by_field_name("function")?;
                let mut current_node = prev_node;
                let mut current_pin = prev_pin;

                // First, extract any extractable expressions from arguments
                if let Some(args) = node.child_by_field_name("arguments") {
                    let mut cursor = args.walk();
                    for arg in args.children(&mut cursor) {
                        if self.is_extractable_expression(&arg) {
                            if let Some((n, p)) = self.extract_expression_deep(&arg, source, file_path, flow, current_node, current_pin) {
                                current_node = n;
                                current_pin = p;
                            }
                        }
                    }
                }

                if callee.kind() == "member_expression" {
                    let object = callee.child_by_field_name("object")?;

                    // Extract the object expression (for chained calls like (await x).map())
                    if self.is_extractable_expression(&object) {
                        if let Some((n, p)) = self.extract_expression_deep(&object, source, file_path, flow, current_node, current_pin) {
                            current_node = n;
                            current_pin = p;
                        }
                    }

                    // Then extract this call
                    self.extract_call(node, source, file_path, flow, current_node, current_pin)
                } else {
                    self.extract_call(node, source, file_path, flow, current_node, current_pin)
                }
            }
            "parenthesized_expression" => {
                // Unwrap parentheses and extract inner expression
                if let Some(inner) = node.child(1) {
                    self.extract_expression_deep(&inner, source, file_path, flow, prev_node, prev_pin)
                } else {
                    None
                }
            }
            "member_expression" => {
                // Property access like obj.prop - extract the object if it's complex
                let object = node.child_by_field_name("object")?;
                if self.is_extractable_expression(&object) {
                    self.extract_expression_deep(&object, source, file_path, flow, prev_node, prev_pin)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    fn is_extractable_expression(&self, node: &Node) -> bool {
        matches!(
            node.kind(),
            "await_expression" | "call_expression" | "parenthesized_expression"
        )
    }

    fn extract_throw_statement(
        &mut self,
        node: &Node,
        source: &str,
        file_path: &str,
        flow: &mut FlowGraph,
        prev_node: NodeId,
        prev_pin: PinId,
    ) -> Option<(NodeId, PinId)> {
        let node_id = self.alloc_node_id();
        let code = self.node_text(node, source);

        let throw_node = FlowNode::new(
            node_id,
            NodeKind::Throw,
            "Throw Error",
            &code,
            self.node_location(node, file_path),
        );
        flow.add_node(throw_node);

        flow.add_edge(FlowEdge {
            id: EdgeId(self.alloc_edge_id()),
            from_node: prev_node,
            from_pin: prev_pin,
            to_node: NodeId(node_id),
            to_pin: PinId(1),
            kind: EdgeKind::Execution,
            label: None,
        });

        None // Throw is terminal
    }

    fn extract_await(
        &mut self,
        node: &Node,
        source: &str,
        file_path: &str,
        flow: &mut FlowGraph,
        prev_node: NodeId,
        prev_pin: PinId,
    ) -> Option<(NodeId, PinId)> {
        let argument = node.child_by_field_name("argument").or_else(|| node.child(1))?;
        let code = self.node_text(&argument, source);
        let label = self.extract_call_label(&argument, source).unwrap_or_else(|| code.clone());

        let node_id = self.alloc_node_id();
        let await_node = FlowNode::new(
            node_id,
            NodeKind::Await,
            &format!("Await: {}", self.truncate(&label, 25)),
            &format!("await {}", code),
            self.node_location(node, file_path),
        );
        flow.add_node(await_node);

        flow.add_edge(FlowEdge {
            id: EdgeId(self.alloc_edge_id()),
            from_node: prev_node,
            from_pin: prev_pin,
            to_node: NodeId(node_id),
            to_pin: PinId(1),
            kind: EdgeKind::Execution,
            label: None,
        });

        Some((NodeId(node_id), PinId(2)))
    }

    fn extract_call(
        &mut self,
        node: &Node,
        source: &str,
        file_path: &str,
        flow: &mut FlowGraph,
        prev_node: NodeId,
        prev_pin: PinId,
    ) -> Option<(NodeId, PinId)> {
        let callee = node.child_by_field_name("function")?;
        let code = self.node_text(node, source);

        let (kind, label) = if callee.kind() == "member_expression" {
            let object = callee.child_by_field_name("object").map(|n| self.node_text(&n, source))?;
            let property = callee.child_by_field_name("property").map(|n| self.node_text(&n, source))?;

            (
                NodeKind::MethodCall {
                    object: object.clone(),
                    method: property.clone(),
                    is_async: false,
                },
                format!("{}.{}", object, property),
            )
        } else {
            let name = self.node_text(&callee, source);
            (
                NodeKind::FunctionCall {
                    name: name.clone(),
                    is_async: false,
                },
                name,
            )
        };

        let node_id = self.alloc_node_id();
        let call_node = FlowNode::new(
            node_id,
            kind,
            &format!("Call: {}", self.truncate(&label, 25)),
            &self.truncate(&code, 50),
            self.node_location(node, file_path),
        );
        flow.add_node(call_node);

        flow.add_edge(FlowEdge {
            id: EdgeId(self.alloc_edge_id()),
            from_node: prev_node,
            from_pin: prev_pin,
            to_node: NodeId(node_id),
            to_pin: PinId(1),
            kind: EdgeKind::Execution,
            label: None,
        });

        Some((NodeId(node_id), PinId(2)))
    }

    // Helper functions

    fn get_function_name(&self, node: &Node, source: &str) -> Option<String> {
        if node.kind() == "function_declaration" {
            node.child_by_field_name("name")
                .map(|n| self.node_text(&n, source))
        } else if node.kind() == "lexical_declaration" {
            // const foo = ...
            let declarator = node.child_by_field_name("declarator")?;
            declarator
                .child_by_field_name("name")
                .map(|n| self.node_text(&n, source))
        } else {
            None
        }
    }

    fn find_function_body<'a>(&self, node: &'a Node<'a>) -> Option<Node<'a>> {
        match node.kind() {
            "function_declaration" | "function_expression" | "method_definition" => {
                node.child_by_field_name("body")
            }
            "arrow_function" => node.child_by_field_name("body"),
            "lexical_declaration" => {
                // const foo = () => {} - get body from the arrow function inside
                let declarator = node.child_by_field_name("declarator")?;
                let value = declarator.child_by_field_name("value")?;
                // Directly get body from arrow function or function expression
                match value.kind() {
                    "arrow_function" | "function_expression" => value.child_by_field_name("body"),
                    _ => None,
                }
            }
            "statement_block" => Some(*node),
            _ => None,
        }
    }

    fn extract_call_label(&self, node: &Node, source: &str) -> Option<String> {
        if node.kind() == "call_expression" {
            let callee = node.child_by_field_name("function")?;
            if callee.kind() == "member_expression" {
                let prop = callee.child_by_field_name("property")?;
                return Some(self.node_text(&prop, source));
            }
            return Some(self.node_text(&callee, source));
        }
        None
    }

    fn node_text(&self, node: &Node, source: &str) -> String {
        source[node.byte_range()].to_string()
    }

    fn node_location(&self, node: &Node, file_path: &str) -> SourceLocation {
        let start = node.start_position();
        let end = node.end_position();
        SourceLocation {
            file_path: file_path.to_string(),
            start_line: start.row as u32 + 1,
            end_line: end.row as u32 + 1,
            start_col: start.column as u32,
            end_col: end.column as u32,
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

impl Default for TypeScriptExtractor {
    fn default() -> Self {
        Self::new().expect("Failed to create TypeScript extractor")
    }
}
