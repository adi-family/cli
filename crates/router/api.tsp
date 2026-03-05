// ADI Router — EventBus definitions
// Source of truth for code generation via tsp-gen eventbus codegen.

// ── EventBus: Router ──────────────────────────────────────

@bus("adi.router")
interface RouterBus {
    @event navigate(path: string, replace?: boolean): void;
    @event changed(path: string, params: Record<string>): void;
    @event registerRoute(pluginId: string, path: string, element: string, label?: string): void;
}
