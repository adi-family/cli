/**
 * Auto-generated plugin types.
 * Import via: import '@adi-family/plugin-xxx'
 * DO NOT EDIT.
 */

import type { VideoPlugin } from './plugin';

export type { VideoPlugin };
export * from './config';

declare module '@adi-family/sdk-plugin' {
  interface PluginApiRegistry {
    'adi.video': VideoPlugin['api'];
  }
}
