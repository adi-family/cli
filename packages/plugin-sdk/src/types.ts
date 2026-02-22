/**
 * Central event registry — augmented by plugins via declaration merging.
 *
 * Plugin authors extend this in their own package:
 *   declare module '@adi-family/sdk-plugin' {
 *     interface EventRegistry {
 *       'my-event': { data: string };
 *       'my-event:ok': { success: boolean; _cid: string };
 *     }
 *   }
 */
export interface EventRegistry {}

/**
 * Extracts all keys K where `${K}:ok` is also in EventRegistry.
 * Constrains bus.send() to events that have a declared reply type.
 */
export type ReplyableEvent = {
  [K in keyof EventRegistry & string]: `${K}:ok` extends keyof EventRegistry
    ? K
    : never;
}[keyof EventRegistry & string];

/** Handler function type for a given event key. */
export type EventHandler<K extends keyof EventRegistry> = (
  payload: EventRegistry[K]
) => void;

/** Correlation ID injected by send() into outgoing payloads. */
export type WithCid<T> = T & { _cid: string };

/** Handle returned by bus.send() — call .wait() or .handle() to receive the reply. */
export interface SendHandle<T> {
  /** Await the correlated :ok reply. Rejects on timeout. */
  wait(): Promise<T>;
  /** Register a one-shot callback for the correlated :ok reply. No timeout. */
  handle(cb: (reply: T) => void): () => void;
}

/** The strictly-typed event bus. */
export interface EventBus {
  /** Broadcast to all subscribers. Queued FIFO if no subscribers yet. */
  emit<K extends keyof EventRegistry>(event: K, payload: EventRegistry[K]): void;

  /** Subscribe. Returns unsubscribe fn. Flushes FIFO queue on first subscribe. */
  on<K extends keyof EventRegistry>(
    event: K,
    handler: EventHandler<K>
  ): () => void;

  /** Subscribe once — auto-removed after first delivery. */
  once<K extends keyof EventRegistry>(
    event: K,
    handler: EventHandler<K>
  ): () => void;

  /**
   * Emit and await correlated reply (`${event}:ok`).
   * SDK injects `_cid`; reply must echo the same `_cid`.
   */
  send<K extends ReplyableEvent>(
    event: K,
    payload: EventRegistry[K]
  ): SendHandle<EventRegistry[`${K}:ok`]>;
}

/**
 * Abstracts where plugins come from. Any backend implements this.
 * SDK ships CocoonPluginRegistry as the built-in implementation.
 */
export interface PluginRegistry {
  /** Returns the URL to fetch the JS bundle for a specific installed version. */
  fetchBundle(id: string, version: string): Promise<string>;

  /**
   * Checks if a newer version is available.
   * Returns { version } if an update exists, null if already up to date.
   */
  checkLatest(
    id: string,
    currentVersion: string
  ): Promise<{ version: string } | null>;
}

/** Describes a plugin to load — id, where it lives, and which version is installed. */
export interface PluginDescriptor {
  id: string;
  registry: PluginRegistry;
  installedVersion: string;
}
