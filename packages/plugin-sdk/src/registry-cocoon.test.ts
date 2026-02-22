// src/registry-cocoon.test.ts
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { CocoonPluginRegistry } from './registry-cocoon.js';

const BASE = 'https://cocoon.example.com';

describe('CocoonPluginRegistry', () => {
  describe('fetchBundle', () => {
    it('returns correct bundle URL', async () => {
      const reg = new CocoonPluginRegistry(BASE);
      const url = await reg.fetchBundle('tasks', '1.2.0');
      expect(url).toBe(`${BASE}/v1/plugins/tasks/1.2.0/web.js`);
    });
  });

  describe('checkLatest', () => {
    beforeEach(() => {
      vi.stubGlobal('fetch', vi.fn());
    });

    it('returns null when version matches latest', async () => {
      vi.mocked(fetch).mockResolvedValueOnce({
        json: async () => ({ version: '1.2.0' }),
      } as Response);

      const reg = new CocoonPluginRegistry(BASE);
      const result = await reg.checkLatest('tasks', '1.2.0');
      expect(result).toBeNull();
    });

    it('returns new version when update available', async () => {
      vi.mocked(fetch).mockResolvedValueOnce({
        json: async () => ({ version: '1.3.0' }),
      } as Response);

      const reg = new CocoonPluginRegistry(BASE);
      const result = await reg.checkLatest('tasks', '1.2.0');
      expect(result).toEqual({ version: '1.3.0' });
    });

    it('calls correct latest endpoint', async () => {
      vi.mocked(fetch).mockResolvedValueOnce({
        json: async () => ({ version: '1.2.0' }),
      } as Response);

      const reg = new CocoonPluginRegistry(BASE);
      await reg.checkLatest('tasks', '1.2.0');
      expect(fetch).toHaveBeenCalledWith(`${BASE}/v1/plugins/tasks/latest`);
    });
  });
});
