/**
 * Auto-generated plugin types.
 * Import via: import '@adi-family/plugin-xxx'
 * DO NOT EDIT.
 */

import type { DebugScreenPlugin } from './plugin';

export type { DebugScreenPlugin };
export * from './config';
export * from './generated';

declare module '@adi-family/sdk-plugin' {
  interface PluginApiRegistry {
    'adi.debug-screen': DebugScreenPlugin['api'];
  }
}
