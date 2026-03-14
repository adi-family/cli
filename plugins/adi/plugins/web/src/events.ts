import type { PluginItem, RegistryPlugin } from './types.js';

declare module '@adi-family/sdk-plugin/types' {
  interface EventRegistry {
    'plugins:search': { query: string; offset: number; limit: number };
    'plugins:search-changed': { plugins: PluginItem[]; total: number; hasMore: boolean };
    'plugins:install-web': { pluginId: string };
    'plugins:install-cocoon': { pluginId: string; cocoonId: string };
    'plugins:install-result': { pluginId: string; cocoonId?: string; success: boolean; error?: string };
    'plugins:detail': { pluginId: string };
    'plugins:detail-changed': { plugin: RegistryPlugin | null };
  }
}

export {};
