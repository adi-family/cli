export type {
  WsState,
  RtcState,
  SignalingMessage,
  CocoonInfo,
  HiveInfo,
  ServiceInfo,
  Capability,
  PtyMessage,
  SilkMessage,
  FileSystemMessage,
  AdiRequest,
  AdiResponse,
  AdiDiscovery,
  AdiServiceInfo,
  AdiMethodInfo,
  DataChannelName,
} from './types.ts';
export { AdiError, AdiTimeoutError, AdiServiceNotFoundError } from './adi-channel.ts';
export type { Connection } from './connection.ts';
export { createSignalingManager, type SignalingManager } from './manager.ts';

import type { EventBus } from '@adi-family/sdk-plugin';
import type { Connection } from './connection.ts';
import { createSignalingManager, type SignalingManager } from './manager.ts';

const GLOBAL_KEY = '__signaling_hub__';
const STORAGE_KEY = 'adi:signaling-urls';

export interface SignalingHub {
  readonly managers: ReadonlyMap<string, SignalingManager>;
  addServer(url: string): SignalingManager;
  removeServer(url: string): void;
  getManager(url: string): SignalingManager | undefined;
}

const loadUrls = (): string[] => {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (raw) {
      const parsed = JSON.parse(raw);
      if (Array.isArray(parsed) && parsed.length > 0) return parsed;
    }
  } catch { /* ignore */ }

  const env = import.meta.env.VITE_SIGNALING_URL as string | undefined;
  return [env ?? 'ws://adi.test/api/signaling/ws'];
};

const saveUrls = (urls: string[]): void => {
  localStorage.setItem(STORAGE_KEY, JSON.stringify(urls));
};

export const createSignalingHub = (
  connections: Map<string, Connection>,
  bus: EventBus,
  getToken: (authDomain: string, sourceUrl?: string) => Promise<string | null>,
): SignalingHub => {
  const managers = new Map<string, SignalingManager>();

  const addServer = (url: string): SignalingManager => {
    const existing = managers.get(url);
    if (existing) return existing;

    const manager = createSignalingManager(url, connections, bus, getToken);
    managers.set(url, manager);
    saveUrls([...managers.keys()]);

    manager.connect();
    return manager;
  };

  const removeServer = (url: string): void => {
    const manager = managers.get(url);
    if (!manager) return;

    manager.disconnect();
    managers.delete(url);
    saveUrls([...managers.keys()]);
  };

  const getManager = (url: string): SignalingManager | undefined => managers.get(url);

  // Auto-connect saved servers
  for (const url of loadUrls()) {
    addServer(url);
  }

  return { managers, addServer, removeServer, getManager };
};

/**
 * Initialize signaling hub: loads URLs from localStorage / env / default, creates managers, auto-connects.
 * HMR-safe via globalThis singleton.
 */
export const initSignalingHub = (
  connections: Map<string, Connection>,
  bus: EventBus,
  getToken: (authDomain: string, sourceUrl?: string) => Promise<string | null>,
): SignalingHub => {
  const existing = (globalThis as Record<string, unknown>)[GLOBAL_KEY] as SignalingHub | undefined;
  if (existing) return existing;

  const hub = createSignalingHub(connections, bus, getToken);
  (globalThis as Record<string, unknown>)[GLOBAL_KEY] = hub;

  // Register renderer for auth-required actions with anonymous option
  (globalThis as Record<string, unknown>)['__adiAuthAnonymous'] = (signalingUrl: string, authDomain: string) => {
    bus.emit('signaling:auth-anonymous', { signalingUrl, authDomain }, 'signaling');
  };

  bus.emit('actions:register-renderer', {
    plugin: 'adi.auth',
    kind: 'auth-required',
    render: (data: Record<string, unknown>) => {
      const options = data.authOptions as string[] | undefined;
      const signalingUrl = data.url as string;
      const authDomain = data.authDomain as string;
      const escaped = (s: string) => s.replace(/'/g, "\\'");

      const anonBtn = options?.includes('anonymous')
        ? `<button
             type="button"
             class="mt-2 px-3 py-1.5 text-xs font-medium rounded bg-brand text-white hover:bg-brand/80 transition-colors"
             onclick="globalThis.__adiAuthAnonymous('${escaped(signalingUrl)}', '${escaped(authDomain)}')"
           >Continue as Guest</button>`
        : '';

      return `<div class="text-xs">
        <div class="font-medium text-text">Authentication Required</div>
        <div class="text-text-muted mt-1">${escaped(signalingUrl)}</div>
        ${anonBtn}
      </div>`;
    },
  }, 'signaling');

  return hub;
};
