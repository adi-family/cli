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
  mode: `${ActionKindMode}`;
}

export interface ActionsRegisterRendererEvent {
  plugin: string;
  kind: string;
  render: (data: Record<string, unknown>, actionId: string) => string;
}

export interface ActionsPushEvent {
  id: string;
  plugin: string;
  kind: string;
  data: Record<string, unknown>;
  priority?: `${ActionPriority}`;
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
  RegisterKind = 'adi.actions-feed:register-kind',
  RegisterRenderer = 'adi.actions-feed:register-renderer',
  Push = 'adi.actions-feed:push',
  Dismiss = 'adi.actions-feed:dismiss',
  Dismissed = 'adi.actions-feed:dismissed',
}

export enum CommandBusKey {
  Register = 'adi.command-palette:register',
  Execute = 'adi.command-palette:execute',
}

export enum NavBusKey {
  Add = 'adi.actions-feed:nav-add',
}
