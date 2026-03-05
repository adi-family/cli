/**
 * Auto-generated plugin entry from Cargo.toml.
 * DO NOT EDIT.
 */

import { PLUGIN_ID } from './config';

import './bus';
export * from './bus';
export * from './config';

import type { CommandPalettePlugin } from './plugin';
export { CommandPalettePlugin, CommandPalettePlugin as PluginShell } from './plugin';

declare module '@adi-family/sdk-plugin' {
  interface PluginApiRegistry {
    [PLUGIN_ID]: CommandPalettePlugin['api'];
  }
}
