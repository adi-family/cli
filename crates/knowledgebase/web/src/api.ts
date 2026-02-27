import type { Connection, Node, Edge, SearchResult, ConflictPair } from './types.js';

const SVC = 'kb';

export const query = (c: Connection, q: string, limit?: number) =>
  c.request<SearchResult[]>(SVC, 'query', { q, limit });

export const querySubgraph = (c: Connection, q: string) =>
  c.request<{ nodes: Node[]; edges: Edge[] }>(SVC, 'subgraph', { q });

export const addNode = (c: Connection, params: { user_said: string; derived_knowledge: string; node_type?: string }) =>
  c.request<Node>(SVC, 'add', params);

export const getNode = (c: Connection, id: string) =>
  c.request<Node>(SVC, 'get', { id });

export const deleteNode = (c: Connection, id: string) =>
  c.request<{ deleted: boolean }>(SVC, 'delete', { id });

export const approveNode = (c: Connection, id: string) =>
  c.request<{ approved: boolean }>(SVC, 'approve', { id });

export const getConflicts = (c: Connection) =>
  c.request<ConflictPair[]>(SVC, 'conflicts', {});

export const getOrphans = (c: Connection) =>
  c.request<Node[]>(SVC, 'orphans', {});

export const addEdge = (c: Connection, params: { from_id: string; to_id: string; edge_type?: string; weight?: number }) =>
  c.request<Edge>(SVC, 'link', params);

export const getStatus = (c: Connection) =>
  c.request<{ initialized: boolean; data_dir: string; embedding_count: number }>(SVC, 'status', {});
