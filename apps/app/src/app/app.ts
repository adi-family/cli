import { EventBus } from '@adi-family/sdk-plugin';
import { Database, PreferencesStore } from './database';
import { RegistryHub } from './registry';
import { SignalingServerHub } from './signaling';
import { DbConnection } from './db-connection';

export interface Context {
  db: Database;
  prefs: PreferencesStore;
  registryHub: RegistryHub;
  signalingHub: SignalingServerHub;
  bus: EventBus;
}

export class App {
  ctx: Context;
  constructor(ctx: Context) {
    this.ctx = ctx;
  }

  static async init(): Promise<App> {
    const bus = EventBus.init();
    const connection = DbConnection.init(`RootStore`);
    const prefs = new PreferencesStore(db);
    const sharedConnections = new Map<string, DbConnection>();

    const ctx = { bus, prefs };

    const [registryHub, signalingHub] = await Promise.all([
      RegistryHub.init(ctx),
      SignalingServerHub.init(ctx, sharedConnections),
    ]);

    return new App({ db, prefs, registryHub, signalingHub, bus });
  }
}
