/**
 * Auto-generated protocol types from TypeSpec.
 * DO NOT EDIT.
 */

export enum NodeType {
  Decision = "decision",
  Fact = "fact",
  Error = "error",
  Guide = "guide",
  Glossary = "glossary",
  Context = "context",
  Assumption = "assumption",
}

export enum EdgeType {
  Supersedes = "supersedes",
  Contradicts = "contradicts",
  Requires = "requires",
  RelatedTo = "related_to",
  DerivedFrom = "derived_from",
  Answers = "answers",
}

export enum ApprovalStatus {
  Pending = "pending",
  Approved = "approved",
  Rejected = "rejected",
}

export enum AuditAction {
  Create = "create",
  Update = "update",
  Delete = "delete",
  Approve = "approve",
  Reject = "reject",
}

export interface Node {
  id: string;
  node_type: NodeType;
  title: string;
  content: string;
  source: string;
  approval_status: ApprovalStatus;
  metadata: Record<string, unknown>;
  tags: string[];
  created_at: string;
  updated_at: string;
}

export interface Edge {
  id: string;
  from_id: string;
  to_id: string;
  edge_type: EdgeType;
  weight: number;
  metadata: Record<string, unknown>;
  created_at: string;
}

export interface SearchResult {
  node: Node;
  score: number;
  edges: Edge[];
}

export interface Subgraph {
  nodes: Node[];
  edges: Edge[];
}

export interface ConflictPair {
  node_a: Node;
  node_b: Node;
}

export interface AuditEntry {
  id: string;
  node_id: string;
  action: AuditAction;
  actor_source: string;
  actor_id?: string;
  details?: Record<string, unknown>;
  created_at: string;
}

export interface NodeStats {
  total_nodes: number;
  total_edges: number;
  by_type: Record<string, number>;
  by_status: Record<string, number>;
  orphan_count: number;
  conflict_count: number;
}

export interface TagInfo {
  tag: string;
  count: number;
}

export interface DeleteResult {
  deleted: boolean;
}

export interface NodeWithCocoon {
  id: string;
  node_type: NodeType;
  title: string;
  content: string;
  source: string;
  approval_status: ApprovalStatus;
  metadata: Record<string, unknown>;
  tags: string[];
  created_at: string;
  updated_at: string;
  cocoonId: string;
}
