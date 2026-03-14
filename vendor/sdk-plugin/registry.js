import { AppContext } from './app-context.js';
import { AdiPlugin } from './plugin.js';
const registry = new Map();
const descriptors = new Map();
let sharedApp;
let appOptions = {};
export function registerPlugin(plugin) {
    registry.set(plugin.id, plugin);
}
/** Configure AppContext options before plugins are loaded. */
export function configureApp(options) {
    if (sharedApp) {
        throw new Error('configureApp() must be called before any plugins are loaded.');
    }
    appOptions = { ...appOptions, ...options };
}
/** @internal Test helper. */
export function _resetRegistry() {
    registry.clear();
    descriptors.clear();
    sharedApp = undefined;
    appOptions = {};
    swMessageController?.abort();
    swMessageController = undefined;
}
function getApp(bus) {
    if (!sharedApp)
        sharedApp = new AppContext(bus, { envSource: import.meta.env, ...appOptions });
    return sharedApp;
}
export async function loadPlugins(bus, pluginDescriptors, options = {}) {
    const timeout = options.timeout ?? 5000;
    for (const d of pluginDescriptors) {
        descriptors.set(d.id, d);
    }
    const importResults = await Promise.allSettled(pluginDescriptors.map((d) => fetchAndImport(d)));
    for (let i = 0; i < importResults.length; i++) {
        const r = importResults[i];
        if (r.status === 'rejected') {
            console.error(`[plugin] failed to load '${pluginDescriptors[i].id}':`, r.reason);
        }
    }
    // Resolve `requires`: auto-fetch missing required plugins from availablePlugins.
    const available = new Map((options.availablePlugins ?? []).map((d) => [d.id, d]));
    const autoInstalled = await resolveRequires(bus, available);
    const plugins = [...registry.values()];
    const requiresEdges = collectRequiresEdges(plugins);
    const { order, cycled } = topoSort(plugins, requiresEdges);
    const loaded = [];
    const failed = [...cycled];
    const timedOut = [];
    const allDescriptors = [...pluginDescriptors, ...autoInstalled];
    const registeredIds = new Set(plugins.map((p) => p.id));
    for (const d of allDescriptors) {
        if (!registeredIds.has(d.id)) {
            console.error(`[plugin] '${d.id}' bundle loaded but did not register — export your plugin class as PluginShell: export { MyPlugin as PluginShell }`);
            failed.push(d.id);
        }
    }
    const app = getApp(bus);
    for (const plugin of order) {
        const result = await initWithTimeout(plugin, app, timeout);
        if (result === 'ok')
            loaded.push(plugin.id);
        else if (result === 'timeout') {
            console.error(`[plugin] '${plugin.id}' timed out during onRegister (>${timeout}ms)`);
            timedOut.push(plugin.id);
        }
        else {
            console.error(`[plugin] '${plugin.id}' threw during onRegister:`, result.error);
            failed.push(plugin.id);
        }
    }
    // Phase 4: Background update checks (non-blocking).
    void checkForUpdates(bus, allDescriptors);
    // Phase 5: Signal completion.
    bus.emit('loading-finished', { loaded, failed, timedOut }, 'plugin-registry');
}
export async function initInternalPlugin(bus, plugin) {
    registerPlugin(plugin);
    await plugin._init(getApp(bus));
}
export async function upgradePlugin(bus, descriptor, options = {}) {
    const timeout = options.timeout ?? 5000;
    const { id, installedVersion } = descriptor;
    const existing = registry.get(id);
    const fromVersion = existing?.version ?? 'unknown';
    bus.emit('plugin:upgrading', { pluginId: id, fromVersion, toVersion: installedVersion }, 'plugin-registry');
    try {
        if (existing) {
            await existing._destroy();
            registry.delete(id);
        }
        // Load new version — module calls registerPlugin() as side effect.
        await fetchAndImport(descriptor);
        const newPlugin = registry.get(id);
        if (!newPlugin)
            throw new Error(`Plugin ${id} did not call registerPlugin()`);
        const result = await initWithTimeout(newPlugin, getApp(bus), timeout);
        if (result === 'timeout') {
            throw new Error(`Plugin ${id} timed out during upgrade`);
        }
        else if (typeof result === 'object') {
            throw new Error(`Plugin ${id} errored during upgrade: ${result.error instanceof Error ? result.error.message : String(result.error)}`);
        }
        descriptors.set(id, descriptor);
        bus.emit('plugin:upgraded', { pluginId: id, fromVersion, toVersion: installedVersion }, 'plugin-registry');
    }
    catch (err) {
        bus.emit('plugin:upgrade-failed', {
            pluginId: id,
            reason: err instanceof Error ? err.message : String(err),
        }, 'plugin-registry');
    }
}
let swMessageController;
export async function registerPluginSW(swUrl, bus) {
    if (!('serviceWorker' in navigator))
        return;
    swMessageController?.abort();
    swMessageController = new AbortController();
    const reg = await navigator.serviceWorker.register(swUrl, { type: 'module' });
    navigator.serviceWorker.addEventListener('message', (event) => {
        const data = event.data;
        if (data?.type !== 'plugin:bundle-updated')
            return;
        for (const [id, descriptor] of descriptors) {
            descriptor.registry
                .getBundleInfo(id, descriptor.installedVersion)
                .then((info) => {
                if (info.jsUrl !== data.url)
                    return;
                descriptor.registry
                    .checkLatest(id, descriptor.installedVersion)
                    .then((latest) => {
                    if (!latest)
                        return;
                    bus.emit('plugin:update-available', {
                        pluginId: id,
                        currentVersion: descriptor.installedVersion,
                        newVersion: latest.version,
                        newUrl: data.url,
                    }, 'plugin-sw');
                })
                    .catch(() => null);
            })
                .catch(() => null);
        }
    }, { signal: swMessageController.signal });
    await reg.update().catch(() => null);
}
async function checkForUpdates(bus, pluginDescriptors) {
    await Promise.allSettled(pluginDescriptors.map(async ({ id, registry: reg, installedVersion }) => {
        const result = await reg.checkLatest(id, installedVersion).catch(() => null);
        if (!result)
            return;
        const info = await reg.getBundleInfo(id, result.version).catch(() => null);
        if (!info)
            return;
        bus.emit('plugin:update-available', {
            pluginId: id,
            currentVersion: installedVersion,
            newVersion: result.version,
            newUrl: info.jsUrl,
        }, 'plugin-registry');
    }));
}
async function initWithTimeout(plugin, app, timeoutMs) {
    return new Promise((resolve) => {
        const timer = setTimeout(() => resolve('timeout'), timeoutMs);
        plugin
            ._init(app)
            .then(() => { clearTimeout(timer); resolve('ok'); })
            .catch((err) => { clearTimeout(timer); resolve({ error: err }); });
    });
}
async function fetchAndImport(descriptor) {
    const bundleInfo = await descriptor.registry.getBundleInfo(descriptor.id, descriptor.installedVersion);
    const fetches = [fetch(bundleInfo.jsUrl)];
    if (bundleInfo.cssUrl) {
        fetches.push(fetch(bundleInfo.cssUrl).catch(() => null));
    }
    const [res, cssRes] = await Promise.all(fetches);
    if (!res?.ok) {
        throw new Error(`Failed to fetch plugin bundle: ${res?.status} ${res?.statusText} (${bundleInfo.jsUrl})`);
    }
    if (cssRes?.ok) {
        const cssText = await cssRes.text();
        if (cssText) {
            const style = document.createElement('style');
            style.setAttribute('data-plugin', descriptor.id);
            style.textContent = cssText;
            document.head.appendChild(style);
        }
    }
    const blob = await res.blob();
    const blobUrl = URL.createObjectURL(blob);
    try {
        const mod = await import(/* @vite-ignore */ blobUrl);
        if ('PluginShell' in mod) {
            if (typeof mod.PluginShell !== 'function') {
                throw new Error(`PluginShell export must be a class, got ${typeof mod.PluginShell}`);
            }
            const instance = new mod.PluginShell();
            if (typeof instance !== 'object' || instance === null ||
                typeof instance['id'] !== 'string' ||
                typeof instance['_init'] !== 'function' ||
                typeof instance['_destroy'] !== 'function') {
                throw new Error(`PluginShell must extend AdiPlugin`);
            }
            registerPlugin(instance);
        }
    }
    finally {
        URL.revokeObjectURL(blobUrl);
    }
}
/** Walk registered plugins' `requires`, fetch missing ones from availablePlugins. */
async function resolveRequires(bus, available) {
    const installed = [];
    const seen = new Set();
    let frontier = [...registry.values()].flatMap((p) => p.requires ?? []);
    while (frontier.length > 0) {
        const next = [];
        for (const reqId of frontier) {
            if (seen.has(reqId) || registry.has(reqId))
                continue;
            seen.add(reqId);
            const desc = available.get(reqId);
            if (!desc) {
                console.error(`[plugin] required plugin '${reqId}' not found in availablePlugins`);
                continue;
            }
            try {
                await fetchAndImport(desc);
                descriptors.set(desc.id, desc);
                installed.push(desc);
                bus.emit('plugin:installed', { pluginId: reqId, reason: 'auto' }, 'plugin-registry');
                const newPlugin = registry.get(reqId);
                if (newPlugin) {
                    next.push(...(newPlugin.requires ?? []));
                }
            }
            catch (err) {
                console.error(`[plugin] failed to auto-install required plugin '${reqId}':`, err);
            }
        }
        frontier = next;
    }
    return installed;
}
/** Collect dependency edges from `requires` (same direction as `dependencies`). */
function collectRequiresEdges(plugins) {
    return plugins.flatMap((p) => (p.requires ?? []).map((req) => [req, p.id]));
}
/** Kahn's algorithm topological sort. */
function topoSort(plugins, extraEdges = []) {
    const ids = new Set(plugins.map((p) => p.id));
    const inDegree = new Map();
    const adj = new Map();
    for (const p of plugins) {
        inDegree.set(p.id, 0);
        adj.set(p.id, []);
    }
    for (const p of plugins) {
        for (const dep of p.dependencies) {
            if (!ids.has(dep))
                continue;
            adj.get(dep).push(p.id);
            inDegree.set(p.id, (inDegree.get(p.id) ?? 0) + 1);
        }
    }
    for (const [from, to] of extraEdges) {
        if (!ids.has(from) || !ids.has(to))
            continue;
        adj.get(from).push(to);
        inDegree.set(to, (inDegree.get(to) ?? 0) + 1);
    }
    const queue = plugins.filter((p) => inDegree.get(p.id) === 0);
    const byId = new Map(plugins.map((p) => [p.id, p]));
    const order = [];
    while (queue.length > 0) {
        const node = queue.shift();
        order.push(node);
        for (const neighborId of adj.get(node.id) ?? []) {
            const deg = (inDegree.get(neighborId) ?? 1) - 1;
            inDegree.set(neighborId, deg);
            if (deg === 0)
                queue.push(byId.get(neighborId));
        }
    }
    const cycled = plugins.filter((p) => !order.includes(p)).map((p) => p.id);
    return { order, cycled };
}
