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

export interface RouteRegisterEvent {
  path: string;
  element: string;
  label?: string;
}

export interface NavAddEvent {
  id: string;
  label: string;
  path: string;
  icon?: string;
}

export interface RouterNavigateEvent {
  path: string;
  replace?: boolean;
}

export interface CommandRegisterEvent {
  id: string;
  label: string;
  shortcut?: string;
}

export interface CommandExecuteEvent {
  id: string;
}
