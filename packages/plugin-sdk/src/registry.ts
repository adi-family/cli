import type { EventBus } from './bus.js';
import type { PluginDescriptor } from './types.js';
import { AppContext, type AppContextOptions } from './app-context.js';
import { AdiPlugin } from './plugin.js';

const registry = new Map<string, AdiPlugin>();
const descriptors = new Map<string, PluginDescriptor>();
let sharedApp: AppContext | undefined;
let appOptions: AppContextOptions = {};

export function registerPlugin(plugin: AdiPlugin): void {
  registry.set(plugin.id, plugin);
}

/** Configure AppContext options before plugins are loaded. */
export function configureApp(options: AppContextOptions): void {
  if (sharedApp) {
    throw new Error(
      'configureApp() must be called before any plugins are loaded.',
    );
  }
  appOptions = { ...appOptions, ...options };
}

/**
 * Load a remote ES module with the page's import map applied.
 *
 * Creates a tiny inline <script type="module"> shim that dynamically imports
 * the real URL. Inline module scripts participate in the document's import map,
 * so when the remote module uses bare specifiers like "lit", the browser
 * resolves them through the import map — including transitive imports.
 */
const IMPORT_TIMEOUT_MS = 15_000;

let importModule: (url: string) => Promise<Record<string, unknown>> = (url) =>
  new Promise((resolve, reject) => {
    const id = `__adiPlugin${Date.now()}_${Math.random().toString(36).slice(2)}`;

    const timer = setTimeout(() => {
      cleanup();
      reject(new Error(`Plugin module timed out after ${IMPORT_TIMEOUT_MS}ms: ${url}`));
    }, IMPORT_TIMEOUT_MS);

    const script = document.createElement('script');

    const cleanup = () => {
      clearTimeout(timer);
      delete (globalThis as Record<string, unknown>)[id];
      delete (globalThis as Record<string, unknown>)[`${id}_err`];
      script.remove();
    };

    (globalThis as Record<string, unknown>)[id] = (mod: Record<string, unknown>) => {
      cleanup();
      resolve(mod);
    };
    (globalThis as Record<string, unknown>)[`${id}_err`] = (err: unknown) => {
      cleanup();
      reject(
        err instanceof Error
          ? err
          : new Error(`Failed to load plugin module: ${url}: ${err}`),
      );
    };

    script.type = 'module';
    script.textContent =
      `import("${url}").then(m=>globalThis["${id}"](m),e=>globalThis["${id}_err"](e));`;
    document.head.appendChild(script);
  });

/** @internal Test helper — override the dynamic import used by fetchAndImport. */
export function _setImportModule(
  fn: ((url: string) => Promise<Record<string, unknown>>) | null,
): void {
  importModule = fn ?? ((url) => import(/* @vite-ignore */ url));
}

/** @internal Test helper. */
export function _resetRegistry(): void {
  registry.clear();
  descriptors.clear();
  sharedApp = undefined;
  appOptions = {};
  swMessageController?.abort();
  swMessageController = undefined;
}

function getApp(bus: EventBus): AppContext {
  if (!sharedApp)
    sharedApp = new AppContext(bus, {
      envSource: import.meta.env,
      ...appOptions,
    });
  return sharedApp;
}

export interface LoadPluginsOptions {
  timeout?: number;
  /** Known plugins available in the registry, used to auto-fetch missing `requires`. */
  availablePlugins?: PluginDescriptor[];
}

