/**
 * Auto-generated eventbus registry from TypeSpec.
 * DO NOT EDIT.
 */

import type { SlotsChangedEvent, SlotsDefineEvent, SlotsPlaceEvent, SlotsRemoveAllEvent, SlotsRemoveEvent } from './bus-types';

declare module '@adi-family/sdk-plugin/types' {
  interface EventRegistry {
    // ── slots ──
    'slots:define': SlotsDefineEvent;
    'slots:place': SlotsPlaceEvent;
    'slots:remove': SlotsRemoveEvent;
    'slots:remove-all': SlotsRemoveAllEvent;
    'slots:changed': SlotsChangedEvent;
  }
}
