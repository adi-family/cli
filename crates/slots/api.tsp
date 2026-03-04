@bus("slots")
interface SlotsBus {
    @event define(id: string, multiple?: boolean): void;
    @event place(slot: string, element: string, priority?: int32, pluginId: string): void;
    @event remove(slot: string, element: string): void;
    @event removeAll(pluginId: string): void;
    @event changed(slot: string): void;
}
