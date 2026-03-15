/**
 * Auto-generated eventbus registry from TypeSpec.
 * DO NOT EDIT.
 */

import type { ActionsDismissEvent, ActionsDismissedEvent, ActionsPushEvent, ActionsRegisterKindEvent, ActionsRegisterRendererEvent } from './bus-types';

declare module '@adi-family/sdk-plugin/types' {
  interface EventRegistry {
    // ── actions ──
    'actions:register-kind': ActionsRegisterKindEvent;
    'actions:register-renderer': ActionsRegisterRendererEvent;
    'actions:push': ActionsPushEvent;
    'actions:dismiss': ActionsDismissEvent;
    'actions:dismissed': ActionsDismissedEvent;
  }
}
