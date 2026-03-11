/**
 * Auto-generated plugin types.
 * Import via: import '@adi-family/plugin-xxx'
 * DO NOT EDIT.
 */

import type { PaymentPlugin } from './plugin';

export type { PaymentPlugin };
export * from './config';

declare module '@adi-family/sdk-plugin' {
  interface PluginApiRegistry {
    'adi.payment': PaymentPlugin['api'];
  }
}
