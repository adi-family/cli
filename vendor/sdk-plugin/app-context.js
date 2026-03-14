/** Shared context passed to every plugin — provides event bus and typed API access. */
export class AppContext {
    bus;
    apis = new Map();
    apiWaiters = new Map();
    _registeredPlugins = new Set();
    envSource;
    storageFactory;
    storageInstances = new Map();
    constructor(bus, options = {}) {
        this.bus = bus;
        this.envSource = options.envSource ?? {};
        this.storageFactory = options.storageFactory;
    }
    /** Read a comma-separated env variable by key (without the VITE_ prefix). */
    env(key) {
        return (this.envSource[`VITE_${key}`] ?? '').split(',').filter(Boolean);
    }
    /** Retrieve a typed plugin API, waiting for it to become available if not yet registered. */
    api(id) {
        const instance = this.apis.get(id);
        if (instance)
            return Promise.resolve(instance);
        return new Promise((resolve) => {
            const key = id;
            const waiters = this.apiWaiters.get(key) ?? [];
            waiters.push(resolve);
            this.apiWaiters.set(key, waiters);
        });
    }
    /** Per-plugin key-value storage backed by the app's storage implementation. */
    storage(pluginId) {
        const cached = this.storageInstances.get(pluginId);
        if (cached)
            return cached;
        if (!this.storageFactory) {
            throw new Error('Storage is not configured. Provide a storageFactory in AppContextOptions.');
        }
        const instance = this.storageFactory(pluginId);
        this.storageInstances.set(pluginId, instance);
        return instance;
    }
    /** IDs of all plugins that have completed _init(). */
    get registeredPlugins() {
        return this._registeredPlugins;
    }
    /** @internal Track a plugin as registered after _init(). */
    _registerPlugin(id) {
        this._registeredPlugins.add(id);
    }
    /** @internal Remove a plugin from the registered set. */
    _unregisterPlugin(id) {
        this._registeredPlugins.delete(id);
    }
    /** @internal Auto-provide from plugin._init(). */
    _provide(id, api) {
        if (this.apis.has(id)) {
            throw new Error(`API '${id}' is already registered.`);
        }
        this.apis.set(id, api);
        const waiters = this.apiWaiters.get(id);
        if (waiters) {
            this.apiWaiters.delete(id);
            for (const resolve of waiters)
                resolve(api);
        }
    }
    /** @internal Remove a provided API (used during plugin unregister). */
    _unprovide(id) {
        this.apis.delete(id);
    }
}
