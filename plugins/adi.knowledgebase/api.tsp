/**
 * Knowledgebase HTTP API
 *
 * Graph-based knowledge management with semantic search and conflict detection.
 */

import "@typespec/http";

using TypeSpec.Http;

namespace Knowledgebase;

@format("uuid")
scalar uuid extends string;

// -- Enums --

enum NodeType {
  decision,
  fact,
  error,
  guide,
  glossary,
  context,
  assumption,
}

enum EdgeType {
  supersedes,
  contradicts,
  requires,
  relatedTo: "related_to",
  derivedFrom: "derived_from",
  answers,
}

// -- Models --

model Node {
  id: uuid;
  nodeType: NodeType;
  title: string;
  content: string;
  source: Record<unknown>;
  confidence: float32;
  createdAt: utcDateTime;
  updatedAt: utcDateTime;
  lastAccessedAt: utcDateTime;
  metadata: Record<unknown>;
}

model Edge {
  id: uuid;
  fromId: uuid;
  toId: uuid;
  edgeType: EdgeType;
  weight: float32;
  createdAt: utcDateTime;
  metadata: Record<unknown>;
}

model AddRequest {
  userSaid: string;
  derivedKnowledge: string;
  nodeType?: string;
}

model LinkRequest {
  fromId: uuid;
  toId: uuid;
  edgeType?: string;
  weight?: float32;
}

model SearchResult {
  node: Node;
  score: float32;
  edges: Edge[];
}

model Subgraph {
  nodes: Node[];
  edges: Edge[];
}

model ConflictPair {
  nodeA: uuid;
  nodeB: uuid;
}

model StatusResponse {
  initialized: boolean;
  dataDir: string;
  embeddings?: int32;
}

model DeletedResponse {
  deleted: uuid;
}

model ApprovedResponse {
  approved: uuid;
}

model QueryParams {
  @query q: string;
  @query limit?: int32;
}

// -- Interfaces --

interface KnowledgebaseService {
  @get
  @route("/status")
  getStatus(): {
    @statusCode statusCode: 200;
    @body body: StatusResponse;
  };

  @post
  @route("/nodes")
  addNode(@body body: AddRequest): {
    @statusCode statusCode: 201;
    @body body: Node;
  };

  @get
  @route("/nodes/{id}")
  getNode(@path id: uuid): {
    @statusCode statusCode: 200;
    @body body: Node;
  };

  @delete
  @route("/nodes/{id}")
  deleteNode(@path id: uuid): {
    @statusCode statusCode: 200;
    @body body: DeletedResponse;
  };

  @post
  @route("/nodes/{id}/approve")
  approveNode(@path id: uuid): {
    @statusCode statusCode: 200;
    @body body: ApprovedResponse;
  };

  @get
  @route("/query")
  query(...QueryParams): {
    @statusCode statusCode: 200;
    @body body: SearchResult[];
  };

  @get
  @route("/subgraph")
  subgraph(...QueryParams): {
    @statusCode statusCode: 200;
    @body body: Subgraph;
  };

  @get
  @route("/conflicts")
  getConflicts(): {
    @statusCode statusCode: 200;
    @body body: ConflictPair[];
  };

  @get
  @route("/orphans")
  getOrphans(): {
    @statusCode statusCode: 200;
    @body body: Node[];
  };

  @post
  @route("/edges")
  addEdge(@body body: LinkRequest): {
    @statusCode statusCode: 201;
    @body body: Edge;
  };
}
