import type { EventBus } from './bus.js';
import type { PluginDescriptor } from './types.js';
import { AdiPlugin } from './plugin.js';

const registry = new Map<string, AdiPlugin>();
const descriptors = new Map<string, PluginDescriptor>();

export function registerPlugin(plugin: AdiPlugin): void {
  registry.set(plugin.id, plugin);
}

/** @internal Test helper. */
export function _resetRegistry(): void {
  registry.clear();
  descriptors.clear();
  swMessageController?.abort();
  swMessageController = undefined;
}

export interface LoadPluginsOptions {
  timeout?: number;
}

export async function loadPlugins(
  bus: EventBus,
  pluginDescriptors: PluginDescriptor[],
  options: LoadPluginsOptions = {}
): Promise<void> {
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

  const plugins = [...registry.values()];
  const { order, cycled } = topoSort(plugins);

  const loaded: string[] = [];
  const failed: string[] = [...cycled];
  const timedOut: string[] = [];

  const registeredIds = new Set(plugins.map((p) => p.id));
  for (const d of pluginDescriptors) {
    if (!registeredIds.has(d.id)) {
      console.error(`[plugin] '${d.id}' bundle loaded but did not register — export your plugin class as PluginShell: export { MyPlugin as PluginShell }`);
      failed.push(d.id);
    }
  }

  for (const plugin of order) {
    const result = await initWithTimeout(plugin, bus, timeout);
    if (result === 'ok') loaded.push(plugin.id);
    else if (result === 'timeout') {
      console.error(`[plugin] '${plugin.id}' timed out during onRegister (>${timeout}ms)`);
      timedOut.push(plugin.id);
    } else {
      console.error(`[plugin] '${plugin.id}' threw during onRegister:`, result.error);
      failed.push(plugin.id);
    }
  }

  // Phase 4: Background update checks (non-blocking).
  void checkForUpdates(bus, pluginDescriptors);

  // Phase 5: Signal completion.
  bus.emit('loading-finished', { loaded, failed, timedOut }, 'plugin-registry');
}

export interface UpgradePluginOptions {
  timeout?: number;
}

export async function initInternalPlugin(bus: EventBus, plugin: AdiPlugin): Promise<void> {
  registerPlugin(plugin);
  await plugin._init(bus);
}

export async function upgradePlugin(
  bus: EventBus,
  descriptor: PluginDescriptor,
  options: UpgradePluginOptions = {}
): Promise<void> {
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
    if (!newPlugin) throw new Error(`Plugin ${id} did not call registerPlugin()`);

    const result = await initWithTimeout(newPlugin, bus, timeout);
    if (result === 'timeout') {
      throw new Error(`Plugin ${id} timed out during upgrade`);
    } else if (typeof result === 'object') {
      throw new Error(
        `Plugin ${id} errored during upgrade: ${result.error instanceof Error ? result.error.message : String(result.error)}`
      );
    }

    descriptors.set(id, descriptor);
    bus.emit('plugin:upgraded', { pluginId: id, fromVersion, toVersion: installedVersion }, 'plugin-registry');
  } catch (err) {
    bus.emit('plugin:upgrade-failed', {
      pluginId: id,
      reason: err instanceof Error ? err.message : String(err),
    }, 'plugin-registry');
  }
}

let swMessageController: AbortController | undefined;

