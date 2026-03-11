/**
 * Auto-generated eventbus registry from TypeSpec.
 * DO NOT EDIT.
 */

import type { ActionKindMode, ActionPriority, ActionsDismissEvent, ActionsDismissedEvent, ActionsPushEvent, ActionsRegisterKindEvent, ActionsRegisterRendererEvent, CommandExecuteEvent, CommandRegisterEvent, NavAddEvent } from './types';
import { ActionsBusKey, CommandBusKey, NavBusKey } from './types';

declare module '@adi-family/sdk-plugin/types' {
  interface EventRegistry {
    // ── actions ──
    [ActionsBusKey.RegisterKind]: ActionsRegisterKindEvent;
    [ActionsBusKey.RegisterRenderer]: ActionsRegisterRendererEvent;
    [ActionsBusKey.Push]: ActionsPushEvent;
    [ActionsBusKey.Dismiss]: ActionsDismissEvent;
    [ActionsBusKey.Dismissed]: ActionsDismissedEvent;

    // ── nav ──
    [NavBusKey.Add]: NavAddEvent;

    // ── command ──
    [CommandBusKey.Register]: CommandRegisterEvent;
    [CommandBusKey.Execute]: CommandExecuteEvent;
  }
}
