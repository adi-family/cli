// src/registry.test.ts
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { registerPlugin, loadPlugins, _resetRegistry } from './registry.js';
import { AdiPlugin } from './plugin.js';
import { createEventBus } from './bus.js';
import type { PluginDescriptor, PluginRegistry } from './types.js';

function makePlugin(
  id: string,
  version = '1.0.0',
  deps: string[] = [],
  onRegister?: () => Promise<void>
): AdiPlugin {
  class P extends AdiPlugin {
    readonly id = id;
    readonly version = version;
    readonly dependencies = deps;
    onRegister = onRegister;
  }
  return new P();
}

function _makeDescriptor(
  plugin: AdiPlugin,
  registry?: PluginRegistry
): PluginDescriptor {
  const reg: PluginRegistry = registry ?? {
    bundleUrl: async () => 'blob:unused',
    checkLatest: async () => null,
  };
  return { id: plugin.id, registry: reg, installedVersion: plugin.version };
}

void _makeDescriptor; // suppress unused-var — provided for future tests

beforeEach(() => {
  _resetRegistry();
});

describe('registerPlugin', () => {
  it('allows plugin to be found by loadPlugins', async () => {
    const plugin = makePlugin('tasks');
    registerPlugin(plugin);
    const bus = createEventBus();
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
    registerPlugin(makePlugin('a', '1.0.0', [], async () => { order.push('a'); }));
    registerPlugin(makePlugin('b', '1.0.0', [], async () => { order.push('b'); }));
    await loadPlugins(createEventBus(), [], { timeout: 1000 });
    expect(order).toEqual(['a', 'b']);
  });

  it('initializes dependency before dependent regardless of registration order', async () => {
    const order: string[] = [];
    registerPlugin(makePlugin('notes', '1.0.0', ['tasks'], async () => { order.push('notes'); }));
    registerPlugin(makePlugin('tasks', '1.0.0', [], async () => { order.push('tasks'); }));
    await loadPlugins(createEventBus(), [], { timeout: 1000 });
    expect(order.indexOf('tasks')).toBeLessThan(order.indexOf('notes'));
  });

  it('emits loading-finished with all loaded ids', async () => {
    registerPlugin(makePlugin('a'));
    registerPlugin(makePlugin('b'));
    const bus = createEventBus();
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
    registerPlugin(makePlugin('hang', '1.0.0', [], () => new Promise(() => {})));
    const bus = createEventBus();
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
    registerPlugin(makePlugin('a', '1.0.0', ['b']));
    registerPlugin(makePlugin('b', '1.0.0', ['a']));
    const bus = createEventBus();
    const handler = vi.fn();
    bus.on('loading-finished', handler, 'test');
    await loadPlugins(bus, [], { timeout: 1000 });
    const { failed } = handler.mock.calls[0][0] as { failed: string[] };
    expect(failed).toEqual(expect.arrayContaining(['a', 'b']));
  });
});
