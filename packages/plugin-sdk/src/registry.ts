// src/registry.ts
import type { EventBus, PluginDescriptor } from './types.js';
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
}

export interface LoadPluginsOptions {
  /** Per-plugin initialization timeout in ms. Default: 5000. */
  timeout?: number;
}

export async function loadPlugins(
  bus: EventBus,
  pluginDescriptors: PluginDescriptor[],
  options: LoadPluginsOptions = {}
): Promise<void> {
  const timeout = options.timeout ?? 5000;

  // Store descriptors for upgrade use
  for (const d of pluginDescriptors) {
    descriptors.set(d.id, d);
  }

  // Phase 1: Fetch + import all plugin modules concurrently.
  // Each module calls registerPlugin() as a side effect.
  await Promise.allSettled(pluginDescriptors.map((d) => fetchAndImport(d)));

  // Phase 2: Resolve dependency graph.
  const plugins = [...registry.values()];
  const { order, cycled } = topoSort(plugins);

  const loaded: string[] = [];
  const failed: string[] = [...cycled];
  const timedOut: string[] = [];

  // Mark descriptors that failed to import (never called registerPlugin).
  const registeredIds = new Set(plugins.map((p) => p.id));
  for (const d of pluginDescriptors) {
    if (!registeredIds.has(d.id)) failed.push(d.id);
  }

  // Phase 3: Initialize in topological order.
  for (const plugin of order) {
    const result = await initWithTimeout(plugin, bus, timeout);
    if (result === 'ok') loaded.push(plugin.id);
    else if (result === 'timeout') timedOut.push(plugin.id);
    else failed.push(plugin.id);
  }

  // Phase 4: Background update checks (non-blocking).
  void checkForUpdates(bus, pluginDescriptors);

  // Phase 5: Signal completion.
  bus.emit('loading-finished', { loaded, failed, timedOut });
}

export interface UpgradePluginOptions {
  /** Timeout for the new plugin's onRegister in ms. Default: 5000. */
  timeout?: number;
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

  bus.emit('plugin:upgrading', { pluginId: id, fromVersion, toVersion: installedVersion });

  try {
    // Tear down old plugin.
    if (existing) {
      await existing._destroy();
      registry.delete(id);
    }

    // Load new version — module calls registerPlugin() as side effect.
    await fetchAndImport(descriptor);

    // Initialize new plugin.
    const newPlugin = registry.get(id);
    if (!newPlugin) throw new Error(`Plugin ${id} did not call registerPlugin()`);

    const result = await initWithTimeout(newPlugin, bus, timeout);
    if (result === 'timeout') {
      throw new Error(`Plugin ${id} timed out during upgrade`);
    } else if (result === 'error') {
      throw new Error(`Plugin ${id} errored during upgrade`);
    }

    descriptors.set(id, descriptor);
    bus.emit('plugin:upgraded', { pluginId: id, fromVersion, toVersion: installedVersion });
  } catch (err) {
    bus.emit('plugin:upgrade-failed', {
      pluginId: id,
      reason: err instanceof Error ? err.message : String(err),
    });
  }
}

let swMessageController: AbortController | undefined;

export async function registerPluginSW(
  swUrl: URL | string,
  bus: EventBus
): Promise<void> {
  if (!('serviceWorker' in navigator)) return;

  // Remove previous listener if re-registering.
  swMessageController?.abort();
  swMessageController = new AbortController();

  const reg = await navigator.serviceWorker.register(swUrl, { type: 'module' });

  // Bridge SW postMessages onto the event bus.
  navigator.serviceWorker.addEventListener('message', (event: MessageEvent) => {
    const data = event.data as { type: string; url: string } | undefined;
    if (data?.type !== 'plugin:bundle-updated') return;

    for (const [id, descriptor] of descriptors) {
      descriptor.registry
        .fetchBundle(id, descriptor.installedVersion)
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
              });
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
      const newUrl = await reg.fetchBundle(id, result.version).catch(() => null);
      if (!newUrl) return;
      bus.emit('plugin:update-available', {
        pluginId: id,
        currentVersion: installedVersion,
        newVersion: result.version,
        newUrl,
      });
    })
  );
}

async function initWithTimeout(
  plugin: AdiPlugin,
  bus: EventBus,
  timeoutMs: number
): Promise<'ok' | 'timeout' | 'error'> {
  return new Promise((resolve) => {
    const timer = setTimeout(() => resolve('timeout'), timeoutMs);
    plugin
      ._init(bus)
      .then(() => { clearTimeout(timer); resolve('ok'); })
      .catch(() => { clearTimeout(timer); resolve('error'); });
  });
}

async function fetchAndImport(descriptor: PluginDescriptor): Promise<void> {
  const url = await descriptor.registry.fetchBundle(
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
    await import(/* @vite-ignore */ blobUrl);
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
