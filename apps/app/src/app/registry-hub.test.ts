import { describe, it, expect, mock, jest, beforeEach, afterEach } from 'bun:test';

const mockListPlugins = mock(() => Promise.resolve([]));
const mockCheckHealth = mock(() => Promise.resolve({ online: true }));

mock.module('@adi-family/sdk-plugin', () => ({
  Logger: class {
    trace() {}
    warn() {}
    error() {}
  },
  trace: () => (_target: unknown, _key: string, desc: PropertyDescriptor) => desc,
  HttpPluginRegistry: class {
    listPlugins = mockListPlugins;
    checkHealth = mockCheckHealth;
  },
  env: () => ['https://default.registry'],
}));

mock.module('./env', () => ({
  DEFAULT_REGISTRIES: ['https://default.registry'],
}));

// Minimal IDB mock
function createMockIDB(stored?: string[]) {
  const data = new Map<string, unknown>();
  if (stored) data.set('registry-urls', stored);

  const makeRequest = <T>(result: T): IDBRequest<T> => {
    const req = { result, error: null } as unknown as IDBRequest<T>;
    queueMicrotask(() => req.onsuccess?.({} as Event));
    return req;
  };

  const objectStore = (mode: IDBTransactionMode) => ({
    get: (key: string) => makeRequest(data.get(key)),
    put: (value: unknown, key: string) => {
      data.set(key, value);
      return makeRequest(undefined);
    },
  });

  return {
    transaction: (_store: string, mode: IDBTransactionMode) => ({
      objectStore: () => objectStore(mode),
    }),
    data,
  };
}

import { RegistryHub } from './registry-hub';

function makeCtx(stored?: string[]) {
  const idb = createMockIDB(stored);
  return {
    ctx: { db: { open: async () => idb as unknown as IDBDatabase } },
    idb,
  };
}

beforeEach(() => {
  mockListPlugins.mockClear();
  mockCheckHealth.mockClear();
});

afterEach(() => {
  jest.restoreAllMocks();
});

