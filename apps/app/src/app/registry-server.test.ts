import { describe, it, expect, mock, jest, beforeEach, afterEach } from 'bun:test';

const mockCheckHealth = mock(() => Promise.resolve({ online: true }));
const mockListPlugins = mock(() => Promise.resolve([{ id: 'p1', installedVersion: '1.0.0' }]));

mock.module('@adi-family/sdk-plugin', () => ({
  Logger: class {
    trace() {}
    warn() {}
    error() {}
  },
  trace: () => (_target: unknown, _key: string, desc: PropertyDescriptor) => desc,
  HttpPluginRegistry: class {
    checkHealth = mockCheckHealth;
    listPlugins = mockListPlugins;
  },
}));

import { RegistryServer } from './registry-server';

beforeEach(() => {
  mockCheckHealth.mockClear();
  mockListPlugins.mockClear();
  mockCheckHealth.mockResolvedValue({ online: true });
  mockListPlugins.mockResolvedValue([{ id: 'p1', installedVersion: '1.0.0' }]);
});

afterEach(() => {
  jest.restoreAllMocks();
});

describe('RegistryServer', () => {
  describe('constructor', () => {
    it('initializes with url and disconnected state', () => {
      const server = new RegistryServer('https://test.registry', () => true);
      expect(server.url).toBe('https://test.registry');
      expect(server.getState()).toBe('disconnected');
      expect(server.getHealth()).toBeNull();
      expect(server.getPlugins()).toEqual([]);
      expect(server.client).toBeDefined();
    });
  });

  describe('connect', () => {
    it('does nothing when isStarted returns false', async () => {
      const server = new RegistryServer('https://test.registry', () => false);
      server.connect();
      await new Promise((r) => setTimeout(r, 50));
      expect(mockCheckHealth).not.toHaveBeenCalled();
    });

    it('triggers poll when started', async () => {
      const server = new RegistryServer('https://test.registry', () => true);
      server.connect();

      // Wait for setTimeout(fn, 0) + async poll to settle
      await new Promise((r) => setTimeout(r, 50));

      expect(mockCheckHealth).toHaveBeenCalledTimes(1);
      server.disconnect();
    });
  });

  describe('disconnect', () => {
    it('sets state to disconnected', () => {
      const server = new RegistryServer('https://test.registry', () => true);
      server.connect();
      server.disconnect();
      expect(server.getState()).toBe('disconnected');
    });

    it('prevents future polling', async () => {
      const server = new RegistryServer('https://test.registry', () => true);
      server.disconnect();
      server.connect(); // connect after disconnect — disposed flag is true
      await new Promise((r) => setTimeout(r, 50));
      expect(mockCheckHealth).not.toHaveBeenCalled();
    });
  });

  describe('poll — online', () => {
    it('transitions to connected when health is online', async () => {
      const server = new RegistryServer('https://test.registry', () => true);
      server.connect();

      await new Promise((r) => setTimeout(r, 50));

      expect(server.getState()).toBe('connected');
      expect(server.getHealth()).toEqual({ online: true });
      expect(server.getPlugins()).toEqual([{ id: 'p1', installedVersion: '1.0.0' }]);
      server.disconnect();
    });
  });

  describe('poll — offline', () => {
    it('transitions to disconnected and clears plugins when offline', async () => {
      mockCheckHealth.mockResolvedValue({ online: false });

      const server = new RegistryServer('https://test.registry', () => true);
      server.connect();

      await new Promise((r) => setTimeout(r, 50));

      expect(server.getState()).toBe('disconnected');
      expect(server.getPlugins()).toEqual([]);
      server.disconnect();
    });
  });

  describe('poll — disposed mid-flight', () => {
    it('does not update state if disposed during health check', async () => {
      let resolveHealth!: (v: unknown) => void;
      mockCheckHealth.mockImplementation(
        () => new Promise((r) => { resolveHealth = r; }),
      );

      const server = new RegistryServer('https://test.registry', () => true);
      server.connect();

      // Wait for setTimeout to fire and poll to start
      await new Promise((r) => setTimeout(r, 50));

      // Now disconnect before resolving
      server.disconnect();
      resolveHealth({ online: true });
      await new Promise((r) => setTimeout(r, 10));

      expect(server.getState()).toBe('disconnected');
    });
  });

  describe('getState / getHealth / getPlugins', () => {
    it('returns initial empty state', () => {
      const server = new RegistryServer('https://test.registry', () => true);
      expect(server.getState()).toBe('disconnected');
      expect(server.getHealth()).toBeNull();
      expect(server.getPlugins()).toEqual([]);
    });
  });
});
