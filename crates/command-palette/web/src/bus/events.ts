/**
 * Auto-generated eventbus registry from TypeSpec.
 * DO NOT EDIT.
 */

import type { CommandExecuteEvent, CommandPaletteOpenEvent, CommandRegisterEvent } from './types';

declare module '@adi-family/sdk-plugin/types' {
  interface EventRegistry {
    // ── adi.command-palette ──
    'adi.command-palette:register': CommandRegisterEvent;
    'adi.command-palette:execute': CommandExecuteEvent;
    'adi.command-palette:open': CommandPaletteOpenEvent;
  }
}
