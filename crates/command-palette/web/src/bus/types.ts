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
  Register = 'adi.command-palette:register',
  Execute = 'adi.command-palette:execute',
}

export enum CommandPaletteBusKey {
  Open = 'adi.command-palette:open',
}
