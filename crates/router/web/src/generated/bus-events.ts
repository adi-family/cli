/**
 * Auto-generated eventbus registry from TypeSpec.
 * DO NOT EDIT.
 */

import type { AdiRouterChangedEvent, AdiRouterNavigateEvent, AdiRouterRegisterRouteEvent } from './bus-types';

declare module '@adi-family/sdk-plugin/types' {
  interface EventRegistry {
    // ── adi.router ──
    'adi.router:navigate': AdiRouterNavigateEvent;
    'adi.router:changed': AdiRouterChangedEvent;
    'adi.router:register-route': AdiRouterRegisterRouteEvent;
  }
}
