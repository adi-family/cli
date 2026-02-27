// src/main.ts
import './index.css';
import './components/app-root.ts';
import './components/command-palette.ts';
import './components/debug-screen.ts';
import './components/actions-loop.ts';
import { getEnabledWebPluginIds } from './plugin-prefs.ts';
import { PluginsPlugin } from './plugins/plugins-page.ts';
import { ActionsPlugin } from './components/actions-loop.ts';
import {
  createEventBus,
  initInternalPlugin,
  loadPlugins,
  registerPluginSW,
  upgradePlugin,
  type EventBus,
} from '@adi-family/sdk-plugin';
import { initSignalingHub } from './services/signaling/index.ts';
import { initRegistryHub } from './services/registry/index.ts';

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

const bus = createEventBus();

const connections = new Map<string, Connection>();
const getToken = (authDomain: string): Promise<string | null> =>
  bus.send('auth:get-token', { authDomain }).wait().then((r: { token: string | null }) => r.token);
const signalingHub = initSignalingHub(connections, bus, getToken);
const registryHub = initRegistryHub();
const debugInfo = { loaded: [] as string[], failed: [] as string[], timedOut: [] as string[] };

(window as unknown as { sdk: object }).sdk = {
  getConnections: () => connections,
  bus,
};
// Expose debug info and signaling outside the typed interface to avoid declaration conflicts
(window as unknown as Record<string, unknown>)['__adiDebug'] = debugInfo;
(window as unknown as Record<string, unknown>)['__adiSignaling'] = signalingHub;
(window as unknown as Record<string, unknown>)['__adiRegistryHub'] = registryHub;

// Initialize built-in internal plugins.
await initInternalPlugin(bus, new PluginsPlugin());
await initInternalPlugin(bus, new ActionsPlugin());

// Register service worker for plugin bundle caching.
await registerPluginSW(new URL('./plugin-sw.js', import.meta.url), bus);

// Expose bus globally so app components can access it.
(globalThis as Record<string, unknown>)['__adiBus'] = bus;

// Discover plugins from all configured registries; failures are isolated per registry.
const registries = [...registryHub.registries.values()];
const results = await Promise.allSettled(registries.map(r => r.listPlugins()));
const pluginDescriptors = results.flatMap(r => r.status === 'fulfilled' ? r.value : []);

// Expose all discovered plugins so the debug screen can display them.
(window as unknown as Record<string, unknown>)['__adiAllPlugins'] = pluginDescriptors;

// Only load plugins explicitly declared as web-loadable.
const webPlugins = pluginDescriptors.filter(d => d.pluginTypes?.includes('web'));

// Filter to only user-enabled web plugins (null = never configured → load none).
const enabledWebIds = getEnabledWebPluginIds();
const enabledWebPlugins = enabledWebIds === null ? [] : webPlugins.filter(d => enabledWebIds.has(d.id));

// Map each plugin id to its source registry so upgrades use the correct one.
const pluginRegistryMap = new Map(enabledWebPlugins.map(d => [d.id, d.registry]));

// Auto-upgrade when SW detects a new version.
bus.on('plugin:update-available', ({ pluginId, newVersion }) => {
  const registry = pluginRegistryMap.get(pluginId);
  if (!registry) return;
  void upgradePlugin(bus, { id: pluginId, registry, installedVersion: newVersion });
});

// Subscribe before loadPlugins so we don't miss the event via FIFO queue.
bus.on('loading-finished', ({ loaded, failed, timedOut }) => {
  // Persist plugin status for the debug screen
  debugInfo.loaded = loaded;
  debugInfo.failed = failed;
  debugInfo.timedOut = timedOut;

  console.info('[plugins] loaded:', loaded);
  if (failed.length) console.warn('[plugins] failed:', failed);
  if (timedOut.length) console.warn('[plugins] timed out:', timedOut);
});

await loadPlugins(bus, enabledWebPlugins, { timeout: 5000 });

window.dispatchEvent(new Event('sdk-ready'));
