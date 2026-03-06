/** Simple key-value storage interface for plugins. */
export interface PluginStorage {
  get<T = unknown>(key: string): Promise<T | undefined>;
  set<T = unknown>(key: string, value: T): Promise<void>;
  delete(key: string): Promise<void>;
  keys(): Promise<string[]>;
}

/** Factory provided by the app to create per-plugin storage instances. */
export type StorageFactory = (pluginId: string) => PluginStorage;
