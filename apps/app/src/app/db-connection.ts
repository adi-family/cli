import { Logger, trace } from '@adi-family/sdk-plugin';

export class DbConnection {
  private log: Logger = new Logger('db-connection', () => ({
    stores: [...this.seenStores],
  }));
  private dbPromise: Promise<IDBDatabase> | null = null;
  private seenStores: Set<string> = new Set();

  constructor() {}

  public static init() {
    return new DbConnection();
  }

  private reset(): void {
    this.dbPromise = null;
  }

  @trace('registering store')
  registerStore(storeName: string): void {
    this.seenStores.add(storeName);
  }

  @trace('opening database')
  open(dbName: string, version: number): Promise<IDBDatabase> {
    if (this.dbPromise) return this.dbPromise;

    this.dbPromise = new Promise((resolve, reject) => {
      const req = indexedDB.open(dbName, version);

      req.onupgradeneeded = () => {
        const db = req.result;

        for (const name of Array.from(this.seenStores)) {
          if (!db.objectStoreNames.contains(name)) {
            db.createObjectStore(name);
          }
        }
      };

      req.onsuccess = () => {
        const db = req.result;
        this.log.trace({ msg: 'connected', dbName, version });

        db.onclose = () => {
          this.log.warn({ msg: 'closed', dbName });
          this.reset();
        };

        db.onversionchange = () => {
          this.log.warn({ msg: 'version-change', dbName });
          db.close();
          this.reset();
        };

        resolve(db);
      };

      req.onerror = () => {
        this.log.error({
          msg: 'open failed',
          dbName,
          error: String(req.error),
        });
        this.reset();
        reject(req.error);
      };
    });

    return this.dbPromise;
  }
}
