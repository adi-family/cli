import { describe, it, expect, mock, jest, beforeEach, afterEach } from 'bun:test';

mock.module('@adi-family/sdk-plugin', () => ({
  Logger: class {
    trace() {}
    warn() {}
    error() {}
  },
  trace: () => (_target: unknown, _key: string, desc: PropertyDescriptor) => desc,
}));

import { DbConnection } from './db-connection';

// Minimal IndexedDB mock
function createMockIndexedDB() {
  const stores = new Map<string, Map<string, unknown>>();
  const objectStoreNames = {
    contains: (name: string) => stores.has(name),
  };

  const createObjectStore = (name: string) => {
    stores.set(name, new Map());
  };

  const mockDb = {
    objectStoreNames,
    createObjectStore,
    onclose: null as ((ev: Event) => void) | null,
    onversionchange: null as ((ev: Event) => void) | null,
    close: mock(() => {}),
    transaction: (_storeName: string | string[], _mode: IDBTransactionMode) => ({
      objectStore: (name: string) => {
        const store = stores.get(name) ?? new Map();
        return {
          get: (key: string) => {
            const req = {
              result: store.get(key),
              error: null,
              onsuccess: null as ((ev: Event) => void) | null,
              onerror: null as ((ev: Event) => void) | null,
            };
            queueMicrotask(() => req.onsuccess?.({} as Event));
            return req;
          },
          put: (value: unknown, key: string) => {
            store.set(key, value);
            const req = {
              result: undefined,
              error: null,
              onsuccess: null as ((ev: Event) => void) | null,
              onerror: null as ((ev: Event) => void) | null,
            };
            queueMicrotask(() => req.onsuccess?.({} as Event));
            return req;
          },
        };
      },
    }),
  };

  let openCallCount = 0;
  let triggerUpgrade = false;

  const mockOpen = (dbName: string, version?: number) => {
    openCallCount++;
    const req = {
      result: mockDb,
      error: null,
      onupgradeneeded: null as ((ev: Event) => void) | null,
      onsuccess: null as ((ev: Event) => void) | null,
      onerror: null as ((ev: Event) => void) | null,
    };
    queueMicrotask(() => {
      if (triggerUpgrade) {
        req.onupgradeneeded?.({} as Event);
      }
      req.onsuccess?.({} as Event);
    });
    return req;
  };

  return {
    mockDb,
    stores,
    mockOpen,
    get openCallCount() { return openCallCount; },
    set triggerUpgrade(v: boolean) { triggerUpgrade = v; },
  };
}

let idbMock: ReturnType<typeof createMockIndexedDB>;

beforeEach(() => {
  idbMock = createMockIndexedDB();
  (globalThis as any).indexedDB = { open: idbMock.mockOpen };
});

afterEach(() => {
  jest.restoreAllMocks();
});

describe('DbConnection', () => {
  describe('init', () => {
    it('creates a new instance', () => {
      const db = DbConnection.init();
      expect(db).toBeInstanceOf(DbConnection);
    });
  });

  describe('registerStore', () => {
    it('registers stores for creation on upgrade', async () => {
      idbMock.triggerUpgrade = true;
      const db = DbConnection.init();
      db.registerStore('prefs');
      db.registerStore('cache');

      await db.open('test-db', 1);
      expect(idbMock.stores.has('prefs')).toBe(true);
      expect(idbMock.stores.has('cache')).toBe(true);
    });
  });

  describe('open', () => {
    it('returns a database instance', async () => {
      const db = DbConnection.init();
      const result = await db.open('test-db', 1);
      expect(result).toBe(idbMock.mockDb);
    });

    it('caches the db promise on subsequent calls', async () => {
      const db = DbConnection.init();
      const first = db.open('test-db', 1);
      const second = db.open('test-db', 1);
      expect(first).toBe(second);
      await first;
      expect(idbMock.openCallCount).toBe(1);
    });

    it('resets cache on error', async () => {
      const failOpen = (_name: string, _version?: number) => {
        const req = {
          result: null,
          error: new DOMException('open failed'),
          onupgradeneeded: null as any,
          onsuccess: null as any,
          onerror: null as any,
        };
        queueMicrotask(() => req.onerror?.({} as Event));
        return req;
      };

      (globalThis as any).indexedDB = { open: failOpen };
      const db = DbConnection.init();

      await expect(db.open('test-db', 1)).rejects.toBeDefined();

      // After failure, the cache should be cleared, allowing retry
      (globalThis as any).indexedDB = { open: idbMock.mockOpen };
      const result = await db.open('test-db', 1);
      expect(result).toBe(idbMock.mockDb);
    });

    it('resets cache on db close event', async () => {
      const db = DbConnection.init();
      await db.open('test-db', 1);

      // Simulate close event
      idbMock.mockDb.onclose?.({} as Event);

      // Next open should create a new request
      (globalThis as any).indexedDB = { open: idbMock.mockOpen };
      const result = await db.open('test-db', 1);
      expect(result).toBeDefined();
      expect(idbMock.openCallCount).toBe(2);
    });

    it('resets and closes on version change event', async () => {
      const db = DbConnection.init();
      await db.open('test-db', 1);

      idbMock.mockDb.onversionchange?.({} as Event);
      expect(idbMock.mockDb.close).toHaveBeenCalled();
    });

    it('does not recreate existing stores on upgrade', async () => {
      idbMock.stores.set('existing', new Map());
      idbMock.triggerUpgrade = true;

      const db = DbConnection.init();
      db.registerStore('existing');
      db.registerStore('new-store');

      await db.open('test-db', 1);
      expect(idbMock.stores.has('existing')).toBe(true);
      expect(idbMock.stores.has('new-store')).toBe(true);
    });
  });
});
