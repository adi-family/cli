/**
 * Auto-generated protocol messages from TypeSpec.
 * DO NOT EDIT.
 */

import type { ApprovalStatus, Edge, EdgeType, Node, NodeType } from './types';

export type SignalingMessage =
  // ── adi.knowledgebase ──
  | { type: 'adi.knowledgebase_create_node'; title: string; content: string; node_type: NodeType; source: string; metadata?: Record<string, unknown>; tags?: string[] }
  | { type: 'adi.knowledgebase_create_node_response'; id: string; node_type: NodeType; title: string; content: string; source: string; approval_status: ApprovalStatus; metadata: Record<string, unknown>; tags: string[]; created_at: string; updated_at: string }
  | { type: 'adi.knowledgebase_get_node'; id: string }
  | { type: 'adi.knowledgebase_get_node_response'; id: string; node_type: NodeType; title: string; content: string; source: string; approval_status: ApprovalStatus; metadata: Record<string, unknown>; tags: string[]; created_at: string; updated_at: string }
  | { type: 'adi.knowledgebase_update_node'; id: string; title?: string; content?: string; node_type?: NodeType; metadata?: Record<string, unknown>; tags?: string[] }
  | { type: 'adi.knowledgebase_update_node_response'; id: string; node_type: NodeType; title: string; content: string; source: string; approval_status: ApprovalStatus; metadata: Record<string, unknown>; tags: string[]; created_at: string; updated_at: string }
  | { type: 'adi.knowledgebase_delete_node'; id: string }
  | { type: 'adi.knowledgebase_delete_node_response'; deleted: boolean }
  | { type: 'adi.knowledgebase_list_nodes'; node_type?: NodeType; approval_status?: ApprovalStatus; source?: string; tags?: string[]; limit?: number; offset?: number }
  | { type: 'adi.knowledgebase_approve_node'; id: string }
  | { type: 'adi.knowledgebase_approve_node_response'; id: string; node_type: NodeType; title: string; content: string; source: string; approval_status: ApprovalStatus; metadata: Record<string, unknown>; tags: string[]; created_at: string; updated_at: string }
  | { type: 'adi.knowledgebase_reject_node'; id: string; reason?: string }
  | { type: 'adi.knowledgebase_reject_node_response'; id: string; node_type: NodeType; title: string; content: string; source: string; approval_status: ApprovalStatus; metadata: Record<string, unknown>; tags: string[]; created_at: string; updated_at: string }
  | { type: 'adi.knowledgebase_list_pending'; limit?: number }
  | { type: 'adi.knowledgebase_create_edge'; from_id: string; to_id: string; edge_type: EdgeType; weight?: number; metadata?: Record<string, unknown> }
  | { type: 'adi.knowledgebase_create_edge_response'; id: string; from_id: string; to_id: string; edge_type: EdgeType; weight: number; metadata: Record<string, unknown>; created_at: string }
  | { type: 'adi.knowledgebase_delete_edge'; id: string }
  | { type: 'adi.knowledgebase_delete_edge_response'; deleted: boolean }
  | { type: 'adi.knowledgebase_get_edges'; node_id: string }
  | { type: 'adi.knowledgebase_search'; query: string; limit?: number; min_score?: number }
  | { type: 'adi.knowledgebase_get_subgraph'; query: string; hops?: number; limit?: number }
  | { type: 'adi.knowledgebase_get_subgraph_response'; nodes: Node[]; edges: Edge[] }
  | { type: 'adi.knowledgebase_get_neighbors'; node_id: string; hops?: number }
  | { type: 'adi.knowledgebase_get_neighbors_response'; nodes: Node[]; edges: Edge[] }
  | { type: 'adi.knowledgebase_get_impact'; node_id: string; edge_types?: EdgeType[] }
  | { type: 'adi.knowledgebase_get_impact_response'; nodes: Node[]; edges: Edge[] }
  | { type: 'adi.knowledgebase_get_conflicts' }
  | { type: 'adi.knowledgebase_get_orphans' }
  | { type: 'adi.knowledgebase_find_duplicates'; content: string; threshold?: number }
  | { type: 'adi.knowledgebase_get_audit_log'; node_id: string; limit?: number }
  | { type: 'adi.knowledgebase_get_stats' }
  | { type: 'adi.knowledgebase_get_stats_response'; total_nodes: number; total_edges: number; by_type: Record<string, number>; by_status: Record<string, number>; orphan_count: number; conflict_count: number }
  | { type: 'adi.knowledgebase_list_tags'; limit?: number };
