// ADI Knowledgebase — EventBus definitions
// Source of truth for code generation via tsp-gen eventbus codegen.

// ── EventBus: Knowledgebase ──────────────────────────────────

@bus("adi.knowledgebase")
interface KnowledgebaseBus {
    // Commands (UI → plugin)
    @event query(q: string, limit?: int32): void;
    @event add(user_said: string, derived_knowledge: string, node_type?: string, cocoonId: string): void;
    @event get(id: string, cocoonId: string): void;
    @event delete(id: string, cocoonId: string): void;
    @event approve(id: string, cocoonId: string): void;
    @event conflicts(): void;
    @event orphans(): void;
    @event link(from_id: string, to_id: string, edge_type?: string, weight?: float32, cocoonId: string): void;

    // Responses (plugin → UI)
    @event resultsChanged(results: unknown[]): void;
    @event nodeChanged(node: unknown, edges: unknown[]): void;
    @event nodeDeleted(id: string, cocoonId: string): void;
    @event conflictsChanged(conflicts: unknown[]): void;
    @event orphansChanged(nodes: unknown[]): void;
}
