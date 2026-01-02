use crate::pin::{Pin, PinDirection, PinKind};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeId(pub u64);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceLocation {
    pub file_path: String,
    pub start_line: u32,
    pub end_line: u32,
    pub start_col: u32,
    pub end_col: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum NodeKind {
    // Entry points
    HttpHandler {
        method: String,
        path: String,
    },
    EventListener {
        event: String,
    },
    FunctionEntry,
    ExportedFunction,

    // NestJS middleware chain
    Guard {
        name: String,
    },
    Pipe {
        name: String,
    },
    Middleware {
        name: String,
    },
    Interceptor {
        name: String,
    },

    // Control flow
    Condition {
        expression: String,
    },
    /// Merge point after branching (if/else, switch, etc.)
    Merge,
    Loop {
        kind: LoopKind,
    },
    TryCatch,
    Await,

    // Actions
    FunctionCall {
        name: String,
        is_async: bool,
    },
    MethodCall {
        object: String,
        method: String,
        is_async: bool,
    },
    /// Call to an injected service method
    ServiceCall {
        service: String,
        method: String,
        /// Link to the target method's flow (if resolved)
        target_flow_id: Option<u64>,
    },
    /// Call to a TypeORM repository method
    RepositoryCall {
        entity: String,
        method: String,
    },
    ExternalCall {
        service: String,
    },
    /// Database query (TypeORM, Prisma, etc.)
    DatabaseQuery {
        kind: DatabaseQueryKind,
        entity: Option<String>,
    },

    // Terminals
    Return {
        has_value: bool,
    },
    Throw,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DatabaseQueryKind {
    Select,
    Insert,
    Update,
    Delete,
    Transaction,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LoopKind {
    For,
    ForOf,
    ForIn,
    While,
    DoWhile,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowNode {
    pub id: NodeId,
    pub kind: NodeKind,
    /// Short label for display (e.g., "If: user != null")
    pub label: String,
    /// Raw code snippet
    pub code_label: String,
    /// Human-readable description (LLM-generated)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub location: SourceLocation,
    pub inputs: Vec<Pin>,
    pub outputs: Vec<Pin>,
    pub position: Option<NodePosition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodePosition {
    pub x: f64,
    pub y: f64,
}

impl FlowNode {
    pub fn new(id: u64, kind: NodeKind, label: &str, code_label: &str, location: SourceLocation) -> Self {
        let (inputs, outputs) = Self::default_pins(&kind);

        Self {
            id: NodeId(id),
            kind,
            label: label.to_string(),
            code_label: code_label.to_string(),
            description: None,
            location,
            inputs,
            outputs,
            position: None,
        }
    }

    /// Set the human-readable description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    fn default_pins(kind: &NodeKind) -> (Vec<Pin>, Vec<Pin>) {
        match kind {
            NodeKind::HttpHandler { .. } | NodeKind::EventListener { .. } | NodeKind::FunctionEntry | NodeKind::ExportedFunction => {
                (vec![], vec![Pin::exec_out().with_id(1)])
            }

            // NestJS middleware chain - passthrough with possible rejection
            NodeKind::Guard { .. } => (
                vec![Pin::exec_in().with_id(1)],
                vec![
                    Pin::exec_out().with_id(2),
                    Pin {
                        id: crate::pin::PinId(3),
                        kind: PinKind::Exec,
                        direction: PinDirection::Output,
                        label: "reject".to_string(),
                        connected: false,
                    },
                ],
            ),

            NodeKind::Pipe { .. } | NodeKind::Middleware { .. } | NodeKind::Interceptor { .. } => (
                vec![Pin::exec_in().with_id(1)],
                vec![
                    Pin::exec_out().with_id(2),
                    Pin::error_out().with_id(3),
                ],
            ),

            // Service/Repository calls - can throw, receive data args, produce result
            NodeKind::ServiceCall { .. } | NodeKind::RepositoryCall { .. } => (
                vec![
                    Pin::exec_in().with_id(1),
                    Pin::data_in(PinKind::Any, "args").with_id(5), // data input for variable args
                ],
                vec![
                    Pin::exec_out().with_id(2),
                    Pin::error_out().with_id(3),
                    Pin::data_out(PinKind::Any, "result").with_id(4),
                ],
            ),

            NodeKind::DatabaseQuery { .. } => (
                vec![Pin::exec_in().with_id(1)],
                vec![
                    Pin::exec_out().with_id(2),
                    Pin::error_out().with_id(3),
                    Pin::data_out(PinKind::Any, "rows").with_id(4),
                ],
            ),

            NodeKind::Condition { .. } => (
                vec![Pin::exec_in().with_id(1)],
                vec![
                    Pin {
                        id: crate::pin::PinId(2),
                        kind: PinKind::Exec,
                        direction: PinDirection::Output,
                        label: "true".to_string(),
                        connected: false,
                    },
                    Pin {
                        id: crate::pin::PinId(3),
                        kind: PinKind::Exec,
                        direction: PinDirection::Output,
                        label: "false".to_string(),
                        connected: false,
                    },
                ],
            ),

            // Merge has two inputs (from branches) and one output
            NodeKind::Merge => (
                vec![
                    Pin {
                        id: crate::pin::PinId(1),
                        kind: PinKind::Exec,
                        direction: PinDirection::Input,
                        label: "branch_a".to_string(),
                        connected: false,
                    },
                    Pin {
                        id: crate::pin::PinId(2),
                        kind: PinKind::Exec,
                        direction: PinDirection::Input,
                        label: "branch_b".to_string(),
                        connected: false,
                    },
                ],
                vec![Pin::exec_out().with_id(3)],
            ),

            NodeKind::Loop { .. } => (
                vec![Pin::exec_in().with_id(1)],
                vec![
                    Pin {
                        id: crate::pin::PinId(2),
                        kind: PinKind::Exec,
                        direction: PinDirection::Output,
                        label: "body".to_string(),
                        connected: false,
                    },
                    Pin {
                        id: crate::pin::PinId(3),
                        kind: PinKind::Exec,
                        direction: PinDirection::Output,
                        label: "done".to_string(),
                        connected: false,
                    },
                ],
            ),

            NodeKind::TryCatch => (
                vec![Pin::exec_in().with_id(1)],
                vec![
                    Pin::exec_out().with_id(2),
                    Pin::error_out().with_id(3),
                ],
            ),

            NodeKind::Await => (
                vec![Pin::exec_in().with_id(1)],
                vec![
                    Pin::exec_out().with_id(2),
                    Pin::error_out().with_id(3),
                    Pin::data_out(PinKind::Any, "result").with_id(4),
                ],
            ),

            NodeKind::FunctionCall { .. } | NodeKind::MethodCall { .. } => (
                vec![
                    Pin::exec_in().with_id(1),
                    Pin::data_in(PinKind::Any, "args").with_id(5), // data input for variable args
                ],
                vec![
                    Pin::exec_out().with_id(2),
                    Pin::error_out().with_id(3),
                    Pin::data_out(PinKind::Any, "result").with_id(4),
                ],
            ),

            NodeKind::ExternalCall { .. } => (
                vec![Pin::exec_in().with_id(1)],
                vec![
                    Pin::exec_out().with_id(2),
                    Pin::error_out().with_id(3),
                    Pin::data_out(PinKind::Any, "response").with_id(4),
                ],
            ),

            NodeKind::Return { .. } => (vec![Pin::exec_in().with_id(1)], vec![]),

            NodeKind::Throw => (vec![Pin::exec_in().with_id(1)], vec![]),
        }
    }

    pub fn with_position(mut self, x: f64, y: f64) -> Self {
        self.position = Some(NodePosition { x, y });
        self
    }
}
