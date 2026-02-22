// src/plugin.test.ts
import { describe, it, expect, vi } from 'vitest';
import { AdiPlugin } from './plugin.js';
import { createEventBus } from './bus.js';

class MinimalPlugin extends AdiPlugin {
  readonly id = 'minimal';
  readonly version = '1.0.0';
}

class HooksPlugin extends AdiPlugin {
  readonly id = 'hooks';
  readonly version = '2.0.0';
  readonly dependencies = ['other'];

  readonly registerCalls: string[] = [];
  readonly unregisterCalls: string[] = [];

  async onRegister() {
    this.registerCalls.push('called');
    this.bus.emit('route:register', { path: '/hooks', element: 'hooks-view' });
  }

  async onUnregister() {
    this.unregisterCalls.push('called');
  }
}

describe('AdiPlugin', () => {
  it('has empty dependencies by default', () => {
    expect(new MinimalPlugin().dependencies).toEqual([]);
  });

  it('can declare dependencies', () => {
    expect(new HooksPlugin().dependencies).toEqual(['other']);
  });

  it('_init() injects bus — onRegister can call this.bus.emit without throwing', async () => {
    const bus = createEventBus();
    const plugin = new HooksPlugin();
    await plugin._init(bus);
    expect(plugin.registerCalls).toEqual(['called']);
  });

  it('_init() calls onRegister()', async () => {
    const bus = createEventBus();
    const plugin = new HooksPlugin();
    await plugin._init(bus);
    expect(plugin.registerCalls).toEqual(['called']);
  });

  it('_init() emits register-finished after onRegister resolves', async () => {
    const bus = createEventBus();
    const handler = vi.fn();
    bus.on('register-finished', handler);
    await new HooksPlugin()._init(bus);
    expect(handler).toHaveBeenCalledWith({ pluginId: 'hooks' });
  });

  it('_init() emits register-finished even without onRegister defined', async () => {
    const bus = createEventBus();
    const handler = vi.fn();
    bus.on('register-finished', handler);
    await new MinimalPlugin()._init(bus);
    expect(handler).toHaveBeenCalledWith({ pluginId: 'minimal' });
  });

  it('_destroy() calls onUnregister()', async () => {
    const bus = createEventBus();
    const plugin = new HooksPlugin();
    await plugin._init(bus);
    await plugin._destroy();
    expect(plugin.unregisterCalls).toEqual(['called']);
  });
});
