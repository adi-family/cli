import { EventBus } from '@adi-family/sdk-plugin';
import { DbConnection } from './db-connection';
import { RegistryHub } from './registry-hub';
import { SignalingHub } from './signaling-hub';

export interface Context {
  db: DbConnection;
  bus: EventBus;
}

export class App {
  static async init(): Promise<App> {
    const bus = EventBus.init();
    const db = DbConnection.init();
    db.registerStore('prefs');

    const ctx: Context = { db, bus };
    await RegistryHub.init(ctx);
    await SignalingHub.init(ctx);

    return new App();
  }
}
