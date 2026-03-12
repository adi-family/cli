import './events.js';
export { EventBus } from './bus.js';
export { AppContext } from './app-context.js';
export { Logger } from './logger.js';
export { trace } from './log-decorator.js';
export { AdiPlugin } from './plugin.js';
export { env } from './env.js';
export { HttpPluginRegistry } from './registry-http.js';
export { registerPlugin, configureApp, initInternalPlugin, loadPlugins, upgradePlugin, registerPluginSW, } from './registry.js';
// Service worker entrypoint: new URL('./sw.js', import.meta.url)
// Register it via registerPluginSW() before calling loadPlugins().