describe('RegistryHub', () => {
  describe('init', () => {
    it('creates an instance with default protected URLs', () => {
      const hub = RegistryHub.init();
      expect(hub).toBeInstanceOf(RegistryHub);
      expect(hub.isProtected('https://default.registry')).toBe(true);
    });
  });

  describe('start', () => {
    it('uses saved URLs when available', async () => {
      const hub = RegistryHub.init();
      const { ctx } = makeCtx(['https://saved.one', 'https://saved.two']);
      await hub.start(ctx);
      expect(hub.allServers().size).toBe(2);
      expect(hub.getServer('https://saved.one')).toBeDefined();
      expect(hub.getServer('https://saved.two')).toBeDefined();
    });

    it('falls back to defaults when no saved URLs', async () => {
      const hub = RegistryHub.init();
      const { ctx } = makeCtx();
      await hub.start(ctx);
      expect(hub.allServers().size).toBe(1);
      expect(hub.getServer('https://default.registry')).toBeDefined();
    });

    it('falls back to defaults when saved is empty array', async () => {
      const hub = RegistryHub.init();
      const { ctx } = makeCtx([]);
      await hub.start(ctx);
      expect(hub.allServers().size).toBe(1);
    });
  });

  describe('addRegistry', () => {
    it('adds a new server and registry', async () => {
      const hub = RegistryHub.init();
      const { ctx } = makeCtx();
      await hub.start(ctx);

      const client = hub.addRegistry('https://new.registry');
      expect(client).toBeDefined();
      expect(hub.getServer('https://new.registry')).toBeDefined();
      expect(hub.allRegistries().has('https://new.registry')).toBe(true);
    });

    it('returns existing client for duplicate URL', async () => {
      const hub = RegistryHub.init();
      const { ctx } = makeCtx();
      await hub.start(ctx);

      const first = hub.addRegistry('https://dup.registry');
      const second = hub.addRegistry('https://dup.registry');
      expect(first).toBe(second);
    });

    it('persists URLs after adding', async () => {
      const hub = RegistryHub.init();
      const { ctx, idb } = makeCtx();
      await hub.start(ctx);

      hub.addRegistry('https://extra.registry');
      await new Promise((r) => setTimeout(r, 10));
      expect(idb.data.get('registry-urls')).toBeDefined();
    });
  });

  describe('removeRegistry', () => {
    it('removes a non-protected server', async () => {
      const hub = RegistryHub.init();
      const { ctx } = makeCtx();
      await hub.start(ctx);

      hub.addRegistry('https://removable.registry');
      expect(hub.allServers().size).toBe(2);

      hub.removeRegistry('https://removable.registry');
      expect(hub.getServer('https://removable.registry')).toBeUndefined();
      expect(hub.allRegistries().has('https://removable.registry')).toBe(false);
    });

    it('does not remove protected URLs', async () => {
      const hub = RegistryHub.init();
      const { ctx } = makeCtx();
      await hub.start(ctx);

      hub.removeRegistry('https://default.registry');
      expect(hub.getServer('https://default.registry')).toBeDefined();
    });

    it('no-ops for unknown URLs', async () => {
      const hub = RegistryHub.init();
      const { ctx } = makeCtx();
      await hub.start(ctx);

      const sizeBefore = hub.allServers().size;
      hub.removeRegistry('https://unknown.registry');
      expect(hub.allServers().size).toBe(sizeBefore);
    });
  });

  describe('isProtected', () => {
    it('returns true for default registries', () => {
      const hub = RegistryHub.init();
      expect(hub.isProtected('https://default.registry')).toBe(true);
    });

    it('returns false for non-default registries', () => {
      const hub = RegistryHub.init();
      expect(hub.isProtected('https://random.registry')).toBe(false);
    });
  });

  describe('fetchAllDescriptors', () => {
    it('returns descriptors from all registries', async () => {
      const hub = RegistryHub.init();
      const { ctx } = makeCtx();
      await hub.start(ctx);

      mockListPlugins.mockResolvedValueOnce([
        { id: 'plugin-a', installedVersion: '1.0.0' },
      ]);

      const descriptors = await hub.fetchAllDescriptors();
      expect(descriptors.length).toBe(1);
      expect(descriptors[0].id).toBe('plugin-a');
    });

    it('handles rejected promises gracefully', async () => {
      const hub = RegistryHub.init();
      const { ctx } = makeCtx();
      await hub.start(ctx);

      mockListPlugins.mockRejectedValueOnce(new Error('network error'));

      const descriptors = await hub.fetchAllDescriptors();
      expect(descriptors).toEqual([]);
    });

    it('returns empty array when no registries', async () => {
      const hub = RegistryHub.init();
      const result = await hub.fetchAllDescriptors();
      expect(result).toEqual([]);
    });
  });

  describe('allRegistries / allServers', () => {
    it('returns readonly maps', async () => {
      const hub = RegistryHub.init();
      const { ctx } = makeCtx();
      await hub.start(ctx);

      const registries = hub.allRegistries();
      const servers = hub.allServers();
      expect(registries.size).toBe(1);
      expect(servers.size).toBe(1);
    });
  });

  describe('dispose', () => {
    it('clears all servers and registries', async () => {
      const hub = RegistryHub.init();
      const { ctx } = makeCtx();
      await hub.start(ctx);

      hub.addRegistry('https://extra.registry');
      expect(hub.allServers().size).toBe(2);

      hub.dispose();
      expect(hub.allServers().size).toBe(0);
      expect(hub.allRegistries().size).toBe(0);
    });
  });

  describe('loadUrls error handling', () => {
    it('returns defaults when DB open fails', async () => {
      const hub = RegistryHub.init();
      const ctx = { db: { open: async () => { throw new Error('db error'); } } };
      await hub.start(ctx as any);
      expect(hub.allServers().size).toBe(1);
    });
  });
});
