// src/app-context.test.ts
import { describe, it, expect, mock } from 'bun:test';
import { AppContext } from './app-context.js';
import { EventBus } from './bus.js';
function makeCtx(opts) {
    return new AppContext(EventBus.init(), opts);
}
function makeMockStorage() {
    const store = new Map();
    return {
        get: async (key) => store.get(key),
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
    it('provides and retrieves an API', () => {
        const ctx = makeCtx();
        const api = { doStuff: () => 42 };
        ctx._provide('my-api', api);
        expect(ctx.api('my-api')).toBe(api);
    });
    it('throws when accessing unregistered API', () => {
        const ctx = makeCtx();
        expect(() => ctx.api('missing')).toThrow("API 'missing' is not registered");
    });
    it('throws on duplicate provide', () => {
        const ctx = makeCtx();
        ctx._provide('dup', {});
        expect(() => ctx._provide('dup', {})).toThrow("API 'dup' is already registered");
    });
    it('unprovide removes the API', () => {
        const ctx = makeCtx();
        ctx._provide('temp', { x: 1 });
        ctx._unprovide('temp');
        expect(() => ctx.api('temp')).toThrow("API 'temp' is not registered");
    });
});
describe('AppContext — storage', () => {
    it('creates storage via factory', () => {
        const factory = mock((_id) => makeMockStorage());
        const ctx = makeCtx({ storageFactory: factory });
        const s = ctx.storage('my-plugin');
        expect(factory).toHaveBeenCalledWith('my-plugin');
        expect(s).toBeDefined();
    });
    it('caches storage instances per plugin', () => {
        const factory = mock((_id) => makeMockStorage());
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
