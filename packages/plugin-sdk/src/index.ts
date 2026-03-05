import './events.js';

export type {
  EventRegistry,
  EventMeta,
  BusMiddleware,
  EventHandler,
  PluginApiRegistry,
  PluginRegistry,
  PluginDescriptor,
} from './types.js';

export { EventBus } from './bus.js';
export { AppContext } from './app-context.js';
export { Logger } from './logger.js';
export type { LogLevel, DebugInfoProvider } from './logger.js';
export { trace } from './log-decorator.js';
export { AdiPlugin } from './plugin.js';
export { env } from './env.js';
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
