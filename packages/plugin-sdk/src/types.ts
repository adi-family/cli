export interface EventRegistry {}

export type EventHandler<K extends keyof EventRegistry> = (
  payload: EventRegistry[K]
) => void;

export interface EventMeta {
  producer: string;
  consumers: string[];
}

export interface BusMiddleware {
  before?(event: string, payload: unknown, meta: EventMeta): void;
  after?(event: string, payload: unknown, meta: EventMeta): void;
  ignored?(event: string, payload: unknown, meta: EventMeta): void;
}


export interface PluginRegistry {
  bundleUrl(id: string, version: string): Promise<string>;

  checkLatest(
    id: string,
    currentVersion: string
  ): Promise<{ version: string } | null>;
}

export interface PluginDescriptor {
  id: string;
  registry: PluginRegistry;
  installedVersion: string;
  latestVersion?: string;
  /** Plugin kinds reported by the registry (e.g. ["web"], ["http","web"], ["core"]). */
  pluginTypes?: string[];
}
