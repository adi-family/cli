/**
 * Auto-generated plugin types.
 * Import via: import '@adi-family/plugin-xxx'
 * DO NOT EDIT.
 */

import type { LlmProxyPlugin } from './plugin';

export type { LlmProxyPlugin };
export * from './config';
export * from './generated';

declare module '@adi-family/sdk-plugin' {
  interface PluginApiRegistry {
    'adi.llm-proxy': LlmProxyPlugin['api'];
  }
}
