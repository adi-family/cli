use lib_flowmap_core::*;
use std::collections::HashMap;
use tree_sitter::{Node, Parser};

/// Tracks a variable binding to its source node
#[derive(Debug, Clone)]
struct VariableBinding {
    /// Node that produces this variable's value
    source_node: NodeId,
    /// Output pin that produces the value (usually "result" pin)
    source_pin: PinId,
}

/// Scope for tracking variable bindings within a flow
#[derive(Debug, Default)]
struct VariableScope {
    bindings: HashMap<String, VariableBinding>,
}

impl VariableScope {
    fn bind(&mut self, name: &str, source_node: NodeId, source_pin: PinId) {
        self.bindings.insert(
            name.to_string(),
            VariableBinding { source_node, source_pin },
        );
    }

    fn get(&self, name: &str) -> Option<&VariableBinding> {
        self.bindings.get(name)
    }
}

/// NestJS-aware extractor that builds a symbol index and resolves cross-file flows
pub struct NestJsExtractor {
    parser: Parser,
    next_node_id: u64,
    next_edge_id: u64,
    next_flow_id: u64,
    symbol_index: SymbolIndex,
}

impl NestJsExtractor {
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
            symbol_index: SymbolIndex::new(),
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

    /// Pass 1: Index a single file for symbols (classes, methods, imports)
    pub fn index_file(&mut self, source: &str, file_path: &str) -> crate::Result<()> {
        let tree = self
            .parser
            .parse(source, None)
            .ok_or_else(|| crate::ParseError::ParseFailed {
                path: file_path.to_string(),
            })?;

        let root = tree.root_node();

        // Extract imports
        self.extract_imports(&root, source, file_path);

        // Extract classes
        self.extract_classes(&root, source, file_path);

        Ok(())
    }

