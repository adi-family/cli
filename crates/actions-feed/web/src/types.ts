/**
 * Auto-generated plugin types.
 * Import via: import '@adi-family/plugin-xxx'
 * DO NOT EDIT.
 */

import type { ActionsFeedPlugin } from './plugin';

export type { ActionsFeedPlugin };
export * from './config';
export * from './generated';

declare module '@adi-family/sdk-plugin' {
  interface PluginApiRegistry {
    'adi.actions-feed': ActionsFeedPlugin['api'];
  }
}
