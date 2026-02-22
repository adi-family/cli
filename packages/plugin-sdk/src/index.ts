// src/index.ts
import './events.js';

export type {
  EventRegistry,
  EventBus,
  ReplyableEvent,
  EventHandler,
  WithCid,
  PluginRegistry,
  PluginDescriptor,
} from './types.js';

export { createEventBus } from './bus.js';
export { AdiPlugin } from './plugin.js';
export { CocoonPluginRegistry } from './registry-cocoon.js';
export {
  registerPlugin,
  loadPlugins,
  upgradePlugin,
  registerPluginSW,
} from './registry.js';
export type { LoadPluginsOptions, UpgradePluginOptions } from './registry.js';

// Service worker entrypoint: new URL('./sw.js', import.meta.url)
// Register it via registerPluginSW() before calling loadPlugins().
