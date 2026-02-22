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

export { registerPlugin, loadPlugins, upgradePlugin, registerPluginSW } from './registry.js';
export type { LoadPluginsOptions } from './registry.js';
