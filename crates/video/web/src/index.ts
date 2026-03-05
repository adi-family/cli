/**
 * Auto-generated plugin entry from Cargo.toml.
 * DO NOT EDIT.
 */

import { PLUGIN_ID } from './config';

export * from './config';

import type { VideoPlugin } from './plugin';
export { VideoPlugin, VideoPlugin as PluginShell } from './plugin';

declare module '@adi-family/sdk-plugin' {
  interface PluginApiRegistry {
    [PLUGIN_ID]: VideoPlugin['api'];
  }
}
