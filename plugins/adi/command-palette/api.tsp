// ADI Command Palette — EventBus definitions
// Source of truth for code generation via tsp-gen eventbus codegen.

// ── EventBus: Command ──────────────────────────────────────

@bus("command")
interface CommandBus {
    @event register(id: string, label: string, shortcut?: string): void;
    @event execute(id: string): void;
}

// ── EventBus: Command Palette ──────────────────────────────

@bus("command-palette")
interface CommandPaletteBus {
    @event open(query?: string): void;
}
