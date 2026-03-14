import {
  EventBus,
  Logger,
  trace,
  configureApp,
  initInternalPlugin,
  type AdiPlugin,
  type PluginDescriptor,
} from '@adi-family/sdk-plugin';
import { AdiDebugScreenBusKey } from '@adi-family/plugin-debug-screen';
import { DbConnection } from './db-connection';
import { PluginCore } from './plugin-core';
import { pluginStorageFactory } from './plugin-storage';
import { RegistryHub } from './registry-hub';
import { createRegistryHubDebugSync } from './registry-hub-debug';
import { getEnabledWebPluginIds } from '../plugin-prefs';

const SIGNALING_CONNECT_TIMEOUT_MS = 10_000;
const MIN_LOADING_MS = 1_500;
const REQUIRED_PLUGINS = new Set(
  (import.meta.env.VITE_REQUIRED_PLUGINS as string ?? '').split(',').filter(Boolean),
);

export interface Context {
  db: DbConnection;
}

export class App {
  private static _instance: App | null = null;

  // @ts-expect-error accessed at runtime by @trace decorator
  private readonly log = new Logger('app');

  readonly bus: EventBus;
  readonly db: DbConnection;
  readonly core!: PluginCore;
  private readonly registryHub!: RegistryHub;

  allPlugins: PluginDescriptor[] = [];
  debug: { loaded: string[]; failed: string[]; timedOut: string[] } | null =
    null;
  private loadingStart = 0;

  private constructor(
    bus: EventBus,
    db: DbConnection,
    core: PluginCore,
    registryHub: RegistryHub,
  ) {
    this.bus = bus;
    this.db = db;
    this.core = core;
    this.registryHub = registryHub;
  }

  static get instance(): App | null {
    return App._instance;
  }

  static get reqInstance(): App {
    if (!App._instance) throw new Error('App not initialized');
    return App._instance;
  }

  static async init(): Promise<App> {
    const loadingStart = Date.now();

    window.dispatchEvent(new CustomEvent('loading-step', { detail: 'init' }));
    const bus = EventBus.init();
    configureApp({ storageFactory: pluginStorageFactory });
    const db = DbConnection.init();
    db.registerStore('prefs');

    window.dispatchEvent(new CustomEvent('loading-step', { detail: 'registry' }));
    const registryHub = RegistryHub.init();
    await registryHub.start({ db });

    window.dispatchEvent(new CustomEvent('loading-step', { detail: 'plugins' }));
    const core = new PluginCore(bus, registryHub);
    const app = new App(bus, db, core, registryHub);
    App._instance = app;
    await app.init();

    app.loadingStart = loadingStart;
    return app;
  }

  @trace('init')
  async init() {
    this.core.registerPluginById('adi.slots');
    this.core.registerPluginById('adi.router');
    this.core.registerPluginById('adi.command-palette');
    this.core.registerPluginById('adi.auth');
    this.core.registerPluginById('adi.debug-screen');
    this.core.registerPluginById('adi.signaling');
    this.core.registerPluginById('adi.cocoon');
    this.core.registerPluginById('adi.actions-feed');
    this.core.registerPluginById('adi.cocoon-control-center');
    this.core.registerPluginById('adi.credentials');
    this.core.registerPluginById('adi.plugins');

    await this.registerEnabledPlugins();
    const { allPlugins, loaded, failed, timedOut, reasons } = await this.core.fetchPlugins();
    this.allPlugins = allPlugins;
    this.debug = { loaded, failed, timedOut };

    const broken = [...failed, ...timedOut];
    const requiredBroken = broken.filter((id) => REQUIRED_PLUGINS.has(id));
    const optionalBroken = broken.filter((id) => !REQUIRED_PLUGINS.has(id));

    if (optionalBroken.length > 0) {
      const details = optionalBroken.map((id) => `${id}: ${reasons[id] ?? 'unknown'}`);
      console.warn(`[app] optional plugins failed to load:\n  ${details.join('\n  ')}`);
    }

    if (requiredBroken.length > 0) {
      const details = requiredBroken.map((id) => `${id}: ${reasons[id] ?? 'unknown'}`);
      throw new Error(`Required plugins failed:\n  ${details.join('\n  ')}`);
    }

    this.registerRegistryHubDebug();
  }

  private registerRegistryHubDebug(): void {
    const debugSync = createRegistryHubDebugSync(this.registryHub);
    this.bus.emit(
      AdiDebugScreenBusKey.RegisterSection,
      {
        pluginId: 'app.registry-hub',
        init: debugSync.init,
        label: 'Registry Hub',
      },
      'app',
    );
  }

  @trace('starting')
  async start(): Promise<void> {
    window.dispatchEvent(new CustomEvent('loading-step', { detail: 'signaling' }));

    await new Promise<void>((resolve, reject) => {
      const timer = setTimeout(() => {
        unsub();
        reject(new Error('Signaling connection timed out after 10s'));
      }, SIGNALING_CONNECT_TIMEOUT_MS);

      const unsub = this.bus.on(
        'adi.signaling:state',
        ({ state }: { state: string }) => {
          if (state === 'connected') {
            clearTimeout(timer);
            unsub();
            resolve();
          }
        },
        'app',
      );
    });

    const elapsed = Date.now() - this.loadingStart;
    if (elapsed < MIN_LOADING_MS) {
      await new Promise((r) => setTimeout(r, MIN_LOADING_MS - elapsed));
    }

    window.dispatchEvent(new Event('app-ready'));
  }

  @trace('disposing')
  dispose(): void {
    this.core.dispose();
    App._instance = null;
  }

  private async registerEnabledPlugins(): Promise<void> {
    const enabledIds = await getEnabledWebPluginIds();
    if (!enabledIds?.size) return;

    for (const id of enabledIds) {
      this.core.registerPluginById(id);
    }
  }
}
