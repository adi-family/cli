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

export interface CommandPaletteOpenEvent {
  query?: string;
}

export enum CommandBusKey {
  Register = 'command:register',
  Execute = 'command:execute',
}

export enum CommandPaletteBusKey {
  Open = 'command-palette:open',
}
