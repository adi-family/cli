/**
 * Auto-generated ADI service client from TypeSpec.
 * DO NOT EDIT.
 */
import type { Connection } from '@adi-family/cocoon-plugin-interface';
import type { AuditEntry, ConflictPair, DeleteResult, Edge, Node, NodeStats, SearchResult, Subgraph, TagInfo } from './models.js';
import { ApprovalStatus, EdgeType, NodeType } from './enums.js';

const SVC_ADI_KNOWLEDGEBASE = 'adi.knowledgebase';

export const createNode = (c: Connection, params: { title: string; content: string; node_type: NodeType; source: string; metadata?: Record<string, unknown>; tags?: string[]; }) =>
  c.request<Node>(SVC_ADI_KNOWLEDGEBASE, 'create_node', params);

export const getNode = (c: Connection, id: string) =>
  c.request<Node>(SVC_ADI_KNOWLEDGEBASE, 'get_node', { id });

export const updateNode = (c: Connection, params: { id: string; title?: string; content?: string; node_type?: NodeType; metadata?: Record<string, unknown>; tags?: string[]; }) =>
  c.request<Node>(SVC_ADI_KNOWLEDGEBASE, 'update_node', params);

export const deleteNode = (c: Connection, id: string) =>
  c.request<DeleteResult>(SVC_ADI_KNOWLEDGEBASE, 'delete_node', { id });

export const listNodes = (c: Connection, params?: { node_type?: NodeType; approval_status?: ApprovalStatus; source?: string; tags?: string[]; limit?: number; offset?: number; }) =>
  c.request<Node[]>(SVC_ADI_KNOWLEDGEBASE, 'list_nodes', params ?? {});

export const approveNode = (c: Connection, id: string) =>
  c.request<Node>(SVC_ADI_KNOWLEDGEBASE, 'approve_node', { id });

export const rejectNode = (c: Connection, params: { id: string; reason?: string; }) =>
  c.request<Node>(SVC_ADI_KNOWLEDGEBASE, 'reject_node', params);

export const listPending = (c: Connection, params?: { limit?: number; }) =>
  c.request<Node[]>(SVC_ADI_KNOWLEDGEBASE, 'list_pending', params ?? {});

export const createEdge = (c: Connection, params: { from_id: string; to_id: string; edge_type: EdgeType; weight?: number; metadata?: Record<string, unknown>; }) =>
  c.request<Edge>(SVC_ADI_KNOWLEDGEBASE, 'create_edge', params);

export const deleteEdge = (c: Connection, id: string) =>
  c.request<DeleteResult>(SVC_ADI_KNOWLEDGEBASE, 'delete_edge', { id });

export const getEdges = (c: Connection, node_id: string) =>
  c.request<Edge[]>(SVC_ADI_KNOWLEDGEBASE, 'get_edges', { node_id });

export const search = (c: Connection, params: { query: string; limit?: number; min_score?: number; }) =>
  c.request<SearchResult[]>(SVC_ADI_KNOWLEDGEBASE, 'search', params);

export const getSubgraph = (c: Connection, params: { query: string; hops?: number; limit?: number; }) =>
  c.request<Subgraph>(SVC_ADI_KNOWLEDGEBASE, 'get_subgraph', params);

export const getNeighbors = (c: Connection, params: { node_id: string; hops?: number; }) =>
  c.request<Subgraph>(SVC_ADI_KNOWLEDGEBASE, 'get_neighbors', params);

export const getImpact = (c: Connection, params: { node_id: string; edge_types?: EdgeType[]; }) =>
  c.request<Subgraph>(SVC_ADI_KNOWLEDGEBASE, 'get_impact', params);

export const getConflicts = (c: Connection) =>
  c.request<ConflictPair[]>(SVC_ADI_KNOWLEDGEBASE, 'get_conflicts', {});

export const getOrphans = (c: Connection) =>
  c.request<Node[]>(SVC_ADI_KNOWLEDGEBASE, 'get_orphans', {});

export const findDuplicates = (c: Connection, params: { content: string; threshold?: number; }) =>
  c.request<SearchResult[]>(SVC_ADI_KNOWLEDGEBASE, 'find_duplicates', params);

export const getAuditLog = (c: Connection, params: { node_id: string; limit?: number; }) =>
  c.request<AuditEntry[]>(SVC_ADI_KNOWLEDGEBASE, 'get_audit_log', params);

export const getStats = (c: Connection) =>
  c.request<NodeStats>(SVC_ADI_KNOWLEDGEBASE, 'get_stats', {});

export const listTags = (c: Connection, params?: { limit?: number; }) =>
  c.request<TagInfo[]>(SVC_ADI_KNOWLEDGEBASE, 'list_tags', params ?? {});
