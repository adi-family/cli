/**
 * Auto-generated plugin entry from Cargo.toml.
 * DO NOT EDIT.
 */

import { PLUGIN_ID } from './config';

export * from './config';

import type { PluginsPlugin } from './plugin';
export { PluginsPlugin, PluginsPlugin as PluginShell } from './plugin';

declare module '@adi-family/sdk-plugin' {
  interface PluginApiRegistry {
    [PLUGIN_ID]: PluginsPlugin['api'];
  }
}
