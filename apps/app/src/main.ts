// src/main.ts
import './index.css';
import './components/app-root.ts';
import {
  createEventBus,
  loadPlugins,
  registerPlugin,
  registerPluginSW,
  CocoonPluginRegistry,
  upgradePlugin,
  type EventBus,
} from '@adi-family/sdk-plugin';
import { TasksPlugin } from '../../../crates/tasks/web/src/index.ts';

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

const cocoonUrl = (): string =>
  (globalThis as unknown as Record<string, string>)['COCOON_URL'] ?? 'http://localhost:4200';

const bus = createEventBus();

const connections = new Map<string, Connection>();
(window as unknown as { sdk: object }).sdk = {
  getConnections: () => connections,
  bus,
};

registerPlugin(new TasksPlugin());

// Register service worker for plugin bundle caching.
await registerPluginSW(new URL('./plugin-sw.js', import.meta.url), bus);

// Expose bus globally so app components can access it.
(globalThis as Record<string, unknown>)['__adiBus'] = bus;

// Auto-upgrade when SW detects a new version.
bus.on('plugin:update-available', ({ pluginId, newVersion }) => {
  void upgradePlugin(bus, {
    id: pluginId,
    registry: new CocoonPluginRegistry(cocoonUrl()),
    installedVersion: newVersion,
  });
});

// Subscribe before loadPlugins so we don't miss the event via FIFO queue.
bus.on('loading-finished', ({ loaded, failed, timedOut }) => {
  console.info('[plugins] loaded:', loaded);
  if (failed.length) console.warn('[plugins] failed:', failed);
  if (timedOut.length) console.warn('[plugins] timed out:', timedOut);
});

// Load plugins — URLs come from Cocoon service discovery at runtime.
await loadPlugins(bus, [
  // Populated at runtime from Cocoon
], { timeout: 5000 });

window.dispatchEvent(new Event('sdk-ready'));
