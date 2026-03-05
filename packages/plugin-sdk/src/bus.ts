import type {
  EventRegistry,
  EventHandler,
  EventMeta,
  BusMiddleware,
} from './types.js';
import { Logger } from './logger.js';

interface QueuedEvent {
  payload: unknown;
  timestamp: number;
}

const QUEUE_TTL_MS = 30_000;

interface ChannelState {
  handlers: Map<EventHandler<never>, string>; // handler → consumer name
  queue: QueuedEvent[];
}

export class EventBus {
  private readonly log: Logger = new Logger(`event-bus`);
  private readonly channels = new Map<string, ChannelState>();
  private readonly middlewares = new Set<BusMiddleware>();

  private constructor() {}

  static init(): EventBus {
    return new EventBus();
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
      ch.queue.push({ payload, timestamp: Date.now() });
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
    this.log.trace({ event, consumer, msg: 'subscribed' });
    if (ch.queue.length > 0) {
      const now = Date.now();
      const flushed = ch.queue.splice(0).filter(e => now - e.timestamp < QUEUE_TTL_MS);
      this.log.debug({ event, consumer, count: flushed.length, msg: 'flushing queue' });
      for (const { payload } of flushed) {
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

  use(middleware: BusMiddleware): () => void {
    this.middlewares.add(middleware);
    return () => {
      this.middlewares.delete(middleware);
    };
  }
}
