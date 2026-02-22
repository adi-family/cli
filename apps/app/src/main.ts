// src/main.ts
import './index.css';
import './components/app-root.ts';
import {
  createEventBus,
  loadPlugins,
  registerPluginSW,
  CocoonPluginRegistry,
  upgradePlugin,
} from '@adi-family/sdk-plugin';

const cocoonUrl = (): string =>
  (globalThis as unknown as Record<string, string>)['COCOON_URL'] ?? 'http://localhost:4200';

const bus = createEventBus();

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

// Load plugins — URLs come from Cocoon service discovery at runtime.
await loadPlugins(bus, [
  // Populated at runtime from Cocoon
], { timeout: 5000 });

bus.on('loading-finished', ({ loaded, failed, timedOut }) => {
  console.info('[plugins] loaded:', loaded);
  if (failed.length) console.warn('[plugins] failed:', failed);
  if (timedOut.length) console.warn('[plugins] timed out:', timedOut);
});
