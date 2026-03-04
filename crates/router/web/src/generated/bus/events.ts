/**
 * Auto-generated eventbus registry from TypeSpec.
 * DO NOT EDIT.
 */

import type { CommandExecuteEvent, CommandRegisterEvent, NavAddEvent, RouteRegisterEvent, RouterChangedEvent, RouterNavigateEvent } from './types';

declare module '@adi-family/sdk-plugin/types' {
  interface EventRegistry {
    // ── route ──
    'route:register': RouteRegisterEvent;

    // ── nav ──
    'nav:add': NavAddEvent;

    // ── router ──
    'router:navigate': RouterNavigateEvent;
    'router:changed': RouterChangedEvent;

    // ── command ──
    'command:register': CommandRegisterEvent;
    'command:execute': CommandExecuteEvent;
  }
}
