/**
 * Auto-generated plugin types.
 * Import via: import '@adi-family/plugin-xxx'
 * DO NOT EDIT.
 */

import type { EmbedProxyPlugin } from './plugin';

export type { EmbedProxyPlugin };
export * from './config';
export * from './generated';

declare module '@adi-family/sdk-plugin' {
  interface PluginApiRegistry {
    'adi.embed-proxy': EmbedProxyPlugin['api'];
  }
}
