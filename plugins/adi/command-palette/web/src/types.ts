/**
 * Auto-generated plugin types.
 * Import via: import '@adi-family/plugin-xxx'
 * DO NOT EDIT.
 */

import type { CommandPalettePlugin } from './plugin';

export type { CommandPalettePlugin };
export * from './config';
export * from './generated';

declare module '@adi-family/sdk-plugin' {
  interface PluginApiRegistry {
    'adi.command-palette': CommandPalettePlugin['api'];
  }
}
