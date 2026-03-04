/**
 * Auto-generated eventbus types from TypeSpec.
 * DO NOT EDIT.
 */

export interface CommandRegisterEvent {
  id: string;
  label: string;
  shortcut?: string;
}

export interface CommandExecuteEvent {
  id: string;
}

export interface RouterNavigateEvent {
  path: string;
  replace?: boolean;
}
