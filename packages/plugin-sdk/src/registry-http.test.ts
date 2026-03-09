// src/registry-http.test.ts
import { describe, it, expect, mock, beforeEach, afterEach } from 'bun:test';
import { HttpPluginRegistry } from './registry-http.js';

const BASE = 'https://registry.example.com';

describe('HttpPluginRegistry', () => {
  let fetchMock: ReturnType<typeof mock>;
  const origFetch = globalThis.fetch;

  beforeEach(() => {
    fetchMock = mock();
    globalThis.fetch = fetchMock as typeof fetch;
  });

  afterEach(() => {
    globalThis.fetch = origFetch;
  });

  describe('url getter', () => {
    it('returns the base URL', () => {
      const reg = new HttpPluginRegistry(BASE);
      expect(reg.url).toBe(BASE);
    });
  });

  describe('bundleUrl', () => {
    it('returns correct bundle URL', async () => {
      const reg = new HttpPluginRegistry(BASE);
      const url = await reg.bundleUrl('tasks', '1.2.0');
      expect(url).toBe(`${BASE}/v1/plugins/tasks/1.2.0/web.js`);
    });
  });

  describe('checkLatest', () => {
    it('returns null when version matches latest', async () => {
      fetchMock.mockResolvedValueOnce({
        ok: true,
        json: async () => ({ version: '1.2.0' }),
      } as Response);

      const reg = new HttpPluginRegistry(BASE);
      const result = await reg.checkLatest('tasks', '1.2.0');
      expect(result).toBeNull();
    });

    it('returns new version when update available', async () => {
      fetchMock.mockResolvedValueOnce({
        ok: true,
        json: async () => ({ version: '1.3.0' }),
      } as Response);

      const reg = new HttpPluginRegistry(BASE);
      const result = await reg.checkLatest('tasks', '1.2.0');
      expect(result).toEqual({ version: '1.3.0' });
    });

    it('calls correct latest endpoint', async () => {
      fetchMock.mockResolvedValueOnce({
        ok: true,
        json: async () => ({ version: '1.2.0' }),
      } as Response);

      const reg = new HttpPluginRegistry(BASE);
      await reg.checkLatest('tasks', '1.2.0');
      expect(fetchMock).toHaveBeenCalledWith(`${BASE}/v1/plugins/tasks/latest`);
    });

    it('throws when registry returns non-2xx', async () => {
      fetchMock.mockResolvedValueOnce({
        ok: false,
        status: 404,
        statusText: 'Not Found',
        json: async () => ({}),
      } as Response);

      const reg = new HttpPluginRegistry(BASE);
      await expect(reg.checkLatest('tasks', '1.2.0')).rejects.toThrow('checkLatest failed: 404');
    });
  });

  describe('checkHealth', () => {
    it('returns online with plugin count on success', async () => {
      fetchMock.mockResolvedValueOnce({
        ok: true,
        json: async () => ({ plugins: [{ id: 'a' }, { id: 'b' }], version: 3 }),
      } as Response);

      const reg = new HttpPluginRegistry(BASE);
      const health = await reg.checkHealth();
      expect(health.online).toBe(true);
      expect(health.pluginCount).toBe(2);
      expect(health.version).toBe(3);
      expect(typeof health.latencyMs).toBe('number');
    });

    it('returns offline on non-2xx', async () => {
      fetchMock.mockResolvedValueOnce({ ok: false } as Response);

      const reg = new HttpPluginRegistry(BASE);
      const health = await reg.checkHealth();
      expect(health.online).toBe(false);
      expect(health.pluginCount).toBe(0);
    });

    it('returns offline on network error', async () => {
      fetchMock.mockRejectedValueOnce(new Error('network down'));

      const reg = new HttpPluginRegistry(BASE);
      const health = await reg.checkHealth();
      expect(health.online).toBe(false);
      expect(health.pluginCount).toBe(0);
    });
  });

  describe('listPlugins', () => {
    it('returns plugin descriptors from index', async () => {
      fetchMock.mockResolvedValueOnce({
        ok: true,
        json: async () => ({
          plugins: [
            { id: 'tasks', latestVersion: '2.0.0', pluginTypes: ['web'] },
            { id: 'auth', latestVersion: '1.0.0' },
          ],
        }),
      } as Response);

      const reg = new HttpPluginRegistry(BASE);
      const plugins = await reg.listPlugins();
      expect(plugins).toHaveLength(2);
      expect(plugins[0].id).toBe('tasks');
      expect(plugins[0].installedVersion).toBe('2.0.0');
      expect(plugins[0].pluginTypes).toEqual(['web']);
      expect(plugins[1].id).toBe('auth');
      expect(plugins[1].pluginTypes).toBeUndefined();
    });

    it('returns empty array on non-2xx', async () => {
      fetchMock.mockResolvedValueOnce({ ok: false } as Response);

      const reg = new HttpPluginRegistry(BASE);
      expect(await reg.listPlugins()).toEqual([]);
    });

    it('returns empty array on network error', async () => {
      fetchMock.mockRejectedValueOnce(new Error('offline'));

      const reg = new HttpPluginRegistry(BASE);
      expect(await reg.listPlugins()).toEqual([]);
    });
  });
});
