// src/registry-cocoon.ts
import type { PluginRegistry } from './types.js';

export class CocoonPluginRegistry implements PluginRegistry {
  constructor(private readonly baseUrl: string) {}

  async fetchBundle(id: string, version: string): Promise<string> {
    return `${this.baseUrl}/v1/plugins/${id}/${version}/web.js`;
  }

  async checkLatest(
    id: string,
    currentVersion: string
  ): Promise<{ version: string } | null> {
    const res = await fetch(`${this.baseUrl}/v1/plugins/${id}/latest`);
    const { version } = (await res.json()) as { version: string };
    return version !== currentVersion ? { version } : null;
  }
}
