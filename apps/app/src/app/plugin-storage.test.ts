import { describe, it, expect, jest, beforeEach, afterEach } from 'bun:test';

mock.module('@adi-family/sdk-plugin', () => ({
  Logger: class {
    trace() {}
    warn() {}
    error() {}
  },
  trace: () => (_target: unknown, _key: string, desc: PropertyDescriptor) => desc,
}));

import { mock } from 'bun:test';
import { pluginStorageFactory } from './plugin-storage';

function createMockIDB() {
  const store = new Map<string, unknown>();

  const objectStore = {
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
    delete: (key: string) => {
      store.delete(key);
      const req = {
        result: undefined,
        error: null,
        onsuccess: null as ((ev: Event) => void) | null,
        onerror: null as ((ev: Event) => void) | null,
      };
      queueMicrotask(() => req.onsuccess?.({} as Event));
      return req;
    },
    getAllKeys: () => {
      const req = {
        result: [...store.keys()],
        error: null,
        onsuccess: null as ((ev: Event) => void) | null,
        onerror: null as ((ev: Event) => void) | null,
      };
      queueMicrotask(() => req.onsuccess?.({} as Event));
      return req;
    },
  };

  const mockDb = {
    transaction: () => ({ objectStore: () => objectStore }),
    close: mock(() => {}),
    objectStoreNames: { contains: () => true },
  };

  const mockOpen = () => {
    const req = {
      result: mockDb,
      error: null,
      onupgradeneeded: null as any,
      onsuccess: null as any,
      onerror: null as any,
    };
    queueMicrotask(() => req.onsuccess?.({} as Event));
    return req;
  };

  return { store, mockOpen, mockDb };
}

let idbMock: ReturnType<typeof createMockIDB>;

beforeEach(() => {
  idbMock = createMockIDB();
  (globalThis as any).indexedDB = { open: idbMock.mockOpen };
});

afterEach(() => {
  jest.restoreAllMocks();
});

describe('pluginStorageFactory', () => {
  it('creates isolated storage per plugin', async () => {
    const storageA = pluginStorageFactory('plugin-a');
    const storageB = pluginStorageFactory('plugin-b');

    await storageA.set('key', 'value-a');
    await storageB.set('key', 'value-b');

    expect(await storageA.get('key')).toBe('value-a');
    expect(await storageB.get('key')).toBe('value-b');
  });

  describe('get', () => {
    it('returns undefined for missing key', async () => {
      const storage = pluginStorageFactory('test');
      expect(await storage.get('missing')).toBeUndefined();
    });

    it('returns stored value', async () => {
      const storage = pluginStorageFactory('test');
      await storage.set('name', 'hello');
      expect(await storage.get('name')).toBe('hello');
    });
  });

  describe('set', () => {
    it('stores values with scoped keys', async () => {
      const storage = pluginStorageFactory('my-plugin');
      await storage.set('config', { theme: 'dark' });

      // Internal store should have scoped key
      expect(idbMock.store.has('my-plugin:config')).toBe(true);
      expect(idbMock.store.get('my-plugin:config')).toEqual({ theme: 'dark' });
    });
  });

  describe('delete', () => {
    it('removes the value', async () => {
      const storage = pluginStorageFactory('test');
      await storage.set('key', 'value');
      await storage.delete('key');
      expect(await storage.get('key')).toBeUndefined();
    });
  });

  describe('keys', () => {
    it('returns only keys for the plugin scope', async () => {
      const storage = pluginStorageFactory('test');
      await storage.set('a', 1);
      await storage.set('b', 2);

      // Add a key from another plugin scope
      idbMock.store.set('other:c', 3);

      const keys = await storage.keys();
      expect(keys.sort()).toEqual(['a', 'b']);
    });

    it('returns empty array when no keys exist', async () => {
      const storage = pluginStorageFactory('empty');
      const keys = await storage.keys();
      expect(keys).toEqual([]);
    });
  });

  describe('close behavior', () => {
    it('closes db after each operation', async () => {
      const storage = pluginStorageFactory('test');
      await storage.set('key', 'value');
      expect(idbMock.mockDb.close).toHaveBeenCalled();
    });
  });
});
