// src/index.ts
import './events.js';

export type {
  EventRegistry,
  EventBus,
  EventMeta,
  BusMiddleware,
  ReplyableEvent,
  EventHandler,
  WithCid,
  PluginRegistry,
  PluginDescriptor,
} from './types.js';

export { createEventBus } from './bus.js';
export { AdiPlugin } from './plugin.js';
export { HttpPluginRegistry } from './registry-http.js';
export type { RegistryHealth } from './registry-http.js';
export {
  registerPlugin,
  initInternalPlugin,
  loadPlugins,
  upgradePlugin,
  registerPluginSW,
} from './registry.js';
export type { LoadPluginsOptions, UpgradePluginOptions } from './registry.js';

// Service worker entrypoint: new URL('./sw.js', import.meta.url)
// Register it via registerPluginSW() before calling loadPlugins().
