/**
 * Auto-generated eventbus registry from TypeSpec.
 * DO NOT EDIT.
 */

import type { ActionKindMode, ActionPriority, ActionsDismissEvent, ActionsDismissedEvent, ActionsPushEvent, ActionsRegisterKindEvent, ActionsRegisterRendererEvent, CommandExecuteEvent, CommandRegisterEvent, NavAddEvent } from './types';

declare module '@adi-family/sdk-plugin/types' {
  interface EventRegistry {
    // ── actions ──
    'actions:register-kind': ActionsRegisterKindEvent;
    'actions:register-renderer': ActionsRegisterRendererEvent;
    'actions:push': ActionsPushEvent;
    'actions:dismiss': ActionsDismissEvent;
    'actions:dismissed': ActionsDismissedEvent;

    // ── nav ──
    'nav:add': NavAddEvent;

    // ── command ──
    'command:register': CommandRegisterEvent;
    'command:execute': CommandExecuteEvent;
  }
}
