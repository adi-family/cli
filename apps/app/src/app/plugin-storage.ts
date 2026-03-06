import type { PluginStorage, StorageFactory } from '@adi-family/sdk-plugin';

const DB_NAME = 'adi-plugin-storage';
const DB_VERSION = 1;
const STORE = 'kv';

const openDb = (): Promise<IDBDatabase> =>
  new Promise((resolve, reject) => {
    const req = indexedDB.open(DB_NAME, DB_VERSION);
    req.onupgradeneeded = () => {
      const db = req.result;
      if (!db.objectStoreNames.contains(STORE)) {
        db.createObjectStore(STORE);
      }
    };
    req.onsuccess = () => resolve(req.result);
    req.onerror = () => reject(req.error);
  });

const withStore = async <T>(
  mode: IDBTransactionMode,
  fn: (store: IDBObjectStore) => IDBRequest<T>,
): Promise<T> => {
  const db = await openDb();
  try {
    const req = fn(db.transaction(STORE, mode).objectStore(STORE));
    return await new Promise<T>((resolve, reject) => {
      req.onsuccess = () => resolve(req.result);
      req.onerror = () => reject(req.error);
    });
  } finally {
    db.close();
  }
};

const scopedKey = (pluginId: string, key: string) => `${pluginId}:${key}`;
const keyPrefix = (pluginId: string) => `${pluginId}:`;

const createPluginStorage = (pluginId: string): PluginStorage => ({
  async get<T = unknown>(key: string): Promise<T | undefined> {
    const result = await withStore('readonly', (s) => s.get(scopedKey(pluginId, key)));
    return result as T | undefined;
  },

  async set<T = unknown>(key: string, value: T): Promise<void> {
    await withStore('readwrite', (s) => s.put(value, scopedKey(pluginId, key)));
  },

  async delete(key: string): Promise<void> {
    await withStore('readwrite', (s) => s.delete(scopedKey(pluginId, key)));
  },

  async keys(): Promise<string[]> {
    const allKeys = await withStore('readonly', (s) => s.getAllKeys());
    const prefix = keyPrefix(pluginId);
    return (allKeys as string[])
      .filter((k) => k.startsWith(prefix))
      .map((k) => k.slice(prefix.length));
  },
});

export const pluginStorageFactory: StorageFactory = createPluginStorage;
