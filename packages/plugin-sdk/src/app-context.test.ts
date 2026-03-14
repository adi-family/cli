// src/app-context.test.ts
import { describe, it, expect, mock } from 'bun:test';
import { AppContext } from './app-context.js';
import { EventBus } from './bus.js';
import type { PluginStorage } from './storage.js';

function makeCtx(opts?: ConstructorParameters<typeof AppContext>[1]) {
  return new AppContext(EventBus.init(), opts);
}

function makeMockStorage(): PluginStorage {
  const store = new Map<string, unknown>();
  return {
    get: async (key) => store.get(key) as any,
    set: async (key, value) => {
      store.set(key, value);
    },
    delete: async (key) => {
      store.delete(key);
    },
    keys: async () => [...store.keys()],
  };
}

describe('AppContext — env', () => {
  it('reads VITE_ prefixed env and splits by comma', () => {
    const ctx = makeCtx({ envSource: { VITE_FEATURES: 'a,b,c' } });
    expect(ctx.env('FEATURES')).toEqual(['a', 'b', 'c']);
  });

  it('returns empty array for missing key', () => {
    const ctx = makeCtx({ envSource: {} });
    expect(ctx.env('MISSING')).toEqual([]);
  });

  it('filters empty segments', () => {
    const ctx = makeCtx({ envSource: { VITE_X: ',a,,b,' } });
    expect(ctx.env('X')).toEqual(['a', 'b']);
  });

  it('defaults envSource to empty object', () => {
    const ctx = makeCtx();
    expect(ctx.env('ANY')).toEqual([]);
  });
});

describe('AppContext — api provide/retrieve', () => {
  it('provides and retrieves an API', async () => {
    const ctx = makeCtx();
    const api = { doStuff: () => 42 };
    ctx._provide('my-api', api);
    expect(await ctx.api('my-api' as never)).toBe(api);
  });

  it('throws on duplicate provide', () => {
    const ctx = makeCtx();
    ctx._provide('dup', {});
    expect(() => ctx._provide('dup', {})).toThrow(
      "API 'dup' is already registered",
    );
  });

  it('resolves immediately when already provided', async () => {
    const ctx = makeCtx();
    const api = { value: 99 };
    ctx._provide('ready', api);
    const result = await ctx.api('ready' as never);
    expect(result).toBe(api);
  });

  it('waits for a late-provided API', async () => {
    const ctx = makeCtx();
    const api = { late: true };
    const promise = ctx.api('deferred' as never);

    ctx._provide('deferred', api);

    const result = await promise;
    expect(result).toBe(api);
  });

  it('resolves multiple waiters', async () => {
    const ctx = makeCtx();
    const api = { shared: true };
    const p1 = ctx.api('multi' as never);
    const p2 = ctx.api('multi' as never);

    ctx._provide('multi', api);

    const [r1, r2] = await Promise.all([p1, p2]);
    expect(r1).toBe(api);
    expect(r2).toBe(api);
  });

  it('waits after unprovide until re-provided', async () => {
    const ctx = makeCtx();
    ctx._provide('temp', { x: 1 });
    ctx._unprovide('temp');

    const api = { x: 2 };
    const promise = ctx.api('temp' as never);
    ctx._provide('temp', api);
    expect(await promise).toBe(api);
  });
});

describe('AppContext — storage', () => {
  it('creates storage via factory', () => {
    const factory = mock((_id: string) => makeMockStorage());
    const ctx = makeCtx({ storageFactory: factory });
    const s = ctx.storage('my-plugin');
    expect(factory).toHaveBeenCalledWith('my-plugin');
    expect(s).toBeDefined();
  });

  it('caches storage instances per plugin', () => {
    const factory = mock((_id: string) => makeMockStorage());
    const ctx = makeCtx({ storageFactory: factory });
    const s1 = ctx.storage('p1');
    const s2 = ctx.storage('p1');
    expect(s1).toBe(s2);
    expect(factory).toHaveBeenCalledTimes(1);
  });

  it('throws when no storageFactory configured', () => {
    const ctx = makeCtx();
    expect(() => ctx.storage('x')).toThrow('Storage is not configured');
  });
});

describe('AppContext — registered plugins tracking', () => {
  it('tracks registered plugins', () => {
    const ctx = makeCtx();
    ctx._registerPlugin('a');
    ctx._registerPlugin('b');
    expect(ctx.registeredPlugins.has('a')).toBe(true);
    expect(ctx.registeredPlugins.has('b')).toBe(true);
    expect(ctx.registeredPlugins.size).toBe(2);
  });

  it('unregister removes plugin', () => {
    const ctx = makeCtx();
    ctx._registerPlugin('a');
    ctx._unregisterPlugin('a');
    expect(ctx.registeredPlugins.has('a')).toBe(false);
  });
});
