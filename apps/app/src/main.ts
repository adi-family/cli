// src/main.ts
import './index.css';
import './components/app-root.ts';
import './components/command-palette.ts';
import './components/debug-screen.ts';
import './components/actions-loop.ts';
import './components/ops-log.ts';
import './components/cocoon-manual-setup.ts';
import './components/signaling-status.ts';

import { getEnabledWebPluginIds, migrateFromLocalStorage as migratePluginPrefs } from './plugin-prefs.ts';
import { PluginsPlugin } from './plugins/plugins-page.ts';
import { ActionsPlugin } from './components/actions-loop.ts';
import {
  initInternalPlugin,
  loadPlugins,
  registerPluginSW,
  upgradePlugin,
  type EventBus,
} from '@adi-family/sdk-plugin';
import { initSignalingHub } from './services/signaling/index.ts';
import { initRegistryHub } from './services/registry/index.ts';

import { setGlobal, getGlobal } from './app/global.ts';

interface Connection {
  id: string;
  services: string[];
  request<T>(service: string, method: string, params?: unknown): Promise<T>;
  stream<T>(service: string, method: string, params?: unknown): AsyncGenerator<T>;
  httpProxy(service: string, path: string, init?: RequestInit): Promise<Response>;
  httpDirect(url: string, init?: RequestInit): Promise<Response>;
}

declare global {
  interface Window {
    sdk: {
      getConnections(): Map<string, Connection>;
      bus: EventBus;
    };
  }
}

const bus = getGlobal().bus;

const connections = new Map<string, Connection>();
const getToken = (authDomain: string, sourceUrl?: string): Promise<string | null> =>
  Promise.race([
    bus.send('auth:get-token', { authDomain, sourceUrl }, 'app').wait()
      .then((r: { token: string | null }) => r.token),
    new Promise<null>((r) => setTimeout(() => r(null), 2_000)),
  ]).catch(() => null);
const registryHub = initRegistryHub();
const debugInfo = { loaded: [] as string[], failed: [] as string[], timedOut: [] as string[] };

(window as unknown as { sdk: object }).sdk = {
  getConnections: () => connections,
  bus,
};
setGlobal({
  debug: debugInfo,
  registryHub,
});

// Migrate plugin preferences from localStorage to IndexedDB (one-time).
await migratePluginPrefs();

// Initialize built-in internal plugins.
await initInternalPlugin(bus, new PluginsPlugin());
await initInternalPlugin(bus, new ActionsPlugin());

// Register service worker for plugin bundle caching.
await registerPluginSW('/sw.js', bus);

// Discover plugins from all configured registries; failures are isolated per registry.
const registries = [...registryHub.registries.values()];
const results = await Promise.allSettled(registries.map(r => r.listPlugins()));
const pluginDescriptors = results.flatMap(r => r.status === 'fulfilled' ? r.value : []);

setGlobal({ allPlugins: pluginDescriptors });

// Only load plugins explicitly declared as web-loadable.
const webPlugins = pluginDescriptors.filter(d => d.pluginTypes?.includes('web'));

// Filter to only user-enabled web plugins (null = never configured → load none).
const enabledWebIds = await getEnabledWebPluginIds();
const enabledWebPlugins = enabledWebIds === null ? [] : webPlugins.filter(d => enabledWebIds.has(d.id));

// Map each plugin id to its source registry so upgrades use the correct one.
const pluginRegistryMap = new Map(enabledWebPlugins.map(d => [d.id, d.registry]));

// Auto-upgrade when SW detects a new version.
bus.on('plugin:update-available', ({ pluginId, newVersion }) => {
  const registry = pluginRegistryMap.get(pluginId);
  if (!registry) return;
  void upgradePlugin(bus, { id: pluginId, registry, installedVersion: newVersion });
}, 'app');

// Subscribe before loadPlugins so we don't miss the event via FIFO queue.
bus.on('loading-finished', ({ loaded, failed, timedOut }) => {
  // Persist plugin status for the debug screen
  debugInfo.loaded = loaded;
  debugInfo.failed = failed;
  debugInfo.timedOut = timedOut;

  console.info('[plugins] loaded:', loaded);
  if (failed.length) console.warn('[plugins] failed:', failed);
  if (timedOut.length) console.warn('[plugins] timed out:', timedOut);
}, 'app');

await loadPlugins(bus, enabledWebPlugins, { timeout: 5000 });

// Initialize signaling AFTER plugins are loaded so auth:get-token has a handler.
initSignalingHub(connections, bus, getToken);

window.dispatchEvent(new Event('app-ready'));
