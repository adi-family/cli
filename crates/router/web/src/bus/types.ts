/**
 * Auto-generated eventbus types from TypeSpec.
 * DO NOT EDIT.
 */

export interface AdiRouterNavigateEvent {
  path: string;
  replace?: boolean;
}

export interface AdiRouterChangedEvent {
  path: string;
  params: Record<string, string>;
}

export interface AdiRouterRegisterRouteEvent {
  pluginId: string;
  path: string;
  init: unknown;
  label?: string;
}

export enum AdiRouterBusKey {
  Navigate = 'adi.router:navigate',
  Changed = 'adi.router:changed',
  RegisterRoute = 'adi.router:register-route',
}
