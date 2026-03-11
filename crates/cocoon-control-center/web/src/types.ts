/**
 * Auto-generated plugin types.
 * Import via: import '@adi-family/plugin-xxx'
 * DO NOT EDIT.
 */

import type { CocoonControlCenterPlugin } from './plugin';

export type { CocoonControlCenterPlugin };
export * from './config';

declare module '@adi-family/sdk-plugin' {
  interface PluginApiRegistry {
    'adi.cocoon-control-center': CocoonControlCenterPlugin['api'];
  }
}
