/**
 * Auto-generated eventbus registry.
 * DO NOT EDIT.
 */

import type { CocoonConnectionAddedEvent, CocoonConnectionRemovedEvent } from './bus-keys';

declare module '@adi-family/sdk-plugin/types' {
  interface EventRegistry {
    // ── adi.cocoon ──
    'adi.cocoon:connection-added': CocoonConnectionAddedEvent;
    'adi.cocoon:connection-removed': CocoonConnectionRemovedEvent;
  }
}
