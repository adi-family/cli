import type { EventBus } from './bus.js';
import type { PluginApiRegistry } from './types.js';

/** Shared context passed to every plugin — provides event bus and typed API access. */
export class AppContext {
  readonly bus: EventBus;
  private readonly apis = new Map<string, unknown>();

  constructor(bus: EventBus) {
    this.bus = bus;
  }

  /** Retrieve a typed plugin API by its registered key. */
  api<K extends keyof PluginApiRegistry>(id: K): PluginApiRegistry[K] {
    const instance = this.apis.get(id as string);
    if (!instance) {
      throw new Error(`API '${String(id)}' is not registered. Ensure the plugin is loaded and calls app.provide().`);
    }
    return instance as PluginApiRegistry[K];
  }

  /** @internal Auto-provide from plugin._init(). */
  _provide(id: string, api: unknown): void {
    if (this.apis.has(id)) {
      throw new Error(`API '${id}' is already registered.`);
    }
    this.apis.set(id, api);
  }

  /** @internal Remove a provided API (used during plugin unregister). */
  _unprovide(id: string): void {
    this.apis.delete(id);
  }
}
