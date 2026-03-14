import type { PluginStorage } from '@adi-family/sdk-plugin';
import type { StoredSession } from './types.js';

const OLD_LS_KEY = 'adi.auth.session';
const LS_KEY_PREFIX = 'adi.auth.session:';
const SESSION_PREFIX = 'session:';

let storage: PluginStorage;

export const init = (s: PluginStorage): void => {
  storage = s;
};

export const load = async (authDomain: string): Promise<StoredSession | null> => {
  try {
    return (await storage.get<StoredSession>(SESSION_PREFIX + authDomain)) ?? null;
  } catch {
    return null;
  }
};

export const save = async (session: StoredSession): Promise<void> => {
  await storage.set(SESSION_PREFIX + session.authUrl, session);
};

export const clear = async (authDomain: string): Promise<void> => {
  await storage.delete(SESSION_PREFIX + authDomain);
};

export const clearAll = async (): Promise<void> => {
  const domains = await listDomains();
  for (const domain of domains) {
    await storage.delete(SESSION_PREFIX + domain);
  }
};

export const loadValid = async (authDomain: string): Promise<StoredSession | null> => {
  const session = await load(authDomain);
  if (!session) return null;
  return session.expiresAt > Date.now() ? session : null;
};

export const listDomains = async (): Promise<string[]> => {
  const keys = await storage.keys();
  return keys
    .filter((k) => k.startsWith(SESSION_PREFIX))
    .map((k) => k.slice(SESSION_PREFIX.length));
};

export const migrateFromLocalStorage = async (): Promise<void> => {
  try {
    const oldRaw = localStorage.getItem(OLD_LS_KEY);
    if (oldRaw) {
      const session = JSON.parse(oldRaw) as StoredSession;
      if (session.authUrl) await save(session);
      localStorage.removeItem(OLD_LS_KEY);
    }

    const keysToRemove: string[] = [];
    for (let i = 0; i < localStorage.length; i++) {
      const key = localStorage.key(i);
      if (key?.startsWith(LS_KEY_PREFIX)) {
        const raw = localStorage.getItem(key);
        if (raw) {
          const session = JSON.parse(raw) as StoredSession;
          await save(session);
        }
        keysToRemove.push(key);
      }
    }
    for (const key of keysToRemove) {
      localStorage.removeItem(key);
    }
  } catch {
    // Migration is best-effort
  }
};
