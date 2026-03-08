import {
  EventBus,
  Logger,
  trace,
  configureApp,
  type PluginDescriptor,
} from '@adi-family/sdk-plugin';
import { AdiDebugScreenBusKey } from '@adi/debug-screen-web-plugin/bus';
import { DbConnection } from './db-connection';
import { PluginCore } from './plugin-core';
import { pluginStorageFactory } from './plugin-storage';
import { RegistryHub } from './registry-hub';
import { createRegistryHubDebugSync } from './registry-hub-debug';
import { getEnabledWebPluginIds } from '../plugin-prefs';

export interface Context {
  db: DbConnection;
}

export class App {
  private static _instance: App | null = null;

  // @ts-expect-error accessed at runtime by @trace decorator
  private readonly log = new Logger('app');

  readonly bus: EventBus;
  readonly db: DbConnection;
  readonly core: PluginCore;
  private readonly registryHub: RegistryHub;

  allPlugins: PluginDescriptor[] = [];
  debug: { loaded: string[]; failed: string[]; timedOut: string[] } | null =
    null;

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
    const bus = EventBus.init();
    configureApp({ storageFactory: pluginStorageFactory });
    const db = DbConnection.init();
    db.registerStore('prefs');
    const registryHub = RegistryHub.init();
    await registryHub.start({ db });
    const core = new PluginCore(bus, registryHub);
    const app = new App(bus, db, core, registryHub);
    App._instance = app;
    await app.init();
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

    await this.registerEnabledPlugins();
    this.allPlugins = await this.core.fetchPlugins();
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
