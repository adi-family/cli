use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeType {
    Decision,
    Fact,
    Error,
    Guide,
    Glossary,
    Context,
    Assumption,
}

impl NodeType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Decision => "decision",
            Self::Fact => "fact",
            Self::Error => "error",
            Self::Guide => "guide",
            Self::Glossary => "glossary",
            Self::Context => "context",
            Self::Assumption => "assumption",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "decision" => Self::Decision,
            "fact" => Self::Fact,
            "error" => Self::Error,
            "guide" => Self::Guide,
            "glossary" => Self::Glossary,
            "context" => Self::Context,
            "assumption" => Self::Assumption,
            other => panic!("unknown NodeType: {other}"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EdgeType {
    Supersedes,
    Contradicts,
    Requires,
    RelatedTo,
    DerivedFrom,
    Answers,
}

impl EdgeType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Supersedes => "supersedes",
            Self::Contradicts => "contradicts",
            Self::Requires => "requires",
            Self::RelatedTo => "related_to",
            Self::DerivedFrom => "derived_from",
            Self::Answers => "answers",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "supersedes" => Self::Supersedes,
            "contradicts" => Self::Contradicts,
            "requires" => Self::Requires,
            "related_to" => Self::RelatedTo,
            "derived_from" => Self::DerivedFrom,
            "answers" => Self::Answers,
            other => panic!("unknown EdgeType: {other}"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalStatus {
    Pending,
    Approved,
    Rejected,
}

impl ApprovalStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Approved => "approved",
            Self::Rejected => "rejected",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "pending" => Self::Pending,
            "approved" => Self::Approved,
            "rejected" => Self::Rejected,
            other => panic!("unknown ApprovalStatus: {other}"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditAction {
    Create,
    Update,
    Delete,
    Approve,
    Reject,
}

impl AuditAction {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Create => "create",
            Self::Update => "update",
            Self::Delete => "delete",
            Self::Approve => "approve",
            Self::Reject => "reject",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub id: Uuid,
    pub node_type: NodeType,
    pub title: String,
    pub content: String,
    pub source: String,
    pub approval_status: ApprovalStatus,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Node {
    pub fn embedding_content(&self) -> String {
        format!("{} {}", self.title, self.content)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    pub id: Uuid,
    pub from_id: Uuid,
    pub to_id: Uuid,
    pub edge_type: EdgeType,
    pub weight: f32,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub node: Node,
    pub score: f32,
    pub edges: Vec<Edge>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subgraph {
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
}

impl Subgraph {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            edges: Vec::new(),
        }
    }

    pub fn add_node(&mut self, node: Node) {
        if !self.nodes.iter().any(|n| n.id == node.id) {
            self.nodes.push(node);
        }
    }

    pub fn add_edge(&mut self, edge: Edge) {
        if !self.edges.iter().any(|e| e.id == edge.id) {
            self.edges.push(edge);
        }
    }
}

impl Default for Subgraph {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictPair {
    pub node_a: Node,
    pub node_b: Node,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub id: Uuid,
    pub node_id: Uuid,
    pub action: AuditAction,
    pub actor_source: String,
    pub actor_id: Option<String>,
    pub details: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeStats {
    pub total_nodes: i32,
    pub total_edges: i32,
    pub by_type: std::collections::HashMap<String, i32>,
    pub by_status: std::collections::HashMap<String, i32>,
    pub orphan_count: i32,
    pub conflict_count: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteResult {
    pub deleted: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// FIX CHECK: unknown NodeType strings must NOT silently become Fact.
    /// Silent fallback corrupts data when encountering unknown types
    /// from newer schema versions or typos. Fix: return Result or panic.
    #[test]
    #[should_panic]
    fn unknown_node_type_must_not_be_silent() {
        // This must panic (or from_str should return Result/Option).
        // Currently it silently returns Fact, which is a data corruption bug.
        let _ = NodeType::from_str("tutorial");
    }

    /// FIX CHECK: unknown EdgeType strings must NOT silently become RelatedTo.
    /// Silent fallback changes graph traversal semantics.
    #[test]
    #[should_panic]
    fn unknown_edge_type_must_not_be_silent() {
        let _ = EdgeType::from_str("depends_on");
    }

    /// FIX CHECK: unknown ApprovalStatus strings must NOT silently become Pending.
    /// Silent fallback re-enters rejected items into the approval workflow.
    #[test]
    #[should_panic]
    fn unknown_approval_status_must_not_be_silent() {
        let _ = ApprovalStatus::from_str("archived");
    }
}
