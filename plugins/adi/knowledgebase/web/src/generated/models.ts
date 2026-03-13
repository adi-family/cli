/**
 * Auto-generated models from TypeSpec.
 * DO NOT EDIT.
 */

import { NodeType, EdgeType, ApprovalStatus, AuditAction } from './enums';

export interface Node {
  id: string;
  node_type: NodeType;
  title: string;
  content: string;
  source: string;
  approval_status: ApprovalStatus;
  metadata: Record<string, unknown>;
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
  created_at: string;
  updated_at: string;
  cocoonId: string;
}
