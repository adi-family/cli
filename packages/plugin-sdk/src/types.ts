export interface EventRegistry {}

export type ReplyableEvent = {
  [K in keyof EventRegistry & string]: `${K}:ok` extends keyof EventRegistry
    ? K
    : never;
}[keyof EventRegistry & string];

export type EventHandler<K extends keyof EventRegistry> = (
  payload: EventRegistry[K]
) => void;

export type WithCid<T> = T & { _cid: string };

export interface SendHandle<T> {
  wait(): Promise<T>;
  handle(cb: (reply: T) => void): () => void;
}

export interface EventMeta {
  producer: string;
  consumers: string[];
}

export interface BusMiddleware {
  before?(event: string, payload: unknown, meta: EventMeta): void;
  after?(event: string, payload: unknown, meta: EventMeta): void;
  ignored?(event: string, payload: unknown, meta: EventMeta): void;
}

export interface EventBus {
  emit<K extends keyof EventRegistry>(event: K, payload: EventRegistry[K], producer: string): void;

  on<K extends keyof EventRegistry>(
    event: K,
    handler: EventHandler<K>,
    consumer: string,
  ): () => void;

  once<K extends keyof EventRegistry>(
    event: K,
    handler: EventHandler<K>,
    consumer: string,
  ): () => void;

  send<K extends ReplyableEvent>(
    event: K,
    payload: EventRegistry[K],
    producer: string,
  ): SendHandle<EventRegistry[`${K}:ok`]>;

  use(middleware: BusMiddleware): () => void;
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
