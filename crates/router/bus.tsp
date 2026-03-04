// ADI Router — EventBus definitions
// Source of truth for code generation via tsp-gen eventbus codegen.

// ── EventBus: Router ──────────────────────────────────────

@bus("route")
interface RouteBus {
    @event register(path: string, element: string, label?: string): void;
}

@bus("nav")
interface NavBus {
    @event add(id: string, label: string, path: string, icon?: string): void;
}

@bus("router")
interface RouterBus {
    @event navigate(path: string, replace?: boolean): void;
    @event changed(path: string, params: Record<string>): void;
}

@bus("command")
interface CommandBus {
    @event register(id: string, label: string, shortcut?: string): void;
    @event execute(id: string): void;
}
