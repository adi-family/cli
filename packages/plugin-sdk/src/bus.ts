// src/bus.ts
import type {
  EventRegistry,
  EventBus,
  EventHandler,
  ReplyableEvent,
  SendHandle,
  WithCid,
} from './types.js';

interface ChannelState {
  handlers: Set<EventHandler<never>>;
  queue: unknown[];
}

export function createEventBus(options: { sendTimeout?: number } = {}): EventBus {
  const sendTimeoutMs = options.sendTimeout ?? 30_000;
  const channels = new Map<string, ChannelState>();

  function getChannel(event: string): ChannelState {
    if (!channels.has(event)) {
      channels.set(event, { handlers: new Set(), queue: [] });
    }
    return channels.get(event)!;
  }

  function emit<K extends keyof EventRegistry>(
    event: K,
    payload: EventRegistry[K]
  ): void {
    const ch = getChannel(event as string);
    if (ch.handlers.size === 0) {
      ch.queue.push(payload);
    } else {
      for (const h of ch.handlers) {
        (h as EventHandler<K>)(payload);
      }
    }
  }

  function on<K extends keyof EventRegistry>(
    event: K,
    handler: EventHandler<K>
  ): () => void {
    const ch = getChannel(event as string);
    ch.handlers.add(handler as EventHandler<never>);
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
    handler: EventHandler<K>
  ): () => void {
    let fired = false;
    let unsub: (() => void) | undefined;
    const wrapper: EventHandler<K> = (payload) => {
      if (fired) return;
      fired = true;
      handler(payload);
      unsub?.();
    };
    unsub = on(event, wrapper);
    // If the queue was flushed synchronously, wrapper already fired but
    // couldn't unsub (unsub was undefined at that moment). Clean up now.
    if (fired) {
      unsub(); // cleanup if queue was flushed synchronously
      return () => {}; // already fired, return no-op
    }
    return unsub;
  }

  function send<K extends ReplyableEvent>(
    event: K,
    payload: EventRegistry[K]
  ): SendHandle<EventRegistry[`${K}:ok`]> {
    const cid = crypto.randomUUID();
    const payloadWithCid = { ...(payload as object), _cid: cid } as unknown as EventRegistry[K];
    const replyEvent = `${event as string}:ok` as `${K}:ok`;

    // Emit immediately — handler starts working before .wait()/.handle() is called.
    // FIFO queue buffers any :ok reply that arrives before the caller subscribes.
    emit(event, payloadWithCid);

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
          });
          unsubRef.fn = unsub;
        });
      },
      handle(cb: (reply: EventRegistry[`${K}:ok`]) => void): () => void {
        // Same ref-cell pattern to avoid TDZ when FIFO flushes synchronously.
        const unsubRef: { fn?: () => void } = {};
        const unsub = on(replyEvent as keyof EventRegistry, (reply) => {
          const typed = reply as WithCid<EventRegistry[`${K}:ok`]>;
          if (typed._cid === cid) {
            unsubRef.fn?.();
            cb(typed);
          }
        });
        unsubRef.fn = unsub;
        return unsub;
      },
    };
  }

  return { emit, on, once, send };
}
