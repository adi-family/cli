// src/plugin.test.ts
import { describe, it, expect, spyOn, mock } from 'bun:test';
import { AdiPlugin } from './plugin.js';
import { EventBus } from './bus.js';
import { AppContext } from './app-context.js';

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
    this.bus.emit('app:theme-changed', { theme: 'dark', mode: 'dark' }, 'plugin:hooks');
  }

  async onUnregister() {
    this.unregisterCalls.push('called');
  }
}

function makeApp(): AppContext {
  return new AppContext(EventBus.init());
}

describe('AdiPlugin', () => {
  it('has empty dependencies by default', () => {
    expect(new MinimalPlugin().dependencies).toEqual([]);
  });

  it('can declare dependencies', () => {
    expect(new HooksPlugin().dependencies).toEqual(['other']);
  });

  it('_init() injects app — this.bus is accessible in onRegister()', async () => {
    const app = makeApp();
    const emitSpy = spyOn(app.bus, 'emit');
    const plugin = new HooksPlugin();
    await plugin._init(app);
    expect(emitSpy).toHaveBeenCalledWith('app:theme-changed', { theme: 'dark', mode: 'dark' }, 'plugin:hooks');
  });

  it('_init() calls onRegister()', async () => {
    const plugin = new HooksPlugin();
    await plugin._init(makeApp());
    expect(plugin.registerCalls).toEqual(['called']);
  });

  it('_init() emits register-finished after onRegister resolves', async () => {
    const app = makeApp();
    const handler = mock();
    app.bus.on('register-finished', handler, 'test');
    await new HooksPlugin()._init(app);
    expect(handler).toHaveBeenCalledWith({ pluginId: 'hooks' });
  });

  it('_init() emits register-finished even without onRegister defined', async () => {
    const app = makeApp();
    const handler = mock();
    app.bus.on('register-finished', handler, 'test');
    await new MinimalPlugin()._init(app);
    expect(handler).toHaveBeenCalledWith({ pluginId: 'minimal' });
  });

  it('_destroy() calls onUnregister()', async () => {
    const plugin = new HooksPlugin();
    await plugin._init(makeApp());
    await plugin._destroy();
    expect(plugin.unregisterCalls).toEqual(['called']);
  });

  it('accessing bus before _init() throws a clear error', () => {
    const plugin = new HooksPlugin();
    expect(() => (plugin as unknown as { bus: unknown }).bus).toThrow("accessed app before _init()");
  });
});
