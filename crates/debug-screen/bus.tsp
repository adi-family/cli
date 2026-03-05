// ADI Debug Screen — EventBus definitions
// Source of truth for code generation via tsp-gen eventbus codegen.

// ── Shared buses (consumed by debug-screen plugin) ────────

@bus("command")
interface CommandBus {
    @event register(id: string, label: string, shortcut?: string): void;
    @event execute(id: string): void;
}