export async function loadPlugins(
  bus: EventBus,
  pluginDescriptors: PluginDescriptor[],
  options: LoadPluginsOptions = {},
): Promise<void> {
  const timeout = options.timeout ?? 5000;

  for (const d of pluginDescriptors) {
    descriptors.set(d.id, d);
  }

  const importResults = await Promise.allSettled(
    pluginDescriptors.map((d) => fetchAndImport(d)),
  );
  for (let i = 0; i < importResults.length; i++) {
    const r = importResults[i];
    if (r.status === 'rejected') {
      console.error(
        `[plugin] failed to load '${pluginDescriptors[i].id}':`,
        r.reason,
      );
    }
  }

  // Resolve `requires`: auto-fetch missing required plugins from availablePlugins.
  const available = new Map(
    (options.availablePlugins ?? []).map((d) => [d.id, d]),
  );
  const autoInstalled = await resolveRequires(bus, available);

  const plugins = [...registry.values()];
  const requiresEdges = collectRequiresEdges(plugins);
  const { order, cycled } = topoSort(plugins, requiresEdges);

  const loaded: string[] = [];
  const failed: string[] = [...cycled];
  const timedOut: string[] = [];
  const reasons: Record<string, string> = {};

  for (const id of cycled) {
    reasons[id] = 'circular dependency detected';
  }

  // Track bundle fetch failures so they get the real reason, not "did not register".
  const fetchFailed = new Set<string>();
  for (let i = 0; i < importResults.length; i++) {
    const r = importResults[i];
    if (r.status === 'rejected') {
      const id = pluginDescriptors[i].id;
      const reason = `failed to load bundle: ${r.reason instanceof Error ? r.reason.message : String(r.reason)}`;
      failed.push(id);
      reasons[id] = reason;
      fetchFailed.add(id);
    }
  }

  const allDescriptors = [...pluginDescriptors, ...autoInstalled];
  const registeredIds = new Set(plugins.map((p) => p.id));
  for (const d of allDescriptors) {
    if (!registeredIds.has(d.id) && !fetchFailed.has(d.id)) {
      const reason = `bundle loaded but did not register (missing PluginShell export)`;
      console.error(`[plugin] '${d.id}' ${reason}`);
      failed.push(d.id);
      reasons[d.id] = reason;
    }
  }

  const app = getApp(bus);
  const levels = topoLevels(order, requiresEdges);
  for (const level of levels) {
    const results = await Promise.all(
      level.map(async (plugin: AdiPlugin) => {
        const result = await initWithTimeout(plugin, app, timeout);
        return { plugin, result };
      }),
    );

    for (const { plugin, result } of results) {
      if (result === 'ok') loaded.push(plugin.id);
      else if (result === 'timeout') {
        const reason = `timed out during onRegister (>${timeout}ms)`;
        console.error(`[plugin] '${plugin.id}' ${reason}`);
        timedOut.push(plugin.id);
        reasons[plugin.id] = reason;
      } else {
        const reason = `threw during onRegister: ${result.error instanceof Error ? result.error.message : String(result.error)}`;
        console.error(`[plugin] '${plugin.id}' ${reason}`);
        failed.push(plugin.id);
        reasons[plugin.id] = reason;
      }
    }
  }

  // Phase 4: Background update checks (non-blocking).
  void checkForUpdates(bus, allDescriptors);

  // Phase 5: Signal completion.
  bus.emit(
    'loading-finished',
    { loaded, failed, timedOut, reasons },
    'plugin-registry',
  );
}

export interface UpgradePluginOptions {
  timeout?: number;
}

export async function initInternalPlugin(
  bus: EventBus,
  plugin: AdiPlugin,
): Promise<void> {
  registerPlugin(plugin);
  await plugin._init(getApp(bus));
}

export async function upgradePlugin(
  bus: EventBus,
  descriptor: PluginDescriptor,
  options: UpgradePluginOptions = {},
): Promise<void> {
  const timeout = options.timeout ?? 5000;
  const { id, installedVersion } = descriptor;
  const existing = registry.get(id);
  const fromVersion = existing?.version ?? 'unknown';

  bus.emit(
    'plugin:upgrading',
    { pluginId: id, fromVersion, toVersion: installedVersion },
    'plugin-registry',
  );

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
    } else if (typeof result === 'object') {
      throw new Error(
        `Plugin ${id} errored during upgrade: ${result.error instanceof Error ? result.error.message : String(result.error)}`,
      );
    }

    descriptors.set(id, descriptor);
    bus.emit(
      'plugin:upgraded',
      { pluginId: id, fromVersion, toVersion: installedVersion },
      'plugin-registry',
    );
  } catch (err) {
    bus.emit(
      'plugin:upgrade-failed',
      {
        pluginId: id,
        reason: err instanceof Error ? err.message : String(err),
      },
      'plugin-registry',
    );
  }
}

let swMessageController: AbortController | undefined;

export async function registerPluginSW(
  swUrl: URL | string,
  bus: EventBus,
): Promise<void> {
  if (!('serviceWorker' in navigator)) return;

  swMessageController?.abort();
  swMessageController = new AbortController();

  const reg = await navigator.serviceWorker.register(swUrl, { type: 'module' });

  navigator.serviceWorker.addEventListener(
    'message',
    (event: MessageEvent) => {
      const data = event.data as { type: string; url: string } | undefined;
      if (data?.type !== 'plugin:bundle-updated') return;

      for (const [id, descriptor] of descriptors) {
        descriptor.registry
          .getBundleInfo(id, descriptor.installedVersion)
          .then((info) => {
            if (info.jsUrl !== data.url) return;
            descriptor.registry
              .checkLatest(id, descriptor.installedVersion)
              .then((latest) => {
                if (!latest) return;
                bus.emit(
                  'plugin:update-available',
                  {
                    pluginId: id,
                    currentVersion: descriptor.installedVersion,
                    newVersion: latest.version,
                    newUrl: data.url,
                  },
                  'plugin-sw',
                );
              })
              .catch(() => null);
          })
          .catch(() => null);
      }
    },
    { signal: swMessageController.signal },
  );

  await reg.update().catch(() => null);
}

async function checkForUpdates(
  bus: EventBus,
  pluginDescriptors: PluginDescriptor[],
): Promise<void> {
  await Promise.allSettled(
    pluginDescriptors.map(async ({ id, registry: reg, installedVersion }) => {
      const result = await reg
        .checkLatest(id, installedVersion)
        .catch(() => null);
      if (!result) return;
      const info = await reg
        .getBundleInfo(id, result.version)
        .catch(() => null);
      if (!info) return;
      bus.emit(
        'plugin:update-available',
        {
          pluginId: id,
          currentVersion: installedVersion,
          newVersion: result.version,
          newUrl: info.jsUrl,
        },
        'plugin-registry',
      );
    }),
  );
}

