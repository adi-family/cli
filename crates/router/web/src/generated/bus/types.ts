/**
 * Auto-generated eventbus types from TypeSpec.
 * DO NOT EDIT.
 */

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

export interface RouterChangedEvent {
  path: string;
  params: Record<string, string>;
}

export interface CommandRegisterEvent {
  id: string;
  label: string;
  shortcut?: string;
}

export interface CommandExecuteEvent {
  id: string;
}
