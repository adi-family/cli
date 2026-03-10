/**
 * Auto-generated eventbus registry from TypeSpec.
 * DO NOT EDIT.
 */

import type { CommandExecuteEvent, CommandPaletteOpenEvent, CommandRegisterEvent } from './types';

declare module '@adi-family/sdk-plugin/types' {
  interface EventRegistry {
    // ── command ──
    'command:register': CommandRegisterEvent;
    'command:execute': CommandExecuteEvent;

    // ── command-palette ──
    'command-palette:open': CommandPaletteOpenEvent;
  }
}
