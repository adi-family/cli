/**
 * Auto-generated eventbus registry from TypeSpec.
 * DO NOT EDIT.
 */

import type { SlotsChangedEvent, SlotsDefineEvent, SlotsPlaceEvent, SlotsRemoveAllEvent, SlotsRemoveEvent } from './types';
import { SlotsBusKey } from './types';

declare module '@adi-family/sdk-plugin/types' {
  interface EventRegistry {
    // ── slots ──
    [SlotsBusKey.Define]: SlotsDefineEvent;
    [SlotsBusKey.Place]: SlotsPlaceEvent;
    [SlotsBusKey.Remove]: SlotsRemoveEvent;
    [SlotsBusKey.RemoveAll]: SlotsRemoveAllEvent;
    [SlotsBusKey.Changed]: SlotsChangedEvent;
  }
}
