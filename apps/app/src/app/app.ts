import {
  EventBus,
  Logger,
  trace,
  type PluginDescriptor,
  loadPlugins,
  upgradePlugin,
} from '@adi-family/sdk-plugin';
import { DbConnection } from './db-connection';
import { PluginCore } from './plugin-core';
import { RegistryPlugin } from '../plugins/registry-plugin';
import { SignalingPlugin } from '../plugins/signaling-plugin';
import { RouterPlugin } from '../plugins/router';
import { PluginsPlugin } from '../plugins/plugins-page';
import { ActionsPlugin } from '../components/actions-loop';
import { DebugScreenPlugin } from '../plugins/debug-screen-plugin';
import {
  getEnabledWebPluginIds,
  migrateFromLocalStorage,
} from '../plugin-prefs';

export interface Context {
  db: DbConnection;
}

export interface AppContext {
  db: DbConnection;
  bus: EventBus;
}

export class App {
  private static _instance: App | null = null;

  private readonly log = new Logger('app');

  readonly bus: EventBus;
  readonly db: DbConnection;
  readonly core: PluginCore;

  allPlugins: PluginDescriptor[] = [];
  debug: { loaded: string[]; failed: string[]; timedOut: string[] } | null = null;

  private constructor(bus: EventBus, db: DbConnection, core: PluginCore) {
    this.bus = bus;
    this.db = db;
    this.core = core;
  }

  get router(): RouterPlugin {
    return this.core.get<RouterPlugin>('app.router')!;
  }

  get signalingHub() {
    return this.core.get<SignalingPlugin>('app.signaling')?.hub;
  }

  get registryHub() {
    return this.core.get<RegistryPlugin>('app.registry')?.hub;
  }

  static get instance(): App | null {
    return App._instance;
  }

  static get reqInstance(): App {
    if (!App._instance) throw new Error('App not initialized');
    return App._instance;
  }

  static init(): App {
    const bus = EventBus.init();
    const db = DbConnection.init();
    db.registerStore('prefs');

    const core = new PluginCore(bus);
    const app = new App(bus, db, core);
    App._instance = app;

    bus.on(
      'loading-finished',
      (payload) => { app.debug = payload; },
      'app',
    );

    bus.on(
      'plugin:update-available',
      (payload) => {
        const descriptor = app.allPlugins.find((d) => d.id === payload.pluginId);
        if (!descriptor) return;
        void upgradePlugin(bus, { ...descriptor, installedVersion: payload.newVersion });
      },
      'app',
    );

    return app;
  }

  @trace('starting')
  async start(): Promise<void> {
    const ctx: AppContext = { db: this.db, bus: this.bus };

    await this.core.install(RegistryPlugin.init(ctx));
    await this.core.install(SignalingPlugin.init(ctx));
    await this.core.install(RouterPlugin.init(ctx));
    await this.core.install(PluginsPlugin.init(ctx));
    await this.core.install(ActionsPlugin.init(ctx));
    await this.core.install(DebugScreenPlugin.init(ctx));

    window.dispatchEvent(new Event('app-ready'));
    void this.loadEnabledPlugins();
  }

  @trace('disposing')
  dispose(): void {
    this.signalingHub?.dispose();
    this.registryHub?.dispose();
    App._instance = null;
  }

  private async loadEnabledPlugins(): Promise<void> {
    await migrateFromLocalStorage();

    const registry = this.core.get<RegistryPlugin>('app.registry');
    if (!registry) {
      this.log.warn({ msg: 'RegistryPlugin not installed, skipping plugin load' });
      return;
    }

    const allDescriptors = await registry.hub.fetchAllDescriptors();
    this.allPlugins = allDescriptors;

    const enabledIds = await getEnabledWebPluginIds();
    const webDescriptors = allDescriptors.filter((d) =>
      d.pluginTypes?.includes('web'),
    );
    const toLoad = enabledIds
      ? webDescriptors.filter((d) => enabledIds.has(d.id))
      : webDescriptors;

    if (toLoad.length === 0) {
      this.bus.emit(
        'loading-finished',
        { loaded: [], failed: [], timedOut: [] },
        'app',
      );
      this.log.warn({ msg: 'No plugins to load' });
      return;
    }

    await loadPlugins(this.bus, toLoad, {
      availablePlugins: allDescriptors,
    });
  }
}
