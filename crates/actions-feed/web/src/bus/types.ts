/**
 * Auto-generated eventbus types from TypeSpec.
 * DO NOT EDIT.
 */

export enum ActionPriority {
  Low = "low",
  Normal = "normal",
  Urgent = "urgent",
}

export enum ActionKindMode {
  Exclusive = "exclusive",
}

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

export interface NavAddEvent {
  id: string;
  label: string;
  path: string;
  icon?: string;
}

export interface CommandRegisterEvent {
  id: string;
  label: string;
  shortcut?: string;
}

export interface CommandExecuteEvent {
  id: string;
}

export enum ActionsBusKey {
  RegisterKind = 'actions:register-kind',
  RegisterRenderer = 'actions:register-renderer',
  Push = 'actions:push',
  Dismiss = 'actions:dismiss',
  Dismissed = 'actions:dismissed',
}

export enum CommandBusKey {
  Register = 'command:register',
  Execute = 'command:execute',
}

export enum NavBusKey {
  Add = 'nav:add',
}
