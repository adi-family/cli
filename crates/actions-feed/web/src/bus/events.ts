/**
 * Auto-generated eventbus registry from TypeSpec.
 * DO NOT EDIT.
 */

import type { ActionsDismissEvent, ActionsDismissedEvent, ActionsPushEvent, ActionsRegisterKindEvent, ActionsRegisterRendererEvent, CommandExecuteEvent, CommandRegisterEvent, NavAddEvent } from './types';

declare module '@adi-family/sdk-plugin/types' {
  interface EventRegistry {
    // ── adi.actions-feed ──
    'adi.actions-feed:register-kind': ActionsRegisterKindEvent;
    'adi.actions-feed:register-renderer': ActionsRegisterRendererEvent;
    'adi.actions-feed:push': ActionsPushEvent;
    'adi.actions-feed:dismiss': ActionsDismissEvent;
    'adi.actions-feed:dismissed': ActionsDismissedEvent;
    'adi.actions-feed:nav-add': NavAddEvent;

    // ── adi.command-palette ──
    'adi.command-palette:register': CommandRegisterEvent;
    'adi.command-palette:execute': CommandExecuteEvent;
  }
}
