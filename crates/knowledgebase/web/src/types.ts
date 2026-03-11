/**
 * Auto-generated plugin types.
 * Import via: import '@adi-family/plugin-xxx'
 * DO NOT EDIT.
 */

import type { KnowledgebasePlugin } from './plugin';

export type { KnowledgebasePlugin };
export * from './config';
export * from './generated';

declare module '@adi-family/sdk-plugin' {
  interface PluginApiRegistry {
    'adi.knowledgebase': KnowledgebasePlugin['api'];
  }
}
