@bus("slots")
interface SlotsBus {
    @event define(id: string, multiple?: boolean): void;
    @event place(slot: string, elementRef: unknown, priority?: int32, pluginId: string): void;
    @event remove(slot: string, elementRef: unknown): void;
    @event removeAll(pluginId: string): void;
    @event changed(slot: string): void;
}
