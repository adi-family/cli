import type { PluginBundleInfo, PluginDescriptor, PluginRegistry } from './types.js';

interface WebPluginEntry {
  id: string;
  name: string;
  description: string;
  latestVersion: string;
  downloads: number;
  author: string;
  tags: string[];
}

interface WebRegistryIndex {
  version: number;
  updatedAt: number;
  plugins: WebPluginEntry[];
}

interface WebPluginInfo {
  id: string;
  version: string;
  jsUrl: string;
  cssUrl?: string;
  sizeBytes: number;
  publishedAt: number;
  changelog?: string;
  previewUrl?: string;
  previewImages: string[];
}

export interface RegistryHealth {
  online: boolean;
  pluginCount: number;
  version?: number;
  latencyMs: number;
}

export class HttpPluginRegistry implements PluginRegistry {
  constructor(private readonly baseUrl: string) {}

  get url(): string { return this.baseUrl; }

  async getBundleInfo(id: string, version: string): Promise<PluginBundleInfo> {
    const res = await fetch(`${this.baseUrl}/v1/${id}/${version}.json`);
    if (!res.ok) {
      throw new Error(`getBundleInfo failed: ${res.status} ${res.statusText}`);
    }
    const info = (await res.json()) as WebPluginInfo;
    return {
      jsUrl: `${this.baseUrl}${info.jsUrl}`,
      cssUrl: info.cssUrl ? `${this.baseUrl}${info.cssUrl}` : undefined,
    };
  }

  async checkLatest(
    id: string,
    currentVersion: string
  ): Promise<{ version: string } | null> {
    const res = await fetch(`${this.baseUrl}/v1/${id}/latest.json`);
    if (!res.ok) {
      throw new Error(`checkLatest failed: ${res.status} ${res.statusText}`);
    }
    const info = (await res.json()) as WebPluginInfo;
    return info.version !== currentVersion ? { version: info.version } : null;
  }

  /** Check reachability, plugin count, and server version. Never throws. */
  async checkHealth(): Promise<RegistryHealth> {
    const start = Date.now();
    try {
      const res = await fetch(`${this.baseUrl}/v1/index.json`);
      const latencyMs = Date.now() - start;
      if (!res.ok) return { online: false, pluginCount: 0, latencyMs };
      const data = (await res.json()) as WebRegistryIndex;
      return { online: true, pluginCount: data.plugins.length, version: data.version, latencyMs };
    } catch {
      return { online: false, pluginCount: 0, latencyMs: Date.now() - start };
    }
  }

  /** Fetch all plugins from the registry index. Returns empty array on any failure. */
  async listPlugins(): Promise<PluginDescriptor[]> {
    try {
      const res = await fetch(`${this.baseUrl}/v1/index.json`);
      if (!res.ok) return [];
      const { plugins } = (await res.json()) as WebRegistryIndex;
      return plugins.map(p => ({
        id: p.id,
        name: p.name,
        description: p.description,
        author: p.author,
        tags: p.tags,
        downloads: p.downloads,
        registry: this,
        installedVersion: p.latestVersion,
        latestVersion: p.latestVersion,
      }));
    } catch {
      return [];
    }
  }
}
