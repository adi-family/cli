use crate::node::{FlowNode, NodeId};
use crate::pin::PinId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FlowId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EdgeId(pub u64);

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EdgeKind {
    Execution,
    Data,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowEdge {
    pub id: EdgeId,
    pub from_node: NodeId,
    pub from_pin: PinId,
    pub to_node: NodeId,
    pub to_pin: PinId,
    pub kind: EdgeKind,
    /// Label for data edges (variable name)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntryPointKind {
    HttpHandler { method: String, path: String },
    EventListener { event: String },
    ExportedFunction,
    MainFunction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowGraph {
    pub id: FlowId,
    pub name: String,
    pub file_path: String,
    pub entry_kind: EntryPointKind,
    pub nodes: HashMap<NodeId, FlowNode>,
    pub edges: Vec<FlowEdge>,
}

impl FlowGraph {
    pub fn new(id: u64, name: &str, file_path: &str, entry_kind: EntryPointKind) -> Self {
        Self {
            id: FlowId(id),
            name: name.to_string(),
            file_path: file_path.to_string(),
            entry_kind,
            nodes: HashMap::new(),
            edges: Vec::new(),
        }
    }

    pub fn add_node(&mut self, node: FlowNode) {
        self.nodes.insert(node.id, node);
    }

    pub fn add_edge(&mut self, edge: FlowEdge) {
        // Mark pins as connected
        if let Some(from_node) = self.nodes.get_mut(&edge.from_node) {
            for pin in &mut from_node.outputs {
                if pin.id == edge.from_pin {
                    pin.connected = true;
                }
            }
        }
        if let Some(to_node) = self.nodes.get_mut(&edge.to_node) {
            for pin in &mut to_node.inputs {
                if pin.id == edge.to_pin {
                    pin.connected = true;
                }
            }
        }
        self.edges.push(edge);
    }

    pub fn entry_node(&self) -> Option<&FlowNode> {
        self.nodes.values().find(|n| n.inputs.is_empty())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowSummary {
    pub id: FlowId,
    pub name: String,
    pub file_path: String,
    pub entry_kind: EntryPointKind,
    pub node_count: usize,
}

impl From<&FlowGraph> for FlowSummary {
    fn from(graph: &FlowGraph) -> Self {
        Self {
            id: graph.id,
            name: graph.name.clone(),
            file_path: graph.file_path.clone(),
            entry_kind: graph.entry_kind.clone(),
            node_count: graph.nodes.len(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowIndex {
    pub root_path: String,
    pub flows: Vec<FlowGraph>,
    /// Maps method symbol IDs to their flow IDs for cross-file linking
    #[serde(default)]
    pub method_flows: HashMap<String, FlowId>,
}

impl FlowIndex {
    pub fn new(root_path: &str) -> Self {
        Self {
            root_path: root_path.to_string(),
            flows: Vec::new(),
            method_flows: HashMap::new(),
        }
    }

    pub fn summaries(&self) -> Vec<FlowSummary> {
        self.flows.iter().map(FlowSummary::from).collect()
    }

    pub fn get_flow(&self, id: FlowId) -> Option<&FlowGraph> {
        self.flows.iter().find(|f| f.id == id)
    }

    pub fn get_flow_mut(&mut self, id: FlowId) -> Option<&mut FlowGraph> {
        self.flows.iter_mut().find(|f| f.id == id)
    }

    /// Register a method's flow for cross-file linking
    pub fn register_method_flow(&mut self, class_name: &str, method_name: &str, flow_id: FlowId) {
        let key = format!("{}::{}", class_name, method_name);
        self.method_flows.insert(key, flow_id);
    }

    /// Find a flow by class and method name
    pub fn find_method_flow(&self, class_name: &str, method_name: &str) -> Option<FlowId> {
        let key = format!("{}::{}", class_name, method_name);
        self.method_flows.get(&key).copied()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FlowIssueKind {
    UnhandledError,
    UnreachableCode,
    InfiniteLoop,
    MissingReturn,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IssueSeverity {
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowIssue {
    pub kind: FlowIssueKind,
    pub node_id: NodeId,
    pub message: String,
    pub severity: IssueSeverity,
}
