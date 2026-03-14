// ADI Debug Screen — EventBus definitions
// Source of truth for code generation via tsp-gen eventbus codegen.

@bus("adi.debug-screen")
interface DebugPageBus {
    @event registerSection(pluginId: string, init: unknown, label: string): void;
}
