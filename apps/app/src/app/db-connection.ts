import { type EventBus, Logger } from '@adi-family/sdk-plugin';

export class DbConnection {
  private log: Logger = new Logger('db-connection');
  private dbPromise: Promise<IDBDatabase> | null = null;
  private seenStores: string[] = [];

  constructor(private name: string) {}
  public static init(name: string) {
    return new DbConnection(name);
  }

  private reset(): void {
    this.dbPromise = null;
  }

  open(bus: EventBus, dbName: string, version: number): Promise<IDBDatabase> {
    if (this.dbPromise) return this.dbPromise;

    this.dbPromise = new Promise((resolve, reject) => {
      const req = indexedDB.open(dbName, version);

      req.onupgradeneeded = () => {
        const db = req.result;

        for (const name of this.seenStores) {
          if (!db.objectStoreNames.contains(name)) {
            db.createObjectStore(name);
          }
        }
      };

      req.onsuccess = () => {
        const db = req.result;
        this.log.trace(bus, { msg: 'connected', dbName, version });

        db.onclose = () => {
          this.log.warn(bus, { msg: 'closed', dbName });
          this.reset();
          bus.emit('db:disconnected', { reason: 'closed' }, this.name);
        };

        db.onversionchange = () => {
          this.log.warn(bus, { msg: 'version-change', dbName });
          db.close();
          this.reset();
          bus.emit('db:disconnected', { reason: 'version-change' }, this.name);
        };

        bus.emit('db:connected', {}, this.name);
        resolve(db);
      };

      req.onerror = () => {
        this.log.error(bus, { msg: 'open failed', dbName, error: String(req.error) });
        this.reset();
        bus.emit('db:error', { error: String(req.error) }, this.name);
        reject(req.error);
      };
    });

    return this.dbPromise;
  }
}
