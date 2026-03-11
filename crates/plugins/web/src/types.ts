/**
 * Auto-generated plugin types.
 * Import via: import '@adi-family/plugin-xxx'
 * DO NOT EDIT.
 */

import type { PluginsPlugin } from './plugin';

export type { PluginsPlugin };
export * from './config';

declare module '@adi-family/sdk-plugin' {
  interface PluginApiRegistry {
    'adi.plugins': PluginsPlugin['api'];
  }
}