type InitResult = 'ok' | 'timeout' | { error: unknown };

async function initWithTimeout(
  plugin: AdiPlugin,
  app: AppContext,
  timeoutMs: number,
): Promise<InitResult> {
  return new Promise((resolve) => {
    const timer = setTimeout(() => resolve('timeout'), timeoutMs);
    plugin
      ._init(app)
      .then(() => {
        clearTimeout(timer);
        resolve('ok');
      })
      .catch((err: unknown) => {
        clearTimeout(timer);
        resolve({ error: err });
      });
  });
}

async function fetchAndImport(descriptor: PluginDescriptor): Promise<void> {
  const bundleInfo = await descriptor.registry.getBundleInfo(
    descriptor.id,
    descriptor.installedVersion,
  );

  if (bundleInfo.cssUrl) {
    const cssRes = await fetch(bundleInfo.cssUrl).catch(() => null);
    if (cssRes?.ok) {
      const cssText = await cssRes.text();
      if (cssText) {
        const style = document.createElement('style');
        style.setAttribute('data-plugin', descriptor.id);
        style.textContent = cssText;
        document.head.appendChild(style);
      }
    }
  }

  const mod = await importModule(bundleInfo.jsUrl);
  if ('PluginShell' in mod) {
    if (typeof mod.PluginShell !== 'function') {
      throw new Error(
        `PluginShell export must be a class, got ${typeof mod.PluginShell}`,
      );
    }
    const instance = new (mod.PluginShell as new () => unknown)();
    if (
      typeof instance !== 'object' ||
      instance === null ||
      typeof (instance as Record<string, unknown>)['id'] !== 'string' ||
      typeof (instance as Record<string, unknown>)['_init'] !== 'function' ||
      typeof (instance as Record<string, unknown>)['_destroy'] !== 'function'
    ) {
      throw new Error(`PluginShell must extend AdiPlugin`);
    }
    registerPlugin(instance as AdiPlugin);
  }
}

/** Walk registered plugins' `requires`, fetch missing ones from availablePlugins. */
async function resolveRequires(
  bus: EventBus,
  available: Map<string, PluginDescriptor>,
): Promise<PluginDescriptor[]> {
  const installed: PluginDescriptor[] = [];
  const seen = new Set<string>();
  let frontier = [...registry.values()].flatMap((p) => p.requires ?? []);

  while (frontier.length > 0) {
    const next: string[] = [];
    for (const reqId of frontier) {
      if (seen.has(reqId) || registry.has(reqId)) continue;
      seen.add(reqId);

      const desc = available.get(reqId);
      if (!desc) {
        console.error(
          `[plugin] required plugin '${reqId}' not found in availablePlugins`,
        );
        continue;
      }

      try {
        await fetchAndImport(desc);
        descriptors.set(desc.id, desc);
        installed.push(desc);
        bus.emit(
          'plugin:installed',
          { pluginId: reqId, reason: 'auto' },
          'plugin-registry',
        );

        const newPlugin = registry.get(reqId);
        if (newPlugin) {
          next.push(...(newPlugin.requires ?? []));
        }
      } catch (err) {
        console.error(
          `[plugin] failed to auto-install required plugin '${reqId}':`,
          err,
        );
      }
    }
    frontier = next;
  }

  return installed;
}

/** Collect dependency edges from `requires` (same direction as `dependencies`). */
function collectRequiresEdges(plugins: AdiPlugin[]): Array<[string, string]> {
  return plugins.flatMap((p) =>
    (p.requires ?? []).map((req): [string, string] => [req, p.id]),
  );
}

/** Group topo-sorted plugins into parallel levels respecting dependency order. */
function topoLevels(
  order: AdiPlugin[],
  edges: Array<[string, string]>,
): AdiPlugin[][] {
  const depSet = new Map<string, Set<string>>();
  for (const [from, to] of edges)
    depSet.set(to, (depSet.get(to) ?? new Set()).add(from));

  const levels: AdiPlugin[][] = [];
  const assigned = new Map<string, number>();

  for (const plugin of order) {
    const deps = depSet.get(plugin.id);
    const level = deps
      ? Math.max(...[...deps].map((d) => (assigned.get(d) ?? -1) + 1))
      : 0;
    assigned.set(plugin.id, level);
    (levels[level] ??= []).push(plugin);
  }

  return levels;
}

/** Kahn's algorithm topological sort. */
function topoSort(
  plugins: AdiPlugin[],
  extraEdges: Array<[string, string]> = [],
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

  for (const [from, to] of extraEdges) {
    if (!ids.has(from) || !ids.has(to)) continue;
    adj.get(from)!.push(to);
    inDegree.set(to, (inDegree.get(to) ?? 0) + 1);
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
