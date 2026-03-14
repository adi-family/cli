import { describe, it, expect, mock, beforeEach, afterEach } from 'bun:test';
import { HttpPluginRegistry } from './registry-http';

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

  describe('getBundleInfo', () => {
    it('fetches version.json and returns resolved URLs', async () => {
      fetchMock.mockResolvedValueOnce({
        ok: true,
        json: async () => ({
          id: 'tasks',
          version: '1.2.0',
          jsUrl: '/v1/tasks/1.2.0/main.js',
          cssUrl: '/v1/tasks/1.2.0/main.css',
          sizeBytes: 1024,
          publishedAt: 1700000000,
          previewImages: [],
        }),
      } as Response);

      const reg = new HttpPluginRegistry(BASE);
      const info = await reg.getBundleInfo('tasks', '1.2.0');
      expect(fetchMock).toHaveBeenCalledWith(`${BASE}/v1/tasks/1.2.0.json`);
      expect(info.jsUrl).toBe(`${BASE}/v1/tasks/1.2.0/main.js`);
      expect(info.cssUrl).toBe(`${BASE}/v1/tasks/1.2.0/main.css`);
    });

    it('returns undefined cssUrl when not present', async () => {
      fetchMock.mockResolvedValueOnce({
        ok: true,
        json: async () => ({
          id: 'tasks',
          version: '1.2.0',
          jsUrl: '/v1/tasks/1.2.0/main.js',
          sizeBytes: 1024,
          publishedAt: 1700000000,
          previewImages: [],
        }),
      } as Response);

      const reg = new HttpPluginRegistry(BASE);
      const info = await reg.getBundleInfo('tasks', '1.2.0');
      expect(info.cssUrl).toBeUndefined();
    });

    it('throws on non-2xx', async () => {
      fetchMock.mockResolvedValueOnce({
        ok: false,
        status: 404,
        statusText: 'Not Found',
      } as Response);

      const reg = new HttpPluginRegistry(BASE);
      await expect(reg.getBundleInfo('tasks', '1.2.0')).rejects.toThrow('getBundleInfo failed: 404');
    });
  });

  describe('checkLatest', () => {
    it('returns null when version matches latest', async () => {
      fetchMock.mockResolvedValueOnce({
        ok: true,
        json: async () => ({ version: '1.2.0', jsUrl: '/v1/tasks/1.2.0/main.js', previewImages: [] }),
      } as Response);

      const reg = new HttpPluginRegistry(BASE);
      const result = await reg.checkLatest('tasks', '1.2.0');
      expect(result).toBeNull();
    });

    it('returns new version when update available', async () => {
      fetchMock.mockResolvedValueOnce({
        ok: true,
        json: async () => ({ version: '1.3.0', jsUrl: '/v1/tasks/1.3.0/main.js', previewImages: [] }),
      } as Response);

      const reg = new HttpPluginRegistry(BASE);
      const result = await reg.checkLatest('tasks', '1.2.0');
      expect(result).toEqual({ version: '1.3.0' });
    });

    it('calls correct latest.json endpoint', async () => {
      fetchMock.mockResolvedValueOnce({
        ok: true,
        json: async () => ({ version: '1.2.0', jsUrl: '/v1/tasks/1.2.0/main.js', previewImages: [] }),
      } as Response);

      const reg = new HttpPluginRegistry(BASE);
      await reg.checkLatest('tasks', '1.2.0');
      expect(fetchMock).toHaveBeenCalledWith(`${BASE}/v1/tasks/latest.json`);
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
        json: async () => ({
          version: 3,
          updatedAt: 1700000000,
          plugins: [
            { id: 'a', name: 'A', description: '', latestVersion: '1.0.0', downloads: 0, author: '', tags: [] },
            { id: 'b', name: 'B', description: '', latestVersion: '1.0.0', downloads: 0, author: '', tags: [] },
          ],
        }),
      } as Response);

      const reg = new HttpPluginRegistry(BASE);
      const health = await reg.checkHealth();
      expect(health.online).toBe(true);
      expect(health.pluginCount).toBe(2);
      expect(health.version).toBe(3);
      expect(typeof health.latencyMs).toBe('number');
      expect(fetchMock).toHaveBeenCalledWith(`${BASE}/v1/index.json`);
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
    it('returns plugin descriptors with metadata from index', async () => {
      fetchMock.mockResolvedValueOnce({
        ok: true,
        json: async () => ({
          version: 1,
          updatedAt: 1700000000,
          plugins: [
            { id: 'tasks', name: 'Tasks', description: 'Task manager', latestVersion: '2.0.0', downloads: 100, author: 'Team', tags: ['tasks'] },
            { id: 'auth', name: 'Auth', description: 'Authentication', latestVersion: '1.0.0', downloads: 50, author: 'Team', tags: ['auth'] },
          ],
        }),
      } as Response);

      const reg = new HttpPluginRegistry(BASE);
      const plugins = await reg.listPlugins();
      expect(plugins).toHaveLength(2);
      expect(plugins[0].id).toBe('tasks');
      expect(plugins[0].installedVersion).toBe('2.0.0');
      expect(plugins[0].name).toBe('Tasks');
      expect(plugins[0].description).toBe('Task manager');
      expect(plugins[0].author).toBe('Team');
      expect(plugins[0].tags).toEqual(['tasks']);
      expect(plugins[0].downloads).toBe(100);
      expect(plugins[1].id).toBe('auth');
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
