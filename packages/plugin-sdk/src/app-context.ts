import type { EventBus } from './bus.js';
import type { PluginApiRegistry } from './types.js';
import type { PluginStorage, StorageFactory } from './storage.js';

export interface AppContextOptions {
  envSource?: Record<string, string | undefined>;
  storageFactory?: StorageFactory;
}

/** Shared context passed to every plugin — provides event bus and typed API access. */
export class AppContext {
  readonly bus: EventBus;
  private readonly apis = new Map<string, unknown>();
  private readonly apiWaiters = new Map<string, Array<(api: unknown) => void>>();
  private readonly _registeredPlugins = new Set<string>();
  private readonly envSource: Record<string, string | undefined>;
  private readonly storageFactory?: StorageFactory;
  private readonly storageInstances = new Map<string, PluginStorage>();

  constructor(bus: EventBus, options: AppContextOptions = {}) {
    this.bus = bus;
    this.envSource = options.envSource ?? {};
    this.storageFactory = options.storageFactory;
  }

  /** Read a comma-separated env variable by key (without the VITE_ prefix). */
  env(key: string): string[] {
    return (this.envSource[`VITE_${key}`] ?? '').split(',').filter(Boolean);
  }

  /** Retrieve a typed plugin API by its registered key. Throws if not yet available. */
  api<K extends keyof PluginApiRegistry>(id: K): PluginApiRegistry[K] {
    const instance = this.apis.get(id as string);
    if (!instance) {
      throw new Error(`API '${String(id)}' is not registered. Ensure the plugin is loaded and calls app.provide().`);
    }
    return instance as PluginApiRegistry[K];
  }

  /** Retrieve a typed plugin API, waiting for it to become available if not yet registered. */
  apiReady<K extends keyof PluginApiRegistry>(id: K): Promise<PluginApiRegistry[K]> {
    const instance = this.apis.get(id as string);
    if (instance) return Promise.resolve(instance as PluginApiRegistry[K]);

    return new Promise<PluginApiRegistry[K]>((resolve) => {
      const key = id as string;
      const waiters = this.apiWaiters.get(key) ?? [];
      waiters.push(resolve as (api: unknown) => void);
      this.apiWaiters.set(key, waiters);
    });
  }

  /** Per-plugin key-value storage backed by the app's storage implementation. */
  storage(pluginId: string): PluginStorage {
    const cached = this.storageInstances.get(pluginId);
    if (cached) return cached;

    if (!this.storageFactory) {
      throw new Error('Storage is not configured. Provide a storageFactory in AppContextOptions.');
    }

    const instance = this.storageFactory(pluginId);
    this.storageInstances.set(pluginId, instance);
    return instance;
  }

  /** IDs of all plugins that have completed _init(). */
  get registeredPlugins(): ReadonlySet<string> {
    return this._registeredPlugins;
  }

  /** @internal Track a plugin as registered after _init(). */
  _registerPlugin(id: string): void {
    this._registeredPlugins.add(id);
  }

  /** @internal Remove a plugin from the registered set. */
  _unregisterPlugin(id: string): void {
    this._registeredPlugins.delete(id);
  }

  /** @internal Auto-provide from plugin._init(). */
  _provide(id: string, api: unknown): void {
    if (this.apis.has(id)) {
      throw new Error(`API '${id}' is already registered.`);
    }
    this.apis.set(id, api);

    const waiters = this.apiWaiters.get(id);
    if (waiters) {
      this.apiWaiters.delete(id);
      for (const resolve of waiters) resolve(api);
    }
  }

  /** @internal Remove a provided API (used during plugin unregister). */
  _unprovide(id: string): void {
    this.apis.delete(id);
  }
}
