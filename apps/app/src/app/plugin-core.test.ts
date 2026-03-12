import { describe, it, expect, mock, jest, beforeEach, afterEach } from 'bun:test';

const mockInitInternalPlugin = mock(() => Promise.resolve());
const mockLoadPlugins = mock(() => Promise.resolve());

mock.module('@adi-family/sdk-plugin', () => ({
  Logger: class {
    trace() {}
    warn() {}
    error() {}
  },
  trace: () => (_target: unknown, _key: string, desc: PropertyDescriptor) => desc,
  initInternalPlugin: mockInitInternalPlugin,
  loadPlugins: mockLoadPlugins,
}));

import { PluginCore } from './plugin-core';

function makeRegistryHub(descriptors: Array<{ id: string; installedVersion: string }> = []) {
  return {
    fetchAllDescriptors: mock(() => Promise.resolve(descriptors)),
    dispose: mock(),
    allServers: () => new Map(),
    allRegistries: () => new Map(),
  };
}

function makeBus() {
  return {
    on: mock(),
    off: mock(),
    emit: mock(),
  };
}

beforeEach(() => {
  mockInitInternalPlugin.mockClear();
  mockLoadPlugins.mockClear();
});

afterEach(() => {
  jest.restoreAllMocks();
});

describe('PluginCore', () => {
  describe('registerPluginById', () => {
    it('adds id to pending set', () => {
      const core = new PluginCore(makeBus() as any, makeRegistryHub() as any);
      core.registerPluginById('adi.auth');
      core.registerPluginById('adi.router');
      // IDs are pending — they won't be in plugins map yet
      expect(core.has('adi.auth')).toBe(false);
    });

    it('deduplicates IDs', async () => {
      const descriptors = [{ id: 'adi.auth', installedVersion: '1.0.0' }];
      const hub = makeRegistryHub(descriptors);
      const core = new PluginCore(makeBus() as any, hub as any);

      core.registerPluginById('adi.auth');
      core.registerPluginById('adi.auth');

      await core.fetchPlugins();
      expect(mockLoadPlugins).toHaveBeenCalledTimes(1);
      const loadedDescriptors = mockLoadPlugins.mock.calls[0][1];
      expect(loadedDescriptors).toHaveLength(1);
    });
  });

  describe('registerPlugin', () => {
    it('calls initInternalPlugin and stores plugin', async () => {
      const core = new PluginCore(makeBus() as any, makeRegistryHub() as any);
      const plugin = { id: 'test-plugin', version: '1.0.0' } as any;

      await core.registerPlugin(plugin);
      expect(mockInitInternalPlugin).toHaveBeenCalledTimes(1);
      expect(core.has('test-plugin')).toBe(true);
      expect(core.get('test-plugin')).toBe(plugin);
    });
  });

  describe('fetchPlugins', () => {
    it('fetches descriptors and loads matching pending plugins', async () => {
      const descriptors = [
        { id: 'adi.auth', installedVersion: '1.0.0' },
        { id: 'adi.router', installedVersion: '1.0.0' },
        { id: 'adi.other', installedVersion: '1.0.0' },
      ];
      const hub = makeRegistryHub(descriptors);
      const bus = makeBus();
      const core = new PluginCore(bus as any, hub as any);

      core.registerPluginById('adi.auth');
      core.registerPluginById('adi.router');

      const result = await core.fetchPlugins();
      expect(result).toHaveLength(3);
      expect(mockLoadPlugins).toHaveBeenCalledTimes(1);

      const [, toLoad] = mockLoadPlugins.mock.calls[0];
      expect(toLoad).toHaveLength(2);
      expect(toLoad.map((d: any) => d.id).sort()).toEqual(['adi.auth', 'adi.router']);
    });

    it('does not call loadPlugins when no pending IDs match', async () => {
      const hub = makeRegistryHub([{ id: 'unrelated', installedVersion: '1.0.0' }]);
      const core = new PluginCore(makeBus() as any, hub as any);

      core.registerPluginById('adi.auth');
      await core.fetchPlugins();

      expect(mockLoadPlugins).not.toHaveBeenCalled();
    });

    it('clears pending IDs after fetch', async () => {
      const descriptors = [{ id: 'adi.auth', installedVersion: '1.0.0' }];
      const hub = makeRegistryHub(descriptors);
      const core = new PluginCore(makeBus() as any, hub as any);

      core.registerPluginById('adi.auth');
      await core.fetchPlugins();

      // Second fetch should not load anything
      mockLoadPlugins.mockClear();
      await core.fetchPlugins();
      expect(mockLoadPlugins).not.toHaveBeenCalled();
    });

    it('deduplicates descriptors by ID', async () => {
      const descriptors = [
        { id: 'adi.auth', installedVersion: '1.0.0' },
        { id: 'adi.auth', installedVersion: '2.0.0' },
      ];
      const hub = makeRegistryHub(descriptors);
      const core = new PluginCore(makeBus() as any, hub as any);

      const result = await core.fetchPlugins();
      expect(result).toHaveLength(1);
      expect(result[0].installedVersion).toBe('1.0.0');
    });
  });

  describe('dispose', () => {
    it('calls registryHub.dispose', () => {
      const hub = makeRegistryHub();
      const core = new PluginCore(makeBus() as any, hub as any);
      core.dispose();
      expect(hub.dispose).toHaveBeenCalledTimes(1);
    });
  });

  describe('get / has / ids', () => {
    it('returns undefined for unregistered plugin', () => {
      const core = new PluginCore(makeBus() as any, makeRegistryHub() as any);
      expect(core.get('missing')).toBeUndefined();
      expect(core.has('missing')).toBe(false);
    });

    it('returns registered plugin IDs', async () => {
      const core = new PluginCore(makeBus() as any, makeRegistryHub() as any);
      const plugin = { id: 'test', version: '1.0.0' } as any;
      await core.registerPlugin(plugin);

      expect(core.ids()).toEqual(['test']);
      expect(core.has('test')).toBe(true);
    });
  });
});
