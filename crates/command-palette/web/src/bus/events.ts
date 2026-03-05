/**
 * Auto-generated eventbus registry from TypeSpec.
 * DO NOT EDIT.
 */

import type { CommandExecuteEvent, CommandPaletteOpenEvent, CommandRegisterEvent } from './types';
import { CommandBusKey, CommandPaletteBusKey } from './types';

declare module '@adi-family/sdk-plugin/types' {
  interface EventRegistry {
    // ── command ──
    [CommandBusKey.Register]: CommandRegisterEvent;
    [CommandBusKey.Execute]: CommandExecuteEvent;

    // ── command-palette ──
    [CommandPaletteBusKey.Open]: CommandPaletteOpenEvent;
  }
}
