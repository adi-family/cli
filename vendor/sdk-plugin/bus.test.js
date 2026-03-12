// src/bus.test.ts
import { describe, it, expect, mock, jest } from 'bun:test';
import { EventBus } from './bus.js';
describe('EventBus — emit + on', () => {
    it('calls handler when event emitted', () => {
        const bus = EventBus.init();
        const handler = mock();
        bus.on('test:ping', handler, 'test');
        bus.emit('test:ping', { value: 42 }, 'test');
        expect(handler).toHaveBeenCalledWith({ value: 42 });
    });
    it('calls multiple handlers', () => {
        const bus = EventBus.init();
        const h1 = mock();
        const h2 = mock();
        bus.on('test:ping', h1, 'test-a');
        bus.on('test:ping', h2, 'test-b');
        bus.emit('test:ping', { value: 1 }, 'test');
        expect(h1).toHaveBeenCalledTimes(1);
        expect(h2).toHaveBeenCalledTimes(1);
    });
    it('unsubscribe stops handler', () => {
        const bus = EventBus.init();
        const handler = mock();
        const unsub = bus.on('test:ping', handler, 'test');
        unsub();
        bus.emit('test:ping', { value: 99 }, 'test');
        expect(handler).not.toHaveBeenCalled();
    });
});
describe('EventBus — FIFO queue', () => {
    it('queues events emitted before any subscriber', () => {
        const bus = EventBus.init();
        bus.emit('test:ping', { value: 1 }, 'test');
        bus.emit('test:ping', { value: 2 }, 'test');
        const received = [];
        bus.on('test:ping', ({ value }) => received.push(value), 'test');
        expect(received).toEqual([1, 2]);
    });
    it('replays queued events to every new subscriber', () => {
        const bus = EventBus.init();
        bus.emit('test:ping', { value: 10 }, 'test');
        const first = [];
        const second = [];
        bus.on('test:ping', ({ value }) => first.push(value), 'test-a');
        bus.on('test:ping', ({ value }) => second.push(value), 'test-b');
        expect(first).toEqual([10]);
        expect(second).toEqual([10]);
        bus.emit('test:ping', { value: 20 }, 'test');
        expect(first).toEqual([10, 20]);
        expect(second).toEqual([10, 20]);
    });
});
describe('EventBus — queue TTL', () => {
    it('drops queued events older than 30s on flush', () => {
        jest.useFakeTimers();
        const bus = EventBus.init();
        bus.emit('test:ping', { value: 1 }, 'test');
        jest.advanceTimersByTime(31_000);
        const handler = mock();
        bus.on('test:ping', handler, 'test');
        expect(handler).not.toHaveBeenCalled();
        jest.useRealTimers();
    });
    it('keeps queued events within 30s TTL', () => {
        jest.useFakeTimers();
        const bus = EventBus.init();
        bus.emit('test:ping', { value: 1 }, 'test');
        jest.advanceTimersByTime(29_000);
        const handler = mock();
        bus.on('test:ping', handler, 'test');
        expect(handler).toHaveBeenCalledWith({ value: 1 });
        jest.useRealTimers();
    });
});
describe('EventBus — once', () => {
    it('handler called only once', () => {
        const bus = EventBus.init();
        const handler = mock();
        bus.once('test:ping', handler, 'test');
        bus.emit('test:ping', { value: 1 }, 'test');
        bus.emit('test:ping', { value: 2 }, 'test');
        expect(handler).toHaveBeenCalledTimes(1);
        expect(handler).toHaveBeenCalledWith({ value: 1 });
    });
    it('once flushes queued event immediately on subscribe', () => {
        const bus = EventBus.init();
        bus.emit('test:ping', { value: 5 }, 'test');
        const handler = mock();
        bus.once('test:ping', handler, 'test');
        expect(handler).toHaveBeenCalledWith({ value: 5 });
    });
    it('once fires at most once even when queue is flushed then emitted again', () => {
        const bus = EventBus.init();
        bus.emit('test:ping', { value: 1 }, 'test'); // queue before subscriber
        const handler = mock();
        bus.once('test:ping', handler, 'test'); // flushes queue synchronously
        bus.emit('test:ping', { value: 2 }, 'test'); // should NOT call handler again
        expect(handler).toHaveBeenCalledTimes(1);
        expect(handler).toHaveBeenCalledWith({ value: 1 });
    });
    it('once returns a no-op unsubscribe after synchronous flush', () => {
        const bus = EventBus.init();
        bus.emit('test:ping', { value: 10 }, 'test'); // pre-queue event
        const handler = mock();
        const unsub = bus.once('test:ping', handler, 'test'); // flushes synchronously
        // Calling unsub on an already-fired once() must not throw.
        expect(() => unsub()).not.toThrow();
        // A second once() registration fires immediately from queue replay.
        const handler2 = mock();
        bus.once('test:ping', handler2, 'test');
        expect(handler2).toHaveBeenCalledTimes(1);
        expect(handler2).toHaveBeenCalledWith({ value: 10 });
    });
});
describe('EventBus — middleware meta', () => {
    it('before/after receive producer and consumers', () => {
        const bus = EventBus.init();
        const metas = [];
        bus.use({
            before: (_e, _p, meta) => metas.push({ phase: 'before', ...meta }),
            after: (_e, _p, meta) => metas.push({ phase: 'after', ...meta }),
        });
        bus.on('test:ping', () => { }, 'widget-a');
        bus.on('test:ping', () => { }, 'widget-b');
        bus.emit('test:ping', { value: 1 }, 'source');
        expect(metas).toEqual([
            { phase: 'before', producer: 'source', consumers: ['widget-a', 'widget-b'] },
            { phase: 'after', producer: 'source', consumers: ['widget-a', 'widget-b'] },
        ]);
    });
    it('ignored fires instead of after when no handlers', () => {
        const bus = EventBus.init();
        const phases = [];
        bus.use({
            after: () => phases.push('after'),
            ignored: () => phases.push('ignored'),
        });
        bus.emit('test:ping', { value: 1 }, 'source');
        expect(phases).toEqual(['ignored']);
    });
});
