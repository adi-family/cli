import type {
  EventRegistry,
  EventHandler,
  EventMeta,
  BusMiddleware,
  ReplyableEvent,
  SendHandle,
  WithCid,
} from './types.js';
import { generateCid } from './cid.js';
import { Logger } from './logger.js';

interface ChannelState {
  handlers: Map<EventHandler<never>, string>; // handler → consumer name
  queue: unknown[];
}

export interface EventBusOptions {
  sendTimeout?: number;
}

export class EventBus {
  private readonly log: Logger = new Logger(`event-bus`);
  private readonly sendTimeoutMs: number;
  private readonly channels = new Map<string, ChannelState>();
  private readonly middlewares = new Set<BusMiddleware>();

  private constructor(options: EventBusOptions = {}) {
    this.sendTimeoutMs = options.sendTimeout ?? 30_000;
  }

  static init(options: EventBusOptions = {}): EventBus {
    return new EventBus(options);
  }

  private getChannel(event: string): ChannelState {
    if (!this.channels.has(event)) {
      this.channels.set(event, { handlers: new Map(), queue: [] });
    }
    return this.channels.get(event)!;
  }

  emit<K extends keyof EventRegistry>(
    event: K,
    payload: EventRegistry[K],
    producer: string,
  ): void {
    const name = event as string;
    const ch = this.getChannel(name);
    const consumers = [...ch.handlers.values()];
    const meta: EventMeta = { producer, consumers };

    for (const mw of this.middlewares) mw.before?.(name, payload, meta);

    if (ch.handlers.size === 0) {
      ch.queue.push(payload);
      this.log.debug({ event: name, producer, msg: 'queued (no handlers)' });
      for (const mw of this.middlewares) mw.ignored?.(name, payload, meta);
    } else {
      this.log.trace({ event: name, producer, consumers });
      for (const [h] of ch.handlers) {
        (h as EventHandler<K>)(payload);
      }
      for (const mw of this.middlewares) mw.after?.(name, payload, meta);
    }
  }

  on<K extends keyof EventRegistry>(
    event: K,
    handler: EventHandler<K>,
    consumer: string,
  ): () => void {
    const ch = this.getChannel(event as string);
    ch.handlers.set(handler as EventHandler<never>, consumer);
    this.log.debug({ event, consumer, msg: 'subscribed' });
    if (ch.queue.length > 0) {
      const flushed = ch.queue.splice(0);
      this.log.debug({ event, consumer, count: flushed.length, msg: 'flushing queue' });
      for (const payload of flushed) {
        handler(payload as EventRegistry[K]);
      }
    }
    return () => {
      ch.handlers.delete(handler as EventHandler<never>);
    };
  }

  once<K extends keyof EventRegistry>(
    event: K,
    handler: EventHandler<K>,
    consumer: string,
  ): () => void {
    let fired = false;
    let unsub: (() => void) | undefined;
    const wrapper: EventHandler<K> = (payload) => {
      if (fired) return;
      fired = true;
      handler(payload);
      unsub?.();
    };
    unsub = this.on(event, wrapper, consumer);
    // If the queue was flushed synchronously, wrapper already fired but
    // couldn't unsub (unsub was undefined at that moment). Clean up now.
    if (fired) {
      unsub();
      return () => {};
    }
    return unsub;
  }

  send<K extends ReplyableEvent>(
    event: K,
    payload: EventRegistry[K],
    producer: string,
  ): SendHandle<EventRegistry[`${K}:ok`]> {
    const cid = generateCid();
    const payloadWithCid = {
      ...(payload as object),
      _cid: cid,
    } as unknown as EventRegistry[K];
    const replyEvent = `${event as string}:ok` as `${K}:ok`;

    this.log.debug({ event, cid, producer, msg: 'send' });
    this.emit(event, payloadWithCid, producer);

    return {
      wait: (): Promise<EventRegistry[`${K}:ok`]> => {
        return new Promise((resolve, reject) => {
          const unsubRef: { fn?: () => void } = {};
          const timer = setTimeout(() => {
            unsubRef.fn?.();
            this.log.warn({ event, cid, producer, msg: 'send timed out' });
            reject(
              new Error(
                `send('${event as string}') timed out after ${this.sendTimeoutMs}ms`,
              ),
            );
          }, this.sendTimeoutMs);
          const unsub = this.on(
            replyEvent as keyof EventRegistry,
            (reply) => {
              const typed = reply as WithCid<EventRegistry[`${K}:ok`]>;
              if (typed._cid === cid) {
                clearTimeout(timer);
                unsubRef.fn?.();
                resolve(typed);
              }
            },
            `${producer}:reply`,
          );
          unsubRef.fn = unsub;
        });
      },
      handle: (cb: (reply: EventRegistry[`${K}:ok`]) => void): (() => void) => {
        let fired = false;
        const unsubRef: { fn?: () => void } = {};
        const unsub = this.on(
          replyEvent as keyof EventRegistry,
          (reply) => {
            const typed = reply as WithCid<EventRegistry[`${K}:ok`]>;
            if (typed._cid === cid) {
              if (fired) return;
              fired = true;
              unsubRef.fn?.();
              cb(typed);
            }
          },
          `${producer}:reply`,
        );
        unsubRef.fn = unsub;
        if (fired) {
          unsub();
          return () => {};
        }
        return unsub;
      },
    };
  }

  use(middleware: BusMiddleware): () => void {
    this.middlewares.add(middleware);
    return () => {
      this.middlewares.delete(middleware);
    };
  }
}
