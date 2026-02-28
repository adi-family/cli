import type { EventBus } from '@adi-family/sdk-plugin';
import type { RegistryHub } from '../services/registry/index';

export interface PluginDebugInfo {
  loaded: string[];
  failed: string[];
  timedOut: string[];
}

export interface PluginEntry {
  id: string;
  installedVersion: string;
  pluginTypes?: string[];
}

export interface AdiGlobal {
  bus: EventBus;
  registryHub: RegistryHub;
  debug: PluginDebugInfo;
  allPlugins: PluginEntry[];
}

const KEY = '__adi';
const w = window as unknown as Record<string, unknown>;

export const getGlobal = (): AdiGlobal => {
  return w[KEY] as AdiGlobal;
}

export const setGlobal = (patch: Partial<AdiGlobal>): void => {
  const w = window as unknown as Record<string, unknown>;
  const obj = (w[KEY] ?? {}) as Partial<AdiGlobal>;
  Object.assign(obj, patch);
  w[KEY] = obj;
};
