import { HttpPluginRegistry } from '@adi-family/sdk-plugin';

const GLOBAL_KEY = '__registry_hub__';
const STORAGE_KEY = 'adi:registry-urls';

export interface RegistryHub {
  readonly registries: ReadonlyMap<string, HttpPluginRegistry>;
  addRegistry(url: string): HttpPluginRegistry;
  removeRegistry(url: string): void;
  getRegistry(url: string): HttpPluginRegistry | undefined;
}

const loadUrls = (): string[] => {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (raw) {
      const parsed = JSON.parse(raw);
      if (Array.isArray(parsed) && parsed.length > 0) return parsed;
    }
  } catch { /* ignore */ }

  const multi = import.meta.env.VITE_REGISTRY_URLS as string | undefined;
  if (multi) return multi.split(',').map(s => s.trim()).filter(Boolean);

  const single = import.meta.env.VITE_REGISTRY_URL as string | undefined;
  return [single ?? 'http://adi.test/registry'];
};

const saveUrls = (urls: string[]): void => {
  localStorage.setItem(STORAGE_KEY, JSON.stringify(urls));
};

export const createRegistryHub = (): RegistryHub => {
  const registries = new Map<string, HttpPluginRegistry>();

  const addRegistry = (url: string): HttpPluginRegistry => {
    const existing = registries.get(url);
    if (existing) return existing;

    const registry = new HttpPluginRegistry(url);
    registries.set(url, registry);
    saveUrls([...registries.keys()]);
    return registry;
  };

  const removeRegistry = (url: string): void => {
    if (!registries.has(url)) return;
    registries.delete(url);
    saveUrls([...registries.keys()]);
  };

  const getRegistry = (url: string): HttpPluginRegistry | undefined => registries.get(url);

  for (const url of loadUrls()) {
    addRegistry(url);
  }

  return { registries, addRegistry, removeRegistry, getRegistry };
};

/** HMR-safe singleton for the registry hub. */
export const initRegistryHub = (): RegistryHub => {
  const existing = (globalThis as Record<string, unknown>)[GLOBAL_KEY] as RegistryHub | undefined;
  if (existing) return existing;

  const hub = createRegistryHub();
  (globalThis as Record<string, unknown>)[GLOBAL_KEY] = hub;
  return hub;
};
