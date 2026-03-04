/**
 * Auto-generated eventbus registry from TypeSpec.
 * DO NOT EDIT.
 */

import type { ActionKindMode, ActionPriority, ActionsDismissEvent, ActionsDismissedEvent, ActionsPushEvent, ActionsRegisterKindEvent, CommandExecuteEvent, CommandRegisterEvent, NavAddEvent, RouteRegisterEvent, RouterNavigateEvent } from './types';

declare module '@adi-family/sdk-plugin/types' {
  interface EventRegistry {
    // ── actions ──
    'actions:register-kind': ActionsRegisterKindEvent;
    'actions:push': ActionsPushEvent;
    'actions:dismiss': ActionsDismissEvent;
    'actions:dismissed': ActionsDismissedEvent;

    // ── route ──
    'route:register': RouteRegisterEvent;

    // ── nav ──
    'nav:add': NavAddEvent;

    // ── router ──
    'router:navigate': RouterNavigateEvent;

    // ── command ──
    'command:register': CommandRegisterEvent;
    'command:execute': CommandExecuteEvent;
  }
}
