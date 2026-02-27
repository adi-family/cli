export type NodeType = 'decision' | 'fact' | 'error' | 'guide' | 'glossary' | 'context' | 'assumption';

export type EdgeType = 'supersedes' | 'contradicts' | 'requires' | 'related_to' | 'derived_from' | 'answers';

export interface Node {
  id: string;
  cocoonId: string;
  node_type: NodeType;
  title: string;
  content: string;
  source: { User?: { statement: string }; Derived?: { interpretation: string; source_id: string | null } };
  confidence: { 0: number };
  created_at: string;
  updated_at: string;
  last_accessed_at: string;
  metadata: Record<string, unknown>;
}

export interface Edge {
  id: string;
  from_id: string;
  to_id: string;
  edge_type: EdgeType;
  weight: number;
  created_at: string;
  metadata: Record<string, unknown>;
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

export interface Connection {
  id: string;
  services: string[];
  request<T>(service: string, method: string, params?: unknown): Promise<T>;
  stream<T>(service: string, method: string, params?: unknown): AsyncGenerator<T>;
  httpProxy(service: string, path: string, init?: RequestInit): Promise<Response>;
  httpDirect(url: string, init?: RequestInit): Promise<Response>;
}
