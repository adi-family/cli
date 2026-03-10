/**
 * Auto-generated plugin entry from Cargo.toml.
 * DO NOT EDIT.
 */

import { PLUGIN_ID } from './config';

import './bus';
export * from './bus';
export * from './config';

import type { CredentialsPlugin } from './plugin';
export { CredentialsPlugin, CredentialsPlugin as PluginShell } from './plugin';

declare module '@adi-family/sdk-plugin' {
  interface PluginApiRegistry {
    [PLUGIN_ID]: CredentialsPlugin['api'];
  }
}
