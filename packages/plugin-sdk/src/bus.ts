import type {
  EventRegistry,
  EventBus,
  EventHandler,
  EventMeta,
  BusMiddleware,
  ReplyableEvent,
  SendHandle,
  WithCid,
} from './types.js';
import { generateCid } from './cid.js';

interface ChannelState {
  handlers: Map<EventHandler<never>, string>; // handler → consumer name
  queue: unknown[];
}

export function createEventBus(options: { sendTimeout?: number } = {}): EventBus {
  const sendTimeoutMs = options.sendTimeout ?? 30_000;
  const channels = new Map<string, ChannelState>();
  const middlewares = new Set<BusMiddleware>();

  function getChannel(event: string): ChannelState {
    if (!channels.has(event)) {
      channels.set(event, { handlers: new Map(), queue: [] });
    }
    return channels.get(event)!;
  }

  function emit<K extends keyof EventRegistry>(
    event: K,
    payload: EventRegistry[K],
    producer: string,
  ): void {
    const name = event as string;
    const ch = getChannel(name);
    const consumers = [...ch.handlers.values()];
    const meta: EventMeta = { producer, consumers };

    for (const mw of middlewares) mw.before?.(name, payload, meta);

    if (ch.handlers.size === 0) {
      ch.queue.push(payload);
      for (const mw of middlewares) mw.ignored?.(name, payload, meta);
    } else {
      for (const [h] of ch.handlers) {
        (h as EventHandler<K>)(payload);
      }
      for (const mw of middlewares) mw.after?.(name, payload, meta);
    }
  }

  function on<K extends keyof EventRegistry>(
    event: K,
    handler: EventHandler<K>,
    consumer: string,
  ): () => void {
    const ch = getChannel(event as string);
    ch.handlers.set(handler as EventHandler<never>, consumer);
    if (ch.queue.length > 0) {
      const flushed = ch.queue.splice(0);
      for (const payload of flushed) {
        handler(payload as EventRegistry[K]);
      }
    }
    return () => {
      ch.handlers.delete(handler as EventHandler<never>);
    };
  }

  function once<K extends keyof EventRegistry>(
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
    unsub = on(event, wrapper, consumer);
    // If the queue was flushed synchronously, wrapper already fired but
    // couldn't unsub (unsub was undefined at that moment). Clean up now.
    if (fired) {
      unsub();
      return () => {};
    }
    return unsub;
  }

  function send<K extends ReplyableEvent>(
    event: K,
    payload: EventRegistry[K],
    producer: string,
  ): SendHandle<EventRegistry[`${K}:ok`]> {
    const cid = generateCid();
    const payloadWithCid = { ...(payload as object), _cid: cid } as unknown as EventRegistry[K];
    const replyEvent = `${event as string}:ok` as `${K}:ok`;

    // Emit immediately — handler starts working before .wait()/.handle() is called.
    // FIFO queue buffers any :ok reply that arrives before the caller subscribes.
    emit(event, payloadWithCid, producer);

    return {
      wait(): Promise<EventRegistry[`${K}:ok`]> {
        return new Promise((resolve, reject) => {
          // Use a ref cell to break the TDZ: the handler captures `unsubRef`
          // by reference so it can call unsub() even when the FIFO flush fires
          // synchronously inside `on()` before `unsub` is assigned.
          const unsubRef: { fn?: () => void } = {};
          const timer = setTimeout(() => {
            unsubRef.fn?.();
            reject(new Error(`send('${event as string}') timed out after ${sendTimeoutMs}ms`));
          }, sendTimeoutMs);
          const unsub = on(replyEvent as keyof EventRegistry, (reply) => {
            const typed = reply as WithCid<EventRegistry[`${K}:ok`]>;
            if (typed._cid === cid) {
              clearTimeout(timer);
              unsubRef.fn?.();
              resolve(typed);
            }
          }, `${producer}:reply`);
          unsubRef.fn = unsub;
        });
      },
      handle(cb: (reply: EventRegistry[`${K}:ok`]) => void): () => void {
        let fired = false;
        const unsubRef: { fn?: () => void } = {};
        const unsub = on(replyEvent as keyof EventRegistry, (reply) => {
          const typed = reply as WithCid<EventRegistry[`${K}:ok`]>;
          if (typed._cid === cid) {
            if (fired) return;
            fired = true;
            unsubRef.fn?.();
            cb(typed);
          }
        }, `${producer}:reply`);
        unsubRef.fn = unsub;
        if (fired) { unsub(); return () => {}; }
        return unsub;
      },
    };
  }

  function use(middleware: BusMiddleware): () => void {
    middlewares.add(middleware);
    return () => { middlewares.delete(middleware); };
  }

  return { emit, on, once, send, use };
}
