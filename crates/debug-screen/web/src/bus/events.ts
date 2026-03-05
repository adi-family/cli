/**
 * Auto-generated eventbus registry from TypeSpec.
 * DO NOT EDIT.
 */

import type { AdiDebugScreenRegisterSectionEvent } from './types';
import { AdiDebugScreenBusKey } from './types';

declare module '@adi-family/sdk-plugin/types' {
  interface EventRegistry {
    // ── adi.debug-screen ──
    [AdiDebugScreenBusKey.RegisterSection]: AdiDebugScreenRegisterSectionEvent;
  }
}
