/**
 * Auto-generated plugin entry from Cargo.toml.
 * DO NOT EDIT.
 */

import { PLUGIN_ID } from './config';

export * from './config';

import type { MonacoEditorPlugin } from './plugin';
export { MonacoEditorPlugin, MonacoEditorPlugin as PluginShell } from './plugin';

declare module '@adi-family/sdk-plugin' {
  interface PluginApiRegistry {
    [PLUGIN_ID]: MonacoEditorPlugin['api'];
  }
}