    fn extract_imports(&mut self, node: &Node, source: &str, file_path: &str) {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "import_statement" {
                self.extract_import(&child, source, file_path);
            }
        }
    }

    fn extract_import(&mut self, node: &Node, source: &str, file_path: &str) {
        // Get the source path
        let source_path = node
            .child_by_field_name("source")
            .map(|n| self.node_text(&n, source).trim_matches(|c| c == '"' || c == '\'').to_string());

        let Some(source_path) = source_path else {
            return;
        };

        // Get import clause
        let Some(clause) = node.children(&mut node.walk()).find(|c| c.kind() == "import_clause") else {
            return;
        };

        // Handle named imports: import { Foo, Bar as Baz } from '...'
        let mut cursor = clause.walk();
        for child in clause.children(&mut cursor) {
            if child.kind() == "named_imports" {
                let mut spec_cursor = child.walk();
                for spec in child.children(&mut spec_cursor) {
                    if spec.kind() == "import_specifier" {
                        let name = spec.child_by_field_name("name").map(|n| self.node_text(&n, source));
                        let alias = spec.child_by_field_name("alias").map(|n| self.node_text(&n, source));

                        if let Some(name) = name {
                            let import_name = alias.clone().unwrap_or_else(|| name.clone());
                            self.symbol_index.add_import(
                                file_path,
                                &import_name,
                                ImportInfo {
                                    source: source_path.clone(),
                                    original_name: if alias.is_some() { Some(name.clone()) } else { None },
                                },
                            );
                        }
                    }
                }
            }
        }
    }

    fn extract_classes(&mut self, node: &Node, source: &str, file_path: &str) {
        match node.kind() {
            "export_statement" => {
                // Process export_statement (includes decorators)
                if let Some(class) = self.extract_class(node, source, file_path) {
                    self.symbol_index.add_class(class);
                }
                // DON'T recurse into children - we already extracted the class
                return;
            }
            "class_declaration" => {
                // Only process bare class declarations (not wrapped in export)
                if let Some(class) = self.extract_class(node, source, file_path) {
                    self.symbol_index.add_class(class);
                }
            }
            _ => {}
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.extract_classes(&child, source, file_path);
        }
    }

    fn extract_class(&self, node: &Node, source: &str, file_path: &str) -> Option<ClassInfo> {
        // Handle export statement wrapping a class
        let class_node = if node.kind() == "export_statement" {
            node.child_by_field_name("declaration")
                .filter(|n| n.kind() == "class_declaration")?
        } else {
            *node
        };

        if class_node.kind() != "class_declaration" {
            return None;
        }

        let name = class_node
            .child_by_field_name("name")
            .map(|n| self.node_text(&n, source))?;

        // Get decorators - check both the export statement AND the class declaration
        let mut decorators = self.extract_decorators(node, source);
        if node.kind() == "export_statement" {
            // Also check decorators directly on the class inside export
            decorators.extend(self.extract_decorators(&class_node, source));
        }

        // Debug: log controllers we find
        if decorators.iter().any(|d| d.name == "Controller") {
            tracing::debug!(
                "Found Controller class: {} with {} decorators in {}",
                name,
                decorators.len(),
                file_path
            );
        }

        // Determine class kind from decorators
        let kind = self.classify_class(&decorators);

        // Extract methods and injections from class body
        let body = class_node.child_by_field_name("body")?;
        let (methods, injections) = self.extract_class_members(&body, source, file_path, &name);

        Some(ClassInfo {
            id: SymbolId::class(file_path, &name),
            name,
            file_path: file_path.to_string(),
            kind,
            decorators,
            methods,
            injections,
            start_line: class_node.start_position().row as u32 + 1,
            end_line: class_node.end_position().row as u32 + 1,
        })
    }

    fn extract_decorators(&self, node: &Node, source: &str) -> Vec<Decorator> {
        let mut decorators = vec![];

        // Debug: print children of this node to understand AST structure
        let children: Vec<_> = {
            let mut cursor = node.walk();
            node.children(&mut cursor).map(|c| c.kind().to_string()).collect()
        };

        // Only log for class-like nodes
        if node.kind() == "export_statement" || node.kind() == "class_declaration" {
            tracing::trace!(
                "Node {} has children: {:?}",
                node.kind(),
                children
            );
        }

        // Look for decorator nodes in children
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "decorator" => {
                    if let Some(dec) = self.parse_decorator(&child, source) {
                        decorators.push(dec);
                    }
                }
                // Some ASTs wrap decorators in a list
                "decorators" => {
                    let mut dec_cursor = child.walk();
                    for dec_child in child.children(&mut dec_cursor) {
                        if dec_child.kind() == "decorator" {
                            if let Some(dec) = self.parse_decorator(&dec_child, source) {
                                decorators.push(dec);
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        decorators
    }

    fn parse_decorator(&self, node: &Node, source: &str) -> Option<Decorator> {
        // Decorator is like @Name or @Name(args)
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "identifier" => {
                    return Some(Decorator::new(&self.node_text(&child, source)));
                }
                "call_expression" => {
                    let func = child.child_by_field_name("function")?;
                    let name = self.node_text(&func, source);

                    let args = child
                        .child_by_field_name("arguments")
                        .map(|args_node| self.extract_call_args(&args_node, source))
                        .unwrap_or_default();

                    let mut dec = Decorator::new(&name);
                    for arg in args {
                        dec = dec.with_arg(&arg);
                    }
                    return Some(dec);
                }
                _ => {}
            }
        }
        None
    }

    fn extract_call_args(&self, args_node: &Node, source: &str) -> Vec<String> {
        let mut args = vec![];
        let mut cursor = args_node.walk();

        for child in args_node.children(&mut cursor) {
            match child.kind() {
                "string" | "template_string" | "identifier" | "property_access_expression"
                | "member_expression" => {
                    args.push(self.node_text(&child, source));
                }
                _ => {}
            }
        }
        args
    }

    fn classify_class(&self, decorators: &[Decorator]) -> ClassKind {
        for dec in decorators {
            match dec.name.as_str() {
                "Controller" => return ClassKind::Controller,
                "Injectable" => {
                    // Check if it's a specific type
                    for d in decorators {
                        if d.name == "UseGuards" {
                            return ClassKind::Guard;
                        }
                    }
                    return ClassKind::Service;
                }
                "Entity" => return ClassKind::Entity,
                _ => {}
            }
        }
        ClassKind::Other
    }

    fn extract_class_members(
        &self,
        body: &Node,
        source: &str,
        file_path: &str,
        class_name: &str,
    ) -> (Vec<MethodInfo>, Vec<InjectionInfo>) {
        let mut methods = vec![];
        let mut injections = vec![];

        // Collect pending decorators - they appear as siblings BEFORE the method
        let mut pending_decorators: Vec<Decorator> = vec![];

        let child_count = body.child_count();
        for i in 0..child_count {
            let Some(child) = body.child(i) else { continue };

            match child.kind() {
                "decorator" => {
                    // Accumulate decorators until we hit a method
                    if let Some(dec) = self.parse_decorator(&child, source) {
                        pending_decorators.push(dec);
                    }
                }
                "method_definition" => {
                    // Pass accumulated decorators to method extraction
                    if let Some(mut method) = self.extract_method(&child, source, file_path, class_name) {
                        // Prepend the pending decorators
                        method.decorators = [pending_decorators.clone(), method.decorators].concat();

                        // Debug: log method decorators
                        let dec_names: Vec<_> = method.decorators.iter().map(|d| d.name.as_str()).collect();
                        tracing::debug!(
                            "Method {}.{} has decorators: {:?}",
                            class_name,
                            method.name,
                            dec_names
                        );

                        methods.push(method);
                    }
                    pending_decorators.clear();
                }
                "public_field_definition" | "property_declaration" => {
                    if let Some(inj) = self.extract_injection(&child, source) {
                        injections.push(inj);
                    }
                    pending_decorators.clear();
                }
                _ => {
                    // Reset decorators on other node types
                    pending_decorators.clear();
                }
            }
        }

        (methods, injections)
    }

    fn extract_method(
        &self,
        node: &Node,
        source: &str,
        file_path: &str,
        class_name: &str,
    ) -> Option<MethodInfo> {
        let name_node = node.child_by_field_name("name")?;
        let name = self.node_text(&name_node, source);

        // Skip constructor
        if name == "constructor" {
            return None;
        }

        let decorators = self.extract_decorators(node, source);

        // Check if async
        let is_async = node.children(&mut node.walk()).any(|c| c.kind() == "async");

        // Check if private (starts with # or has private keyword)
        let is_private = name.starts_with('#')
            || node
                .children(&mut node.walk())
                .any(|c| c.kind() == "accessibility_modifier" && self.node_text(&c, source) == "private");

        // Extract parameters
        let parameters = node
            .child_by_field_name("parameters")
            .map(|p| self.extract_parameters(&p, source))
            .unwrap_or_default();

        Some(MethodInfo {
            id: SymbolId::method(file_path, class_name, &name),
            name,
            is_async,
            is_private,
            decorators,
            parameters,
            start_line: node.start_position().row as u32 + 1,
            end_line: node.end_position().row as u32 + 1,
        })
    }

    fn extract_parameters(&self, node: &Node, source: &str) -> Vec<ParameterInfo> {
        let mut params = vec![];
        let mut cursor = node.walk();

        for child in node.children(&mut cursor) {
            if child.kind() == "required_parameter" || child.kind() == "optional_parameter" {
                let decorators = self.extract_decorators(&child, source);

                let name = child
                    .child_by_field_name("pattern")
                    .map(|n| self.node_text(&n, source))
                    .unwrap_or_default();

                let type_name = child
                    .child_by_field_name("type")
                    .map(|n| self.node_text(&n, source).trim_start_matches(':').trim().to_string());

                params.push(ParameterInfo {
                    name,
                    type_name,
                    decorators,
                });
            }
        }
        params
    }

    fn extract_injection(&self, node: &Node, source: &str) -> Option<InjectionInfo> {
        let decorators = self.extract_decorators(node, source);

        // Look for @Inject or @InjectRepository
        let (kind, type_arg) = decorators.iter().find_map(|d| match d.name.as_str() {
            "Inject" => Some((InjectionKind::Inject, d.args.first().cloned())),
            "InjectRepository" => Some((InjectionKind::InjectRepository, d.args.first().cloned())),
            _ => None,
        })?;

        let prop_name = node
            .child_by_field_name("name")
            .map(|n| self.node_text(&n, source))?;

        let type_name = type_arg.unwrap_or_else(|| {
            node.child_by_field_name("type")
                .map(|n| self.node_text(&n, source).trim_start_matches(':').trim().to_string())
                .unwrap_or_default()
        });

        Some(InjectionInfo {
            property_name: prop_name,
            type_name,
            kind,
        })
    }

    /// Pass 2: Build flows from the indexed symbols
    pub fn build_flows(&mut self, source_files: &[(String, String)]) -> crate::Result<Vec<FlowGraph>> {
        let mut flows = vec![];
        let mut method_flows: std::collections::HashMap<String, u64> = std::collections::HashMap::new();

        // Build flows for HTTP endpoints
        let endpoints = self.symbol_index.http_endpoints();
        for endpoint in endpoints {
            let Some((_, source)) = source_files.iter().find(|(path, _)| *path == endpoint.file_path) else {
                continue;
            };

            let tree = self
                .parser
                .parse(source, None)
                .ok_or_else(|| crate::ParseError::ParseFailed {
                    path: endpoint.file_path.clone(),
                })?;

            if let Some(flow) = self.build_endpoint_flow(&endpoint, &tree.root_node(), source) {
                flows.push(flow);
            }
        }

        // Build flows for ALL service methods (for drill-down)
        let classes: Vec<_> = self.symbol_index.classes.values().cloned().collect();
        for class in classes {
            // Skip controllers (already handled above) and entities
            if matches!(class.kind, ClassKind::Controller | ClassKind::Entity) {
                continue;
            }

            let Some((_, source)) = source_files.iter().find(|(path, _)| *path == class.file_path) else {
                continue;
            };

            let tree = self
                .parser
                .parse(source, None)
                .ok_or_else(|| crate::ParseError::ParseFailed {
                    path: class.file_path.clone(),
                })?;

            for method in &class.methods {
                if method.is_private {
                    continue;
                }

                if let Some(flow) = self.build_method_flow(&class, method, &tree.root_node(), source) {
                    // Register for cross-linking
                    let key = format!("{}::{}", class.name, method.name);
                    method_flows.insert(key, flow.id.0);
                    flows.push(flow);
                }
            }
        }

        // Resolve target_flow_id in ServiceCall nodes
        self.resolve_service_calls(&mut flows, &method_flows);

        Ok(flows)
    }

    fn build_method_flow(
        &mut self,
        class: &ClassInfo,
        method: &MethodInfo,
        root: &Node,
        source: &str,
    ) -> Option<FlowGraph> {
        let flow_id = self.alloc_flow_id();
        let name = format!("{}.{}", class.name, method.name);

        let mut flow = FlowGraph::new(
            flow_id,
            &name,
            &class.file_path,
            EntryPointKind::ExportedFunction,
        );

        // Create method entry node
        let entry_id = self.alloc_node_id();
        let entry_node = FlowNode::new(
            entry_id,
            NodeKind::FunctionEntry,
            &name,
            &format!("{}()", method.name),
            SourceLocation {
                file_path: class.file_path.clone(),
                start_line: method.start_line,
                end_line: method.start_line,
                start_col: 0,
                end_col: 0,
            },
        );
        flow.add_node(entry_node);

        // Find and extract method body
        if let Some(body) = self.find_method_body_at_line(root, method.start_line) {
            let mut scope = VariableScope::default();
            self.extract_body_nodes(
                &body,
                source,
                &class.file_path,
                &mut flow,
                NodeId(entry_id),
                PinId(1),
                &mut scope,
            );
        }

        Some(flow)
    }

    fn resolve_service_calls(
        &self,
        flows: &mut [FlowGraph],
        method_flows: &std::collections::HashMap<String, u64>,
    ) {
        // Build injection map: controller -> (property_name -> service_class_name)
        let mut injection_map: std::collections::HashMap<String, std::collections::HashMap<String, String>> =
            std::collections::HashMap::new();

        for class in self.symbol_index.classes.values() {
            let mut props = std::collections::HashMap::new();
            for injection in &class.injections {
                props.insert(injection.property_name.clone(), injection.type_name.clone());
            }
            if !props.is_empty() {
                tracing::debug!("Injections for {}: {:?}", class.name, props);
            }
            injection_map.insert(class.name.clone(), props);
        }

        tracing::debug!("Method flows available: {:?}", method_flows.keys().collect::<Vec<_>>());

        // Update ServiceCall nodes with target_flow_id
        for flow in flows.iter_mut() {
            // Try to find the class name from the flow's file path
            let flow_class = self.find_class_for_flow(flow);

            for node in flow.nodes.values_mut() {
                if let NodeKind::ServiceCall { service, method, target_flow_id } = &mut node.kind {
                    tracing::debug!(
                        "ServiceCall: {}.{} in flow '{}' (class: {:?})",
                        service, method, flow.name, flow_class
                    );

                    // Look up the service type from injections
                    if let Some(class_name) = &flow_class {
                        if let Some(props) = injection_map.get(class_name) {
                            if let Some(service_class) = props.get(service) {
                                // Find the target flow
                                let key = format!("{}::{}", service_class, method);
                                tracing::debug!("Looking for flow key: {}", key);
                                if let Some(&flow_id) = method_flows.get(&key) {
                                    *target_flow_id = Some(flow_id);
                                    tracing::info!(
                                        "Resolved {}.{} -> flow {}",
                                        service_class, method, flow_id
                                    );
                                } else {
                                    tracing::warn!("No flow found for key: {}", key);
                                }
                            } else {
                                tracing::warn!("No injection found for property: {}", service);
                            }
                        } else {
                            tracing::warn!("No injections for class: {}", class_name);
                        }
                    }
                }
            }
        }
    }

    /// Find the class name that owns a flow based on file path
    fn find_class_for_flow(&self, flow: &FlowGraph) -> Option<String> {
        for class in self.symbol_index.classes.values() {
            if class.file_path == flow.file_path {
                return Some(class.name.clone());
            }
        }
        None
    }

    fn build_endpoint_flow(
        &mut self,
        endpoint: &HttpEndpoint,
        root: &Node,
        source: &str,
    ) -> Option<FlowGraph> {
        let flow_id = self.alloc_flow_id();
        let name = format!("{} {}", endpoint.method, endpoint.path);

        let mut flow = FlowGraph::new(
            flow_id,
            &name,
            &endpoint.file_path,
            EntryPointKind::HttpHandler {
                method: endpoint.method.clone(),
                path: endpoint.path.clone(),
            },
        );

        // Create HTTP entry node
        let entry_id = self.alloc_node_id();
        let entry_node = FlowNode::new(
            entry_id,
            NodeKind::HttpHandler {
                method: endpoint.method.clone(),
                path: endpoint.path.clone(),
            },
            &name,
            &format!("@{}('{}')", endpoint.method, endpoint.path),
            SourceLocation {
                file_path: endpoint.file_path.clone(),
                start_line: endpoint.line,
                end_line: endpoint.line,
                start_col: 0,
                end_col: 0,
            },
        );
        flow.add_node(entry_node);

        let mut prev_node = NodeId(entry_id);
        let mut prev_pin = PinId(1);

        // Add guard nodes
        for guard in &endpoint.guards {
            let guard_id = self.alloc_node_id();
            let guard_node = FlowNode::new(
                guard_id,
                NodeKind::Guard {
                    name: guard.clone(),
                },
                &format!("Guard: {}", guard),
                &format!("@UseGuards({})", guard),
                SourceLocation {
                    file_path: endpoint.file_path.clone(),
                    start_line: endpoint.line,
                    end_line: endpoint.line,
                    start_col: 0,
                    end_col: 0,
                },
            );
            flow.add_node(guard_node);

            // Connect from previous
            flow.add_edge(FlowEdge {
                id: EdgeId(self.alloc_edge_id()),
                from_node: prev_node,
                from_pin: prev_pin,
                to_node: NodeId(guard_id),
                to_pin: PinId(1),
                kind: EdgeKind::Execution,
                label: None,
            });

            prev_node = NodeId(guard_id);
            prev_pin = PinId(2); // success output
        }

        // Find the method body in AST and extract its flow
        if let Some(body) = self.find_method_body_at_line(root, endpoint.line) {
            let mut scope = VariableScope::default();
            self.extract_body_nodes(
                &body,
                source,
                &endpoint.file_path,
                &mut flow,
                prev_node,
                prev_pin,
                &mut scope,
            );
        }

        Some(flow)
    }

    fn find_method_body_at_line<'a>(&self, node: &Node<'a>, target_line: u32) -> Option<Node<'a>> {
        let node_line = node.start_position().row as u32 + 1;

        // If this is the target method, return its body
        if node.kind() == "method_definition" && node_line == target_line {
            return node.child_by_field_name("body");
        }

        // Recurse into children
        let child_count = node.child_count();
        for i in 0..child_count {
            if let Some(child) = node.child(i) {
                if let Some(body) = self.find_method_body_at_line(&child, target_line) {
                    return Some(body);
                }
            }
        }
        None
    }

    fn extract_body_nodes(
        &mut self,
        body: &Node,
        source: &str,
        file_path: &str,
        flow: &mut FlowGraph,
        prev_node: NodeId,
        prev_pin: PinId,
        scope: &mut VariableScope,
    ) -> Option<(NodeId, PinId)> {
        let mut current_node = prev_node;
        let mut current_pin = prev_pin;

        let mut cursor = body.walk();
        for child in body.children(&mut cursor) {
            if let Some((new_node, new_pin)) =
                self.extract_statement(&child, source, file_path, flow, current_node, current_pin, scope)
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
        scope: &mut VariableScope,
    ) -> Option<(NodeId, PinId)> {
        match node.kind() {
            "if_statement" => self.extract_if(node, source, file_path, flow, prev_node, prev_pin, scope),
            "try_statement" => {
                self.extract_try_catch(node, source, file_path, flow, prev_node, prev_pin, scope)
            }
            "for_statement" | "for_in_statement" | "while_statement" => {
                self.extract_loop(node, source, file_path, flow, prev_node, prev_pin, scope)
            }
            "return_statement" => {
                self.extract_return(node, source, file_path, flow, prev_node, prev_pin, scope)
            }
            "throw_statement" => {
                self.extract_throw(node, source, file_path, flow, prev_node, prev_pin)
            }
            "expression_statement" => {
                if let Some(expr) = node.child(0) {
                    self.extract_expression(&expr, source, file_path, flow, prev_node, prev_pin, scope)
                } else {
                    None
                }
            }
            "lexical_declaration" | "variable_declaration" => {
                self.extract_declaration(node, source, file_path, flow, prev_node, prev_pin, scope)
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
        scope: &mut VariableScope,
    ) -> Option<(NodeId, PinId)> {
        // Look for initializer with await or call
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "variable_declarator" {
                // Get variable name
                let var_name = child
                    .child_by_field_name("name")
                    .map(|n| self.node_text(&n, source));

                if let Some(value) = child.child_by_field_name("value") {
                    let result = self.extract_expression(&value, source, file_path, flow, prev_node, prev_pin, scope);

                    // Bind variable to the result node
                    if let (Some(name), Some((node_id, _))) = (var_name, result) {
                        // The result pin for data is typically PinId(3) for "result" output
                        scope.bind(&name, node_id, PinId(4)); // result output pin
                    }

                    return result;
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
        scope: &VariableScope,
    ) -> Option<(NodeId, PinId)> {
        match node.kind() {
            "await_expression" => {
                self.extract_await(node, source, file_path, flow, prev_node, prev_pin)
            }
            "call_expression" => {
                self.extract_call_with_data(node, source, file_path, flow, prev_node, prev_pin, scope)
            }
            _ => None,
        }
    }

    fn extract_if(
        &mut self,
        node: &Node,
        source: &str,
        file_path: &str,
        flow: &mut FlowGraph,
        prev_node: NodeId,
        prev_pin: PinId,
        scope: &mut VariableScope,
    ) -> Option<(NodeId, PinId)> {
        let condition = node.child_by_field_name("condition")?;
        let condition_text = self.node_text(&condition, source);

        let node_id = self.alloc_node_id();
        let if_node = FlowNode::new(
            node_id,
            NodeKind::Condition {
                expression: condition_text.clone(),
            },
            &format!("If: {}", self.truncate(&condition_text, 30)),
            &format!("if ({})", condition_text),
            self.node_location(node, file_path),
        );
        flow.add_node(if_node);

        flow.add_edge(FlowEdge {
            id: EdgeId(self.alloc_edge_id()),
            from_node: prev_node,
            from_pin: prev_pin,
            to_node: NodeId(node_id),
            to_pin: PinId(1),
            kind: EdgeKind::Execution,
            label: None,
        });

        if let Some(consequence) = node.child_by_field_name("consequence") {
            self.extract_body_nodes(&consequence, source, file_path, flow, NodeId(node_id), PinId(2), scope);
        }

        if let Some(alternative) = node.child_by_field_name("alternative") {
            self.extract_body_nodes(&alternative, source, file_path, flow, NodeId(node_id), PinId(3), scope);
        }

        Some((NodeId(node_id), PinId(2)))
    }

    fn extract_try_catch(
        &mut self,
        node: &Node,
        source: &str,
        file_path: &str,
        flow: &mut FlowGraph,
        prev_node: NodeId,
        prev_pin: PinId,
        scope: &mut VariableScope,
    ) -> Option<(NodeId, PinId)> {
        let node_id = self.alloc_node_id();
        let try_node = FlowNode::new(
            node_id,
            NodeKind::TryCatch,
            "Try/Catch",
            "try { ... } catch { ... }",
            self.node_location(node, file_path),
        );
        flow.add_node(try_node);

        flow.add_edge(FlowEdge {
            id: EdgeId(self.alloc_edge_id()),
            from_node: prev_node,
            from_pin: prev_pin,
            to_node: NodeId(node_id),
            to_pin: PinId(1),
            kind: EdgeKind::Execution,
            label: None,
        });

        if let Some(body) = node.child_by_field_name("body") {
            self.extract_body_nodes(&body, source, file_path, flow, NodeId(node_id), PinId(2), scope);
        }

        if let Some(handler) = node.child_by_field_name("handler") {
            if let Some(catch_body) = handler.child_by_field_name("body") {
                self.extract_body_nodes(&catch_body, source, file_path, flow, NodeId(node_id), PinId(3), scope);
            }
        }

        Some((NodeId(node_id), PinId(2)))
    }

    fn extract_loop(
        &mut self,
        node: &Node,
        source: &str,
        file_path: &str,
        flow: &mut FlowGraph,
        prev_node: NodeId,
        prev_pin: PinId,
        scope: &mut VariableScope,
    ) -> Option<(NodeId, PinId)> {
        let loop_kind = match node.kind() {
            "for_statement" => LoopKind::For,
            "for_in_statement" => LoopKind::ForIn,
            "while_statement" => LoopKind::While,
            _ => LoopKind::For,
        };

        let node_id = self.alloc_node_id();
        let loop_node = FlowNode::new(
            node_id,
            NodeKind::Loop { kind: loop_kind },
            "Loop",
            &self.truncate(&self.node_text(node, source), 40),
            self.node_location(node, file_path),
        );
        flow.add_node(loop_node);

        flow.add_edge(FlowEdge {
            id: EdgeId(self.alloc_edge_id()),
            from_node: prev_node,
            from_pin: prev_pin,
            to_node: NodeId(node_id),
            to_pin: PinId(1),
            kind: EdgeKind::Execution,
            label: None,
        });

        if let Some(body) = node.child_by_field_name("body") {
            self.extract_body_nodes(&body, source, file_path, flow, NodeId(node_id), PinId(2), scope);
        }

        Some((NodeId(node_id), PinId(3)))
    }

    fn extract_return(
        &mut self,
        node: &Node,
        source: &str,
        file_path: &str,
        flow: &mut FlowGraph,
        prev_node: NodeId,
        prev_pin: PinId,
        scope: &mut VariableScope,
    ) -> Option<(NodeId, PinId)> {
        let has_value = node.child_count() > 1;

        // First, extract expressions inside the return (await, calls, etc.)
        let (current_node, current_pin) = if has_value {
            if let Some(expr) = node.child(1) {
                if let Some((n, p)) = self.extract_expression_deep(&expr, source, file_path, flow, prev_node, prev_pin, scope) {
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

        None
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
        scope: &VariableScope,
    ) -> Option<(NodeId, PinId)> {
        match node.kind() {
            "await_expression" => {
                let argument = node.child(1)?;
                // Check if argument is a service call
                if let Some(result) = self.try_extract_service_call(&argument, source, file_path, flow, prev_node, prev_pin, scope) {
                    return Some(result);
                }
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
                            if let Some((n, p)) = self.extract_expression_deep(&arg, source, file_path, flow, current_node, current_pin, scope) {
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
                        if let Some((n, p)) = self.extract_expression_deep(&object, source, file_path, flow, current_node, current_pin, scope) {
                            current_node = n;
                            current_pin = p;
                        }
                    }

                    // Check if it's a service call
                    if let Some(result) = self.try_extract_service_call(node, source, file_path, flow, current_node, current_pin, scope) {
                        return Some(result);
                    }
                    self.extract_call_with_data(node, source, file_path, flow, current_node, current_pin, scope)
                } else {
                    if let Some(result) = self.try_extract_service_call(node, source, file_path, flow, current_node, current_pin, scope) {
                        return Some(result);
                    }
                    self.extract_call_with_data(node, source, file_path, flow, current_node, current_pin, scope)
                }
            }
            "parenthesized_expression" => {
                if let Some(inner) = node.child(1) {
                    self.extract_expression_deep(&inner, source, file_path, flow, prev_node, prev_pin, scope)
                } else {
                    None
                }
            }
            "member_expression" => {
                let object = node.child_by_field_name("object")?;
                if self.is_extractable_expression(&object) {
                    self.extract_expression_deep(&object, source, file_path, flow, prev_node, prev_pin, scope)
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

    fn extract_throw(
        &mut self,
        node: &Node,
        source: &str,
        file_path: &str,
        flow: &mut FlowGraph,
        prev_node: NodeId,
        prev_pin: PinId,
    ) -> Option<(NodeId, PinId)> {
        let node_id = self.alloc_node_id();

        let throw_node = FlowNode::new(
            node_id,
            NodeKind::Throw,
            "Throw Error",
            &self.node_text(node, source),
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

        None
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
        let argument = node.child(1)?; // await <expr>

        // Check if this is a service call (this.service.method())
        let empty_scope = VariableScope::default();
        if let Some(call) = self.try_extract_service_call(&argument, source, file_path, flow, prev_node, prev_pin, &empty_scope) {
            return Some(call);
        }

        // Fall back to generic await
        let code = self.node_text(&argument, source);
        let node_id = self.alloc_node_id();

        let await_node = FlowNode::new(
            node_id,
            NodeKind::Await,
            &format!("Await: {}", self.truncate(&code, 25)),
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

    fn try_extract_service_call(
        &mut self,
        node: &Node,
        source: &str,
        file_path: &str,
        flow: &mut FlowGraph,
        prev_node: NodeId,
        prev_pin: PinId,
        scope: &VariableScope,
    ) -> Option<(NodeId, PinId)> {
        if node.kind() != "call_expression" {
            return None;
        }

        let callee = node.child_by_field_name("function")?;
        if callee.kind() != "member_expression" {
            return None;
        }

        let object = callee.child_by_field_name("object")?;
        let property = callee.child_by_field_name("property")?;

        // Check if it's this.something.method()
        let is_this_call = if object.kind() == "member_expression" {
            // this.service.method()
            object
                .child_by_field_name("object")
                .is_some_and(|o| self.node_text(&o, source) == "this")
        } else {
            self.node_text(&object, source) == "this"
        };

        if !is_this_call {
            return None;
        }

        let method_name = self.node_text(&property, source);

        // Get the service name (the property after 'this.')
        let service_name = if object.kind() == "member_expression" {
            object
                .child_by_field_name("property")
                .map(|p| self.node_text(&p, source))
                .unwrap_or_default()
        } else {
            // Direct this.method() - internal method call
            return None;
        };

        let node_id = self.alloc_node_id();

        // Determine if this is a repository call
        let (kind, label) = if service_name.contains("Repository") || service_name.ends_with("Repo") {
            let entity = service_name
                .trim_end_matches("Repository")
                .trim_end_matches("Repo")
                .to_string();
            (
                NodeKind::RepositoryCall {
                    entity: entity.clone(),
                    method: method_name.clone(),
                },
                format!("{}.{}", entity, method_name),
            )
        } else {
            (
                NodeKind::ServiceCall {
                    service: service_name.clone(),
                    method: method_name.clone(),
                    target_flow_id: None, // TODO: resolve cross-file
                },
                format!("{}.{}", service_name, method_name),
            )
        };

        let service_node = FlowNode::new(
            node_id,
            kind,
            &label,
            &format!("this.{}.{}()", service_name, method_name),
            self.node_location(node, file_path),
        );
        flow.add_node(service_node);

        flow.add_edge(FlowEdge {
            id: EdgeId(self.alloc_edge_id()),
            from_node: prev_node,
            from_pin: prev_pin,
            to_node: NodeId(node_id),
            to_pin: PinId(1),
            kind: EdgeKind::Execution,
            label: None,
        });

        // Add data edges for variable arguments
        self.add_data_edges_for_args(node, source, flow, NodeId(node_id), scope);

        Some((NodeId(node_id), PinId(2)))
    }

    /// Add data edges from variable bindings to a call node's input
    fn add_data_edges_for_args(
        &mut self,
        node: &Node,
        source: &str,
        flow: &mut FlowGraph,
        to_node: NodeId,
        scope: &VariableScope,
    ) {
        if let Some(args) = node.child_by_field_name("arguments") {
            let mut cursor = args.walk();
            for arg in args.children(&mut cursor) {
                // Find the root variable in the expression
                if let Some((var_name, binding)) = self.find_variable_in_expr(&arg, source, scope) {
                    flow.add_edge(FlowEdge {
                        id: EdgeId(self.alloc_edge_id()),
                        from_node: binding.source_node,
                        from_pin: binding.source_pin,
                        to_node,
                        to_pin: PinId(5), // data input pin (args)
                        kind: EdgeKind::Data,
                        label: Some(var_name),
                    });
                }
            }
        }
    }

    /// Find a tracked variable in an expression (handles member access, calls, etc.)
    fn find_variable_in_expr<'a>(
        &self,
        node: &Node,
        source: &str,
        scope: &'a VariableScope,
    ) -> Option<(String, &'a VariableBinding)> {
        match node.kind() {
            "identifier" => {
                let name = self.node_text(node, source);
                scope.get(&name).map(|b| (name, b))
            }
            "member_expression" => {
                // user.id -> trace back to "user"
                let object = node.child_by_field_name("object")?;
                self.find_variable_in_expr(&object, source, scope)
            }
            "call_expression" => {
                // dto.toObject() -> trace back to "dto"
                let callee = node.child_by_field_name("function")?;
                self.find_variable_in_expr(&callee, source, scope)
            }
            "subscript_expression" => {
                // arr[0] -> trace back to "arr"
                let object = node.child_by_field_name("object")?;
                self.find_variable_in_expr(&object, source, scope)
            }
            _ => None,
        }
    }

    fn extract_call_with_data(
        &mut self,
        node: &Node,
        source: &str,
        file_path: &str,
        flow: &mut FlowGraph,
        prev_node: NodeId,
        prev_pin: PinId,
        scope: &VariableScope,
    ) -> Option<(NodeId, PinId)> {
        // Try service call first
        if let Some(result) = self.try_extract_service_call(node, source, file_path, flow, prev_node, prev_pin, scope) {
            return Some(result);
        }

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

        // Add data edges for variable arguments
        self.add_data_edges_for_args(node, source, flow, NodeId(node_id), scope);

        Some((NodeId(node_id), PinId(2)))
    }

    // Helpers

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

    /// Get the symbol index for inspection
    pub fn symbol_index(&self) -> &SymbolIndex {
        &self.symbol_index
    }
}

impl Default for NestJsExtractor {
    fn default() -> Self {
        Self::new().expect("Failed to create NestJS extractor")
    }
}
