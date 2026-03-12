import { describe, it, expect, mock, jest, beforeEach, afterEach } from 'bun:test';
import { getEnabledWebPluginIds, setEnabledWebPluginIds } from './plugin-prefs';

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
    queueMicrotask(() => {
      req.onupgradeneeded?.({} as Event);
      req.onsuccess?.({} as Event);
    });
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

describe('plugin-prefs', () => {
  describe('getEnabledWebPluginIds', () => {
    it('returns null when no IDs stored', async () => {
      const result = await getEnabledWebPluginIds();
      expect(result).toBeNull();
    });

    it('returns Set of stored IDs', async () => {
      idbMock.store.set('enabled-web-plugins', ['adi.auth', 'adi.router']);
      const result = await getEnabledWebPluginIds();
      expect(result).toBeInstanceOf(Set);
      expect(result!.size).toBe(2);
      expect(result!.has('adi.auth')).toBe(true);
      expect(result!.has('adi.router')).toBe(true);
    });

    it('returns null on IDB error', async () => {
      (globalThis as any).indexedDB = {
        open: () => {
          const req = {
            result: null,
            error: new DOMException('fail'),
            onupgradeneeded: null as any,
            onsuccess: null as any,
            onerror: null as any,
          };
          queueMicrotask(() => req.onerror?.({} as Event));
          return req;
        },
      };

      const result = await getEnabledWebPluginIds();
      expect(result).toBeNull();
    });
  });

  describe('setEnabledWebPluginIds', () => {
    it('persists plugin IDs to IDB', async () => {
      await setEnabledWebPluginIds(['adi.auth', 'adi.router']);
      expect(idbMock.store.get('enabled-web-plugins')).toEqual(['adi.auth', 'adi.router']);
    });

    it('accepts iterable (Set)', async () => {
      await setEnabledWebPluginIds(new Set(['adi.auth']));
      expect(idbMock.store.get('enabled-web-plugins')).toEqual(['adi.auth']);
    });
  });

  describe('roundtrip', () => {
    it('set then get returns same IDs', async () => {
      await setEnabledWebPluginIds(['adi.auth', 'adi.router']);
      const result = await getEnabledWebPluginIds();
      expect(result).toBeInstanceOf(Set);
      expect([...result!].sort()).toEqual(['adi.auth', 'adi.router']);
    });
  });
});
