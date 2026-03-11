/**
 * Auto-generated plugin types.
 * Import via: import '@adi-family/plugin-xxx'
 * DO NOT EDIT.
 */

import type { TasksPlugin } from './plugin';

export type { TasksPlugin };
export * from './config';
export * from './generated';

declare module '@adi-family/sdk-plugin' {
  interface PluginApiRegistry {
    'adi.tasks': TasksPlugin['api'];
  }
}
