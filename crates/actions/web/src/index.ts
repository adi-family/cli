/**
 * Auto-generated plugin entry from Cargo.toml.
 * DO NOT EDIT.
 */

import { PLUGIN_ID } from './config';

export * from './config';

import type { ActionsPlugin } from './plugin';
export { ActionsPlugin, ActionsPlugin as PluginShell } from './plugin';

declare module '@adi-family/sdk-plugin' {
  interface PluginApiRegistry {
    [PLUGIN_ID]: ActionsPlugin['api'];
  }
}
