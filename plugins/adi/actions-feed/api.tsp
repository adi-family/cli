enum ActionPriority {
    low: "low",
    normal: "normal",
    urgent: "urgent",
}

enum ActionKindMode {
    exclusive: "exclusive",
}

@bus("actions")
interface ActionsBus {
    @event registerKind(plugin: string, kind: string, mode: ActionKindMode): void;
    @event registerRenderer(plugin: string, kind: string, render: unknown): void;
    @event push(id: string, plugin: string, kind: string, data: Record<unknown>, priority?: ActionPriority): void;
    @event dismiss(id: string): void;
    @event dismissed(id: string, plugin: string, kind: string): void;
}
