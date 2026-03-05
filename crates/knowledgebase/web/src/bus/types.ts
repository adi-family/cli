/**
 * Auto-generated eventbus types from TypeSpec.
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

export interface Node {
  id: string;
  nodeType: NodeType;
  title: string;
  content: string;
  source: Record<string, unknown>;
  confidence: number;
  createdAt: string;
  updatedAt: string;
  lastAccessedAt: string;
  metadata: Record<string, unknown>;
}

export interface Edge {
  id: string;
  fromId: string;
  toId: string;
  edgeType: EdgeType;
  weight: number;
  createdAt: string;
  metadata: Record<string, unknown>;
}

export interface AddRequest {
  userSaid: string;
  derivedKnowledge: string;
  nodeType?: string;
}

export interface LinkRequest {
  fromId: string;
  toId: string;
  edgeType?: string;
  weight?: number;
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
  nodeA: string;
  nodeB: string;
}

export interface StatusResponse {
  initialized: boolean;
  dataDir: string;
  embeddings?: number;
}

export interface DeletedResponse {
  deleted: string;
}

export interface ApprovedResponse {
  approved: string;
}

export interface QueryParams {
  q: string;
  limit?: number;
}
