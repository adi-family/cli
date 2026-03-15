// src/registry.test.ts
import { describe, it, expect, mock, spyOn, jest, beforeEach, afterEach } from 'bun:test';
import { registerPlugin, loadPlugins, configureApp, initInternalPlugin, _resetRegistry, _setImportModule } from './registry.js';
import { AdiPlugin } from './plugin.js';
import { EventBus } from './bus.js';
import type { PluginDescriptor } from './types.js';

interface PluginOpts {
  deps?: string[];
  requires?: string[];
  onRegister?: () => Promise<void> | void;
}

function makePlugin(id: string, version = '1.0.0', opts: PluginOpts = {}): AdiPlugin {
  const { deps = [], requires = [], onRegister } = opts;
  class P extends AdiPlugin {
    readonly id = id;
    readonly version = version;
    readonly dependencies = deps;
    readonly requires = requires;
    onRegister = onRegister;
  }
  return new P();
}

/**
 * Set up mocks so fetchAndImport works in bun test.
 *
 * Overrides the dynamic import to register plugins as a side effect
 * and return an empty module (no PluginShell export).
 */
function mockFetchChain(
  pluginDefs: Array<{ id: string; version?: string; requires?: string[] }>
): void {
  const plugins = pluginDefs.map((def) =>
    makePlugin(def.id, def.version ?? '1.0.0', { requires: def.requires ?? [] })
  );

  let idx = 0;
  spyOn(globalThis, 'fetch').mockImplementation(async () => new Response(''));
  _setImportModule(async () => {
    const plugin = plugins[idx++];
    if (plugin) registerPlugin(plugin);
    return {};
  });
}

beforeEach(() => {
  _resetRegistry();
});

afterEach(() => {
  _setImportModule(null);
  jest.restoreAllMocks();
});

describe('registerPlugin', () => {
  it('allows plugin to be found by loadPlugins', async () => {
    const plugin = makePlugin('tasks');
    registerPlugin(plugin);
    const bus = EventBus.init();
    const handler = mock();
    bus.on('loading-finished', handler, 'test');
    await loadPlugins(bus, [], { timeout: 1000 });
    expect(handler).toHaveBeenCalledWith(
      expect.objectContaining({ loaded: ['tasks'] })
    );
  });
});

describe('loadPlugins — ordering', () => {
  it('initializes plugins with no deps in registration order', async () => {
    const order: string[] = [];
    registerPlugin(makePlugin('a', '1.0.0', { onRegister: async () => { order.push('a'); } }));
    registerPlugin(makePlugin('b', '1.0.0', { onRegister: async () => { order.push('b'); } }));
    await loadPlugins(EventBus.init(), [], { timeout: 1000 });
    expect(order).toEqual(['a', 'b']);
  });

  it('initializes dependency before dependent regardless of registration order', async () => {
    const order: string[] = [];
    registerPlugin(makePlugin('notes', '1.0.0', { deps: ['tasks'], onRegister: async () => { order.push('notes'); } }));
    registerPlugin(makePlugin('tasks', '1.0.0', { onRegister: async () => { order.push('tasks'); } }));
    await loadPlugins(EventBus.init(), [], { timeout: 1000 });
    expect(order.indexOf('tasks')).toBeLessThan(order.indexOf('notes'));
  });

  it('emits loading-finished with all loaded ids', async () => {
    registerPlugin(makePlugin('a'));
    registerPlugin(makePlugin('b'));
    const bus = EventBus.init();
    const handler = mock();
    bus.on('loading-finished', handler, 'test');
    await loadPlugins(bus, [], { timeout: 1000 });
    expect(handler).toHaveBeenCalledWith(
      expect.objectContaining({ loaded: expect.arrayContaining(['a', 'b']) })
    );
  });
});

describe('loadPlugins — timeout', () => {
  it('marks plugin as timedOut if onRegister never resolves', async () => {
    registerPlugin(makePlugin('hang', '1.0.0', { onRegister: () => new Promise(() => {}) }));
    const bus = EventBus.init();
    const handler = mock();
    bus.on('loading-finished', handler, 'test');
    await loadPlugins(bus, [], { timeout: 50 });
    expect(handler).toHaveBeenCalledWith(
      expect.objectContaining({ timedOut: ['hang'], loaded: [] })
    );
  });
});

describe('loadPlugins — cycle detection', () => {
  it('marks cycled plugins as failed', async () => {
    registerPlugin(makePlugin('a', '1.0.0', { deps: ['b'] }));
    registerPlugin(makePlugin('b', '1.0.0', { deps: ['a'] }));
    const bus = EventBus.init();
    const handler = mock();
    bus.on('loading-finished', handler, 'test');
    await loadPlugins(bus, [], { timeout: 1000 });
    const { failed } = handler.mock.calls[0][0] as { failed: string[] };
    expect(failed).toEqual(expect.arrayContaining(['a', 'b']));
  });
});