export async function registerPluginSW(
  swUrl: URL | string,
  bus: EventBus
): Promise<void> {
  if (!('serviceWorker' in navigator)) return;

  swMessageController?.abort();
  swMessageController = new AbortController();

  const reg = await navigator.serviceWorker.register(swUrl, { type: 'module' });

  navigator.serviceWorker.addEventListener('message', (event: MessageEvent) => {
    const data = event.data as { type: string; url: string } | undefined;
    if (data?.type !== 'plugin:bundle-updated') return;

    for (const [id, descriptor] of descriptors) {
      descriptor.registry
        .bundleUrl(id, descriptor.installedVersion)
        .then((bundleUrl) => {
          if (bundleUrl !== data.url) return;
          descriptor.registry
            .checkLatest(id, descriptor.installedVersion)
            .then((latest) => {
              if (!latest) return;
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

async function checkForUpdates(
  bus: EventBus,
  pluginDescriptors: PluginDescriptor[]
): Promise<void> {
  await Promise.allSettled(
    pluginDescriptors.map(async ({ id, registry: reg, installedVersion }) => {
      const result = await reg.checkLatest(id, installedVersion).catch(() => null);
      if (!result) return;
      const newUrl = await reg.bundleUrl(id, result.version).catch(() => null);
      if (!newUrl) return;
      bus.emit('plugin:update-available', {
        pluginId: id,
        currentVersion: installedVersion,
        newVersion: result.version,
        newUrl,
      }, 'plugin-registry');
    })
  );
}

type InitResult = 'ok' | 'timeout' | { error: unknown };

async function initWithTimeout(
  plugin: AdiPlugin,
  bus: EventBus,
  timeoutMs: number
): Promise<InitResult> {
  return new Promise((resolve) => {
    const timer = setTimeout(() => resolve('timeout'), timeoutMs);
    plugin
      ._init(bus)
      .then(() => { clearTimeout(timer); resolve('ok'); })
      .catch((err: unknown) => { clearTimeout(timer); resolve({ error: err }); });
  });
}

async function fetchAndImport(descriptor: PluginDescriptor): Promise<void> {
  const url = await descriptor.registry.bundleUrl(
    descriptor.id,
    descriptor.installedVersion
  );
  const res = await fetch(url);
  if (!res.ok) {
    throw new Error(`Failed to fetch plugin bundle: ${res.status} ${res.statusText} (${url})`);
  }
  const blob = await res.blob();
  const blobUrl = URL.createObjectURL(blob);
  try {
    const mod = await import(/* @vite-ignore */ blobUrl);
    // Convention: export { MyPlugin as PluginShell } — SDK auto-registers it.
    if ('PluginShell' in mod) {
      if (typeof mod.PluginShell !== 'function') {
        throw new Error(`PluginShell export must be a class, got ${typeof mod.PluginShell}`);
      }
      const instance = new (mod.PluginShell as new () => unknown)();
      if (
        typeof instance !== 'object' || instance === null ||
        typeof (instance as Record<string, unknown>)['id'] !== 'string' ||
        typeof (instance as Record<string, unknown>)['_init'] !== 'function' ||
        typeof (instance as Record<string, unknown>)['_destroy'] !== 'function'
      ) {
        throw new Error(`PluginShell must extend AdiPlugin`);
      }
      registerPlugin(instance as AdiPlugin);
    }
  } finally {
    URL.revokeObjectURL(blobUrl);
  }
}

/** Kahn's algorithm topological sort. */
function topoSort(
  plugins: AdiPlugin[]
): { order: AdiPlugin[]; cycled: string[] } {
  const ids = new Set(plugins.map((p) => p.id));
  const inDegree = new Map<string, number>();
  const adj = new Map<string, string[]>();

  for (const p of plugins) {
    inDegree.set(p.id, 0);
    adj.set(p.id, []);
  }

  for (const p of plugins) {
    for (const dep of p.dependencies) {
      if (!ids.has(dep)) continue;
      adj.get(dep)!.push(p.id);
      inDegree.set(p.id, (inDegree.get(p.id) ?? 0) + 1);
    }
  }

  const queue = plugins.filter((p) => inDegree.get(p.id) === 0);
  const byId = new Map(plugins.map((p) => [p.id, p]));
  const order: AdiPlugin[] = [];

  while (queue.length > 0) {
    const node = queue.shift()!;
    order.push(node);
    for (const neighborId of adj.get(node.id) ?? []) {
      const deg = (inDegree.get(neighborId) ?? 1) - 1;
      inDegree.set(neighborId, deg);
      if (deg === 0) queue.push(byId.get(neighborId)!);
    }
  }

  const cycled = plugins.filter((p) => !order.includes(p)).map((p) => p.id);
  return { order, cycled };
}
