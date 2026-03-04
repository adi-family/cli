const DB_NAME = 'adi-app';
const DB_VERSION = 1;
const STORE_NAME = 'prefs';
const PREFS_KEY = 'enabled-web-plugins';

const openDb = (): Promise<IDBDatabase> =>
  new Promise((resolve, reject) => {
    const req = indexedDB.open(DB_NAME, DB_VERSION);
    req.onupgradeneeded = () => {
      const db = req.result;
      if (!db.objectStoreNames.contains(STORE_NAME)) {
        db.createObjectStore(STORE_NAME);
      }
    };
    req.onsuccess = () => resolve(req.result);
    req.onerror = () => reject(req.error);
  });

const idbGet = async <T>(key: string): Promise<T | undefined> => {
  const db = await openDb();
  try {
    const tx = db.transaction(STORE_NAME, 'readonly');
    const store = tx.objectStore(STORE_NAME);
    const req = store.get(key);
    return await new Promise<T | undefined>((resolve, reject) => {
      req.onsuccess = () => resolve(req.result as T | undefined);
      req.onerror = () => reject(req.error);
    });
  } finally {
    db.close();
  }
};

const idbPut = async (key: string, value: unknown): Promise<void> => {
  const db = await openDb();
  try {
    const tx = db.transaction(STORE_NAME, 'readwrite');
    const store = tx.objectStore(STORE_NAME);
    const req = store.put(value, key);
    await new Promise<void>((resolve, reject) => {
      req.onsuccess = () => resolve();
      req.onerror = () => reject(req.error);
    });
  } finally {
    db.close();
  }
};

/** Returns the set of user-enabled web plugin IDs, or null if never configured. */
export async function getEnabledWebPluginIds(): Promise<Set<string> | null> {
  try {
    const ids = await idbGet<string[]>(PREFS_KEY);
    return ids ? new Set(ids) : null;
  } catch {
    return null;
  }
}

/** Persists the enabled web plugin ID set to IndexedDB. */
export async function setEnabledWebPluginIds(ids: Iterable<string>): Promise<void> {
  await idbPut(PREFS_KEY, [...ids]);
}
