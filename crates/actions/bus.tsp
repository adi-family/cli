// ADI Actions — EventBus definitions
// Source of truth for code generation via tsp-gen eventbus codegen.

// ── Supporting Types ──────────────────────────────────────

enum ActionPriority {
    low: "low",
    normal: "normal",
    urgent: "urgent",
}

enum ActionKindMode {
    exclusive: "exclusive",
}

// ── EventBus: Actions ─────────────────────────────────────

@bus("actions")
interface ActionsBus {
    @event registerKind(plugin: string, kind: string, mode: ActionKindMode): void;
    @event push(id: string, plugin: string, kind: string, data: Record<unknown>, priority?: ActionPriority): void;
    @event dismiss(id: string): void;
    @event dismissed(id: string, plugin: string, kind: string): void;
}

// ── Shared buses (consumed by actions plugin) ─────────────

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
}

@bus("command")
interface CommandBus {
    @event register(id: string, label: string, shortcut?: string): void;
    @event execute(id: string): void;
}
