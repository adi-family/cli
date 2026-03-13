@format("uuid")
scalar uuid extends string;

// ── Enums ──

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

enum ApprovalStatus {
  pending,
  approved,
  rejected,
}

enum AuditAction {
  create,
  update,
  delete,
  approve,
  reject,
}

// ── Models ──

model Node {
  id: uuid;
  node_type: NodeType;
  title: string;
  content: string;
  source: string;
  approval_status: ApprovalStatus;
  metadata: Record<unknown>;
  created_at: utcDateTime;
  updated_at: utcDateTime;
}

model Edge {
  id: uuid;
  from_id: uuid;
  to_id: uuid;
  edge_type: EdgeType;
  weight: float32;
  metadata: Record<unknown>;
  created_at: utcDateTime;
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
  node_a: Node;
  node_b: Node;
}

model AuditEntry {
  id: uuid;
  node_id: uuid;
  action: AuditAction;
  actor_source: string;
  actor_id?: string;
  details?: Record<unknown>;
  created_at: utcDateTime;
}

model NodeStats {
  total_nodes: int32;
  total_edges: int32;
  by_type: Record<int32>;
  by_status: Record<int32>;
  orphan_count: int32;
  conflict_count: int32;
}

model DeleteResult {
  deleted: boolean;
}

// ── AdiService ──

@channel("adi.knowledgebase")
interface KnowledgebaseService {
  // Node CRUD
  @request createNode(
    title: string,
    content: string,
    node_type: NodeType,
    source: string,
    metadata?: Record<unknown>,
  ): Node;

  @request getNode(id: uuid): Node;

  @request updateNode(
    id: uuid,
    title?: string,
    content?: string,
    node_type?: NodeType,
    metadata?: Record<unknown>,
  ): Node;

  @request deleteNode(id: uuid): DeleteResult;

  @request listNodes(
    node_type?: NodeType,
    approval_status?: ApprovalStatus,
    source?: string,
    limit?: int32,
    offset?: int32,
  ): Node[];

  // Approval workflow
  @request approveNode(id: uuid): Node;
  @request rejectNode(id: uuid, reason?: string): Node;
  @request listPending(limit?: int32): Node[];

  // Edge CRUD
  @request createEdge(
    from_id: uuid,
    to_id: uuid,
    edge_type: EdgeType,
    weight?: float32,
    metadata?: Record<unknown>,
  ): Edge;

  @request deleteEdge(id: uuid): DeleteResult;
  @request getEdges(node_id: uuid): Edge[];

  // Search
  @request search(query: string, limit?: int32, min_score?: float32): SearchResult[];
  @request getSubgraph(query: string, hops?: int32, limit?: int32): Subgraph;
  @request getNeighbors(node_id: uuid, hops?: int32): Subgraph;

  // Graph analysis
  @request getImpact(node_id: uuid, edge_types?: EdgeType[]): Subgraph;
  @request getConflicts(): ConflictPair[];
  @request getOrphans(): Node[];
  @request findDuplicates(content: string, threshold?: float32): SearchResult[];

  // Audit + Stats
  @request getAuditLog(node_id: uuid, limit?: int32): AuditEntry[];
  @request getStats(): NodeStats;
}

// ── Event Bus ──

model NodeWithCocoon {
  ...Node;
  cocoonId: string;
}

@bus("adi.knowledgebase")
interface KnowledgebaseBus {
  // Commands (UI -> plugin)
  @event listNodes(node_type?: NodeType, approval_status?: ApprovalStatus, source?: string): void;
  @event getNode(id: uuid, cocoonId: string): void;
  @event search(cocoonId: string, query: string, limit?: int32): void;
  @event approveNode(id: uuid, cocoonId: string): void;
  @event rejectNode(id: uuid, cocoonId: string, reason?: string): void;
  @event deleteNode(id: uuid, cocoonId: string): void;
  @event getConflicts(cocoonId: string): void;
  @event getOrphans(cocoonId: string): void;
  @event getStats(cocoonId: string): void;

  // Responses (plugin -> UI)
  @event nodesChanged(nodes: NodeWithCocoon[]): void;
  @event nodeDetail(node: NodeWithCocoon): void;
  @event searchResults(results: SearchResult[], cocoonId: string): void;
  @event nodeMutated(node: NodeWithCocoon): void;
  @event nodeDeleted(id: uuid, cocoonId: string): void;
  @event conflictsChanged(conflicts: ConflictPair[], cocoonId: string): void;
  @event orphansChanged(orphans: NodeWithCocoon[]): void;
  @event statsChanged(stats: NodeStats, cocoonId: string): void;
  @event error(message: string, event: string): void;
}
