import { EventBus } from '@adi-family/sdk-plugin';
import { DbConnection } from './db-connection';
import { RegistryHub } from './registry-hub';
import { SignalingHub } from './signaling-hub';

export interface Context {
  db: DbConnection;
}

export class App {
  private static _instance: App | null = null;

  readonly bus: EventBus;
  readonly db: DbConnection;
  readonly registryHub: RegistryHub;
  readonly signalingHub: SignalingHub;

  debug: { loaded: string[]; failed: string[]; timedOut: string[] } | null = null;
  allPlugins: Array<{ id: string; installedVersion: string; pluginTypes?: string[] }> = [];
  authAnonymous: ((signalingUrl: string, authDomain: string) => void) | null = null;

  private constructor(
    bus: EventBus,
    db: DbConnection,
    registryHub: RegistryHub,
    signalingHub: SignalingHub,
  ) {
    this.bus = bus;
    this.db = db;
    this.registryHub = registryHub;
    this.signalingHub = signalingHub;
  }

  static get instance(): App | null {
    return App._instance;
  }

  static init(): App {
    const bus = EventBus.init();
    const db = DbConnection.init();
    db.registerStore('prefs');

    const registryHub = RegistryHub.init();
    const signalingHub = SignalingHub.init(bus);

    const app = new App(bus, db, registryHub, signalingHub);
    App._instance = app;

    return app;
  }

  async start(): Promise<void> {
    const ctx = { db: this.db };
    await Promise.all([
      this.registryHub.start(ctx),
      this.signalingHub.start(ctx),
    ]);
  }

  dispose(): void {
    this.signalingHub.dispose();
    this.registryHub.dispose();
    App._instance = null;
  }
}
