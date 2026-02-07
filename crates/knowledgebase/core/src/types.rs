use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Confidence level for knowledge nodes
/// - 1.0 = Explicitly approved by user
/// - 0.8-0.99 = Strong evidence, multiple sources agree
/// - 0.5-0.79 = Reasonable inference, single source
/// - 0.0-0.49 = Weak inference, needs validation
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Confidence(pub f32);

impl Confidence {
    pub const APPROVED: Self = Self(1.0);
    pub const STRONG: Self = Self(0.9);
    pub const MEDIUM: Self = Self(0.7);
    pub const WEAK: Self = Self(0.3);

    pub fn new(value: f32) -> Self {
        Self(value.clamp(0.0, 1.0))
    }

    pub fn is_approved(&self) -> bool {
        self.0 >= 1.0
    }

    pub fn is_assumption(&self) -> bool {
        self.0 < 1.0
    }
}

impl Default for Confidence {
    fn default() -> Self {
        Self::MEDIUM
    }
}

/// Source of knowledge - either user input or AI-derived
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum KnowledgeSource {
    /// Exact user statement (verbatim, immutable)
    User { statement: String },
    /// AI-derived interpretation
    Derived {
        interpretation: String,
        source_id: Option<Uuid>,
    },
}

/// Types of knowledge nodes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeType {
    /// Architectural/product choices with rationale
    Decision,
    /// Immutable truths, definitions
    Fact,
    /// Known issues with causes and fixes
    Error,
    /// Procedural how-to knowledge
    Guide,
    /// Term definitions
    Glossary,
    /// When/where knowledge applies
    Context,
    /// Unvalidated beliefs flagged for verification
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
}

/// Types of edges between nodes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EdgeType {
    /// Version chain (new replaces old)
    Supersedes,
    /// Conflict marker (requires resolution)
    Contradicts,
    /// Dependency (A needs B)
    Requires,
    /// Weak association
    RelatedTo,
    /// Source reference
    DerivedFrom,
    /// Maps questions to knowledge
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
}

/// Conflict resolution status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConflictStatus {
    Detected,
    Pending,
    Resolved,
    Escalated,
}

/// Clarification status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClarificationStatus {
    Clear,
    Unclear,
    Pending,
    Clarified,
    Blocked,
}

/// A knowledge node in the graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub id: Uuid,
    pub node_type: NodeType,
    pub title: String,
    pub content: String,
    pub source: KnowledgeSource,
    pub confidence: Confidence,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_accessed_at: DateTime<Utc>,
    pub metadata: serde_json::Value,
}

impl Node {
    pub fn new(
        node_type: NodeType,
        title: String,
        content: String,
        source: KnowledgeSource,
        confidence: Confidence,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            node_type,
            title,
            content,
            source,
            confidence,
            created_at: now,
            updated_at: now,
            last_accessed_at: now,
            metadata: serde_json::json!({}),
        }
    }

    /// Content for embedding generation
    pub fn embedding_content(&self) -> String {
        format!("{} {}", self.title, self.content)
    }
}

/// An edge connecting two nodes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    pub id: Uuid,
    pub from_id: Uuid,
    pub to_id: Uuid,
    pub edge_type: EdgeType,
    pub weight: f32,
    pub created_at: DateTime<Utc>,
    pub metadata: serde_json::Value,
}

impl Edge {
    pub fn new(from_id: Uuid, to_id: Uuid, edge_type: EdgeType, weight: f32) -> Self {
        Self {
            id: Uuid::new_v4(),
            from_id,
            to_id,
            edge_type,
            weight: weight.clamp(0.0, 1.0),
            created_at: Utc::now(),
            metadata: serde_json::json!({}),
        }
    }
}

/// Query result with relevance score
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub node: Node,
    pub score: f32,
    pub edges: Vec<Edge>,
}

/// Subgraph for agent consumption
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

/// Clarification request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClarificationRequest {
    pub id: Uuid,
    pub node_id: Uuid,
    pub status: ClarificationStatus,
    pub aspect: ClarificationAspect,
    pub created_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
}

/// Type of clarification needed
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClarificationAspect {
    /// "When does this apply?"
    ScopeUnclear,
    /// "What exactly is X?"
    DefinitionVague,
    /// "Why was this decided?"
    MissingContext,
    /// "Is this still true?"
    OutdatedSuspicion,
    /// "Which one is correct?"
    ConflictUnresolved,
}

/// Conflict between nodes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conflict {
    pub id: Uuid,
    pub node_a: Uuid,
    pub node_b: Uuid,
    pub status: ConflictStatus,
    pub description: String,
    pub created_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
}
