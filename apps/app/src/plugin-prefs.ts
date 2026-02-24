const KEY = 'adi:enabled-web-plugins';

/** Returns the set of user-enabled web plugin IDs, or null if never configured. */
export function getEnabledWebPluginIds(): Set<string> | null {
  const raw = localStorage.getItem(KEY);
  if (raw === null) return null;
  try {
    return new Set(JSON.parse(raw) as string[]);
  } catch {
    return null;
  }
}

/** Persists the enabled web plugin ID set to localStorage. */
export function setEnabledWebPluginIds(ids: Iterable<string>): void {
  localStorage.setItem(KEY, JSON.stringify([...ids]));
}
