import { Logger } from './logger.js';
const QUEUE_TTL_MS = 30_000;
export class EventBus {
    log = new Logger(`event-bus`);
    channels = new Map();
    middlewares = new Set();
    constructor() { }
    static init() {
        return new EventBus();
    }
    getChannel(event) {
        if (!this.channels.has(event)) {
            this.channels.set(event, { handlers: new Map(), queue: [] });
        }
        return this.channels.get(event);
    }
    emit(event, payload, producer) {
        const name = event;
        const ch = this.getChannel(name);
        const consumers = [...ch.handlers.values()];
        const meta = { producer, consumers };
        for (const mw of this.middlewares)
            mw.before?.(name, payload, meta);
        if (ch.handlers.size === 0) {
            ch.queue.push({ payload, timestamp: Date.now() });
            this.log.debug({ event: name, producer, msg: 'queued (no handlers)' });
            for (const mw of this.middlewares)
                mw.ignored?.(name, payload, meta);
        }
        else {
            this.log.trace({ event: name, producer, consumers });
            for (const [h] of ch.handlers) {
                h(payload);
            }
            for (const mw of this.middlewares)
                mw.after?.(name, payload, meta);
        }
    }
    on(event, handler, consumer) {
        const ch = this.getChannel(event);
        ch.handlers.set(handler, consumer);
        this.log.trace({ event, consumer, msg: 'subscribed' });
        if (ch.queue.length > 0) {
            const now = Date.now();
            ch.queue = ch.queue.filter(e => now - e.timestamp < QUEUE_TTL_MS);
            this.log.debug({ event, consumer, count: ch.queue.length, msg: 'replaying queue' });
            for (const { payload } of ch.queue) {
                handler(payload);
            }
        }
        return () => {
            ch.handlers.delete(handler);
        };
    }
    once(event, handler, consumer) {
        let fired = false;
        let unsub;
        const wrapper = (payload) => {
            if (fired)
                return;
            fired = true;
            handler(payload);
            unsub?.();
        };
        unsub = this.on(event, wrapper, consumer);
        // If the queue was flushed synchronously, wrapper already fired but
        // couldn't unsub (unsub was undefined at that moment). Clean up now.
        if (fired) {
            unsub();
            return () => { };
        }
        return unsub;
    }
    use(middleware) {
        this.middlewares.add(middleware);
        return () => {
            this.middlewares.delete(middleware);
        };
    }
}
