import { getBus } from './bus';

const DB_NAME = 'adi-app';
const DB_VERSION = 1;
const SOURCE = 'database';

const STORES = ['prefs'] as const;
export type StoreName = (typeof STORES)[number];

let dbPromise: Promise<IDBDatabase> | null = null;

function resetConnection() {
  dbPromise = null;
}

function getDb(): Promise<IDBDatabase> {
  if (!dbPromise) {
    dbPromise = new Promise((resolve, reject) => {
      const req = indexedDB.open(DB_NAME, DB_VERSION);
      req.onupgradeneeded = () => {
        const db = req.result;
        for (const name of STORES) {
          if (!db.objectStoreNames.contains(name)) {
            db.createObjectStore(name);
          }
        }
      };

      req.onsuccess = () => {
        const db = req.result;

        db.onclose = () => {
          resetConnection();
          getBus().emit('db:disconnected', { reason: 'closed' }, SOURCE);
        };

        db.onversionchange = () => {
          db.close();
          resetConnection();
          getBus().emit('db:disconnected', { reason: 'version-change' }, SOURCE);
        };
        
        getBus().emit('db:connected', {}, SOURCE);
        resolve(db);
      };

      req.onerror = () => {
        resetConnection();
        getBus().emit('db:error', { error: String(req.error) }, SOURCE);
        reject(req.error);
      };
    });
  }
  return dbPromise;
}

async function withStore<T>(
  store: StoreName,
  mode: IDBTransactionMode,
  fn: (s: IDBObjectStore) => IDBRequest<T>,
  retry = true,
): Promise<T> {
  const db = await getDb();
  try {
    const tx = db.transaction(store, mode);
    const s = tx.objectStore(store);
    const req = fn(s);
    return await new Promise<T>((resolve, reject) => {
      req.onsuccess = () => resolve(req.result);
      req.onerror = () => reject(req.error);
    });
  } catch (err) {
    if (retry) {
      resetConnection();
      getBus().emit('db:reconnecting', { store, mode }, SOURCE);
      return withStore(store, mode, fn, false);
    }
    getBus().emit('db:error', { error: String(err), store, mode }, SOURCE);
    throw err;
  }
}

export async function get<T>(store: StoreName, key: string): Promise<T | undefined> {
  const result = await withStore<T>(store, 'readonly', (s) => s.get(key) as IDBRequest<T>);
  return result ?? undefined;
}

export async function put(store: StoreName, key: string, value: unknown): Promise<IDBValidKey> {
  return withStore(store, 'readwrite', (s) => s.put(value, key));
}

export async function del(store: StoreName, key: string): Promise<undefined> {
  return withStore(store, 'readwrite', (s) => s.delete(key));
}

export async function getAll<T>(store: StoreName): Promise<T[]> {
  return withStore<T[]>(store, 'readonly', (s) => s.getAll() as IDBRequest<T[]>);
}

export async function keys(store: StoreName): Promise<IDBValidKey[]> {
  return withStore(store, 'readonly', (s) => s.getAllKeys());
}

export async function clear(store: StoreName): Promise<undefined> {
  return withStore(store, 'readwrite', (s) => s.clear());
}