describe('loadPlugins — requires', () => {
  it('auto-fetches missing required plugin from availablePlugins', async () => {
    registerPlugin(makePlugin('notes', '1.0.0', { requires: ['adi.auth'] }));

    mockFetchChain([{ id: 'adi.auth' }]);

    const bus = EventBus.init();
    const installed = mock();
    bus.on('plugin:installed', installed, 'test');
    const finished = mock();
    bus.on('loading-finished', finished, 'test');

    const availablePlugins: PluginDescriptor[] = [{
      id: 'adi.auth',
      installedVersion: '1.0.0',
      registry: { url: 'https://test', getBundleInfo: async () => ({ jsUrl: 'https://test/adi.auth.js' }), checkLatest: async () => null },
    }];

    await loadPlugins(bus, [], { timeout: 1000, availablePlugins });

    expect(installed).toHaveBeenCalledWith({ pluginId: 'adi.auth', reason: 'auto' });
    const { loaded } = finished.mock.calls[0][0] as { loaded: string[] };
    expect(loaded).toContain('adi.auth');
    expect(loaded).toContain('notes');
  });

  it('initializes required plugin before the requirer', async () => {
    const order: string[] = [];
    registerPlugin(makePlugin('app', '1.0.0', {
      requires: ['auth'],
      onRegister: async () => { order.push('app'); },
    }));
    registerPlugin(makePlugin('auth', '1.0.0', {
      onRegister: async () => { order.push('auth'); },
    }));

    await loadPlugins(EventBus.init(), [], { timeout: 1000 });

    expect(order.indexOf('auth')).toBeLessThan(order.indexOf('app'));
  });

  it('resolves transitive requires', async () => {
    registerPlugin(makePlugin('app', '1.0.0', { requires: ['mid'] }));

    // mid requires base — both must be auto-fetched
    mockFetchChain([
      { id: 'mid', requires: ['base'] },
      { id: 'base' },
    ]);

    const bus = EventBus.init();
    const installed = mock();
    bus.on('plugin:installed', installed, 'test');
    const finished = mock();
    bus.on('loading-finished', finished, 'test');

    const availablePlugins: PluginDescriptor[] = [
      { id: 'mid', installedVersion: '1.0.0', registry: { url: 'https://test', getBundleInfo: async () => ({ jsUrl: 'https://test/mid.js' }), checkLatest: async () => null } },
      { id: 'base', installedVersion: '1.0.0', registry: { url: 'https://test', getBundleInfo: async () => ({ jsUrl: 'https://test/base.js' }), checkLatest: async () => null } },
    ];

    await loadPlugins(bus, [], { timeout: 1000, availablePlugins });

    expect(installed).toHaveBeenCalledTimes(2);
    expect(installed).toHaveBeenCalledWith({ pluginId: 'mid', reason: 'auto' });
    expect(installed).toHaveBeenCalledWith({ pluginId: 'base', reason: 'auto' });

    const { loaded } = finished.mock.calls[0][0] as { loaded: string[] };
    expect(loaded).toContain('mid');
    expect(loaded).toContain('base');
    expect(loaded).toContain('app');
  });

  it('skips already-registered required plugin', async () => {
    registerPlugin(makePlugin('app', '1.0.0', { requires: ['auth'] }));
    registerPlugin(makePlugin('auth'));

    const fetchCalls: string[] = [];
    const origFetch = globalThis.fetch;
    globalThis.fetch = (async (input: RequestInfo | URL) => {
      fetchCalls.push(String(input));
      return new Response('');
    }) as typeof fetch;

    const bus = EventBus.init();
    const installed = mock();
    bus.on('plugin:installed', installed, 'test');

    const availablePlugins: PluginDescriptor[] = [{
      id: 'auth',
      installedVersion: '1.0.0',
      registry: { url: 'https://test', getBundleInfo: async () => ({ jsUrl: 'https://test/auth.js' }), checkLatest: async () => null },
    }];

    await loadPlugins(bus, [], { timeout: 1000, availablePlugins });
    globalThis.fetch = origFetch;

    expect(fetchCalls).toEqual([]);
    expect(installed).not.toHaveBeenCalled();
  });
});

describe('configureApp', () => {
  it('throws if called after plugins are loaded', async () => {
    registerPlugin(makePlugin('a'));
    await loadPlugins(EventBus.init(), [], { timeout: 1000 });
    expect(() => configureApp({})).toThrow('must be called before');
  });
});

describe('initInternalPlugin', () => {
  it('registers and initializes the plugin', async () => {
    const order: string[] = [];
    const plugin = makePlugin('internal', '1.0.0', {
      onRegister: async () => { order.push('registered'); },
    });
    const bus = EventBus.init();
    await initInternalPlugin(bus, plugin);
    expect(order).toEqual(['registered']);
  });
});

describe('loadPlugins — onRegister error', () => {
  it('marks plugin as failed when onRegister throws', async () => {
    registerPlugin(makePlugin('bad', '1.0.0', {
      onRegister: async () => { throw new Error('init boom'); },
    }));
    const bus = EventBus.init();
    const handler = mock();
    bus.on('loading-finished', handler, 'test');
    await loadPlugins(bus, [], { timeout: 1000 });
    const { failed, loaded } = handler.mock.calls[0][0] as { failed: string[]; loaded: string[] };
    expect(failed).toContain('bad');
    expect(loaded).not.toContain('bad');
  });
});
