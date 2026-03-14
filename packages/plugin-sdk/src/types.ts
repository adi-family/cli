export interface EventRegistry {}

export interface PluginApiRegistry {}

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


export interface PluginBundleInfo {
  jsUrl: string;
  cssUrl?: string;
}

export interface PluginRegistry {
  readonly url: string;

  getBundleInfo(id: string, version: string): Promise<PluginBundleInfo>;

  checkLatest(
    id: string,
    currentVersion: string
  ): Promise<{ version: string } | null>;
}

export interface PluginDescriptor {
  id: string;
  name?: string;
  description?: string;
  author?: string;
  tags?: string[];
  downloads?: number;
  registry: PluginRegistry;
  installedVersion: string;
  latestVersion?: string;
}
