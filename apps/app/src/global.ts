import type { EventBus } from '@adi-family/sdk-plugin';
import type { SignalingHub } from './services/signaling/index.ts';
import type { RegistryHub } from './services/registry/index.ts';

export interface AdiGlobal {
  debug: { loaded: string[]; failed: string[]; timedOut: string[] };
  registryHub: RegistryHub;
  bus: EventBus;
  allPlugins: Array<{ id: string; installedVersion: string; pluginTypes?: string[] }>;
  signalingHub: SignalingHub;
  authAnonymous: (signalingUrl: string, authDomain: string) => void;
}

const KEY = '__adi';

/** Read the shared global object (or a specific field). */
export const getGlobal = <K extends keyof AdiGlobal>(key: K): AdiGlobal[K] | undefined =>
  ((window as unknown as Record<string, unknown>)[KEY] as Partial<AdiGlobal> | undefined)?.[key];

/** Set one or more fields on the shared global object. */
export const setGlobal = (patch: Partial<AdiGlobal>): void => {
  const w = window as unknown as Record<string, unknown>;
  const obj = (w[KEY] ?? {}) as Partial<AdiGlobal>;
  Object.assign(obj, patch);
  w[KEY] = obj;
};
