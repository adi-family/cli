// src/registry.test.ts
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { registerPlugin, loadPlugins, _resetRegistry } from './registry.js';
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
 * Set up fetch + URL mocks so fetchAndImport works in Node.
 * Each call to fetchAndImport pops the next data URL from the queue.
 */
function mockFetchChain(
  pluginDefs: Array<{ id: string; version?: string; requires?: string[] }>
): void {
  const dataUrls = pluginDefs.map((def) => {
    const code = [
      `export class PluginShell {`,
      `  id = '${def.id}';`,
      `  version = '${def.version ?? '1.0.0'}';`,
      `  dependencies = [];`,
      `  requires = ${JSON.stringify(def.requires ?? [])};`,
      `  _init() { return Promise.resolve(); }`,
      `  _destroy() { return Promise.resolve(); }`,
      `}`,
    ].join('\n');
    return `data:text/javascript,${encodeURIComponent(code)}`;
  });

  let idx = 0;
  vi.spyOn(globalThis, 'fetch').mockImplementation(async () => new Response(new Blob([''])));
  vi.spyOn(URL, 'createObjectURL').mockImplementation(() => dataUrls[idx++]);
  vi.spyOn(URL, 'revokeObjectURL').mockImplementation(() => {});
}

beforeEach(() => {
  _resetRegistry();
});

afterEach(() => {
  vi.restoreAllMocks();
});

describe('registerPlugin', () => {
  it('allows plugin to be found by loadPlugins', async () => {
    const plugin = makePlugin('tasks');
    registerPlugin(plugin);
    const bus = EventBus.init();
    const handler = vi.fn();
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
    const handler = vi.fn();
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
    const handler = vi.fn();
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
    const handler = vi.fn();
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
    const installed = vi.fn();
    bus.on('plugin:installed', installed, 'test');
    const finished = vi.fn();
    bus.on('loading-finished', finished, 'test');

    const availablePlugins: PluginDescriptor[] = [{
      id: 'adi.auth',
      installedVersion: '1.0.0',
      registry: { bundleUrl: async () => 'https://test/adi.auth.js', checkLatest: async () => null },
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
    const installed = vi.fn();
    bus.on('plugin:installed', installed, 'test');
    const finished = vi.fn();
    bus.on('loading-finished', finished, 'test');

    const availablePlugins: PluginDescriptor[] = [
      { id: 'mid', installedVersion: '1.0.0', registry: { bundleUrl: async () => 'https://test/mid.js', checkLatest: async () => null } },
      { id: 'base', installedVersion: '1.0.0', registry: { bundleUrl: async () => 'https://test/base.js', checkLatest: async () => null } },
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

    const fetchSpy = vi.spyOn(globalThis, 'fetch');

    const bus = EventBus.init();
    const installed = vi.fn();
    bus.on('plugin:installed', installed, 'test');

    const availablePlugins: PluginDescriptor[] = [{
      id: 'auth',
      installedVersion: '1.0.0',
      registry: { bundleUrl: async () => 'https://test/auth.js', checkLatest: async () => null },
    }];

    await loadPlugins(bus, [], { timeout: 1000, availablePlugins });

    expect(fetchSpy).not.toHaveBeenCalled();
    expect(installed).not.toHaveBeenCalled();
  });
});
