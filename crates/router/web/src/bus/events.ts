/**
 * Auto-generated eventbus registry from TypeSpec.
 * DO NOT EDIT.
 */

import type { AdiRouterChangedEvent, AdiRouterNavigateEvent, AdiRouterRegisterRouteEvent } from './types';
import { AdiRouterBusKey } from './types';

declare module '@adi-family/sdk-plugin/types' {
  interface EventRegistry {
    // ── adi.router ──
    [AdiRouterBusKey.Navigate]: AdiRouterNavigateEvent;
    [AdiRouterBusKey.Changed]: AdiRouterChangedEvent;
    [AdiRouterBusKey.RegisterRoute]: AdiRouterRegisterRouteEvent;
  }
}
