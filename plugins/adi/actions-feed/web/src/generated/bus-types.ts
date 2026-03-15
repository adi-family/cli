/**
 * Auto-generated eventbus types from TypeSpec.
 * DO NOT EDIT.
 */

import { ActionKindMode, ActionPriority } from './enums';

export interface ActionsRegisterKindEvent {
  plugin: string;
  kind: string;
  mode: ActionKindMode;
}

export interface ActionsRegisterRendererEvent {
  plugin: string;
  kind: string;
  render: unknown;
}

export interface ActionsPushEvent {
  id: string;
  plugin: string;
  kind: string;
  data: Record<string, unknown>;
  priority?: ActionPriority;
}

export interface ActionsDismissEvent {
  id: string;
}

export interface ActionsDismissedEvent {
  id: string;
  plugin: string;
  kind: string;
}

export enum ActionsBusKey {
  RegisterKind = 'actions:register-kind',
  RegisterRenderer = 'actions:register-renderer',
  Push = 'actions:push',
  Dismiss = 'actions:dismiss',
  Dismissed = 'actions:dismissed',
}
