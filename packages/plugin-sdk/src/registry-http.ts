// src/registry-http.ts
import type { PluginDescriptor, PluginRegistry } from './types.js';

interface RegistryIndexEntry {
  id: string;
  latestVersion: string;
  pluginTypes?: string[];
}

interface RegistryIndex {
  plugins: RegistryIndexEntry[];
  version?: number;
}

export interface RegistryHealth {
  online: boolean;
  pluginCount: number;
  /** Reported by the registry server if it includes a `version` field in the index. */
  version?: number;
  latencyMs: number;
}

export class HttpPluginRegistry implements PluginRegistry {
  constructor(private readonly baseUrl: string) {}

  get url(): string { return this.baseUrl; }

  async bundleUrl(id: string, version: string): Promise<string> {
    return `${this.baseUrl}/v1/plugins/${id}/${version}/web.js`;
  }

  async checkLatest(
    id: string,
    currentVersion: string
  ): Promise<{ version: string } | null> {
    const res = await fetch(`${this.baseUrl}/v1/plugins/${id}/latest`);
    if (!res.ok) {
      throw new Error(`checkLatest failed: ${res.status} ${res.statusText}`);
    }
    const { version } = (await res.json()) as { version: string };
    return version !== currentVersion ? { version } : null;
  }

  /** Check reachability, plugin count, and optional server version. Never throws. */
  async checkHealth(): Promise<RegistryHealth> {
    const start = Date.now();
    try {
      const res = await fetch(`${this.baseUrl}/v1/index`);
      const latencyMs = Date.now() - start;
      if (!res.ok) return { online: false, pluginCount: 0, latencyMs };
      const data = (await res.json()) as RegistryIndex;
      return { online: true, pluginCount: data.plugins.length, version: data.version, latencyMs };
    } catch {
      return { online: false, pluginCount: 0, latencyMs: Date.now() - start };
    }
  }

  /** Fetch all plugins from the registry index. Returns empty array on any failure. */
  async listPlugins(): Promise<PluginDescriptor[]> {
    try {
      const res = await fetch(`${this.baseUrl}/v1/index`);
      if (!res.ok) return [];
      const { plugins } = (await res.json()) as RegistryIndex;
      return plugins.map(p => ({
        id: p.id,
        registry: this,
        installedVersion: p.latestVersion,
        latestVersion: p.latestVersion,
        pluginTypes: p.pluginTypes,
      }));
    } catch {
      return [];
    }
  }
}
