// src/bus.test.ts
import { describe, it, expect, vi } from 'vitest';
import { createEventBus } from './bus.js';

declare module './types.js' {
  interface EventRegistry {
    'test:ping': { value: number };
    'test:ping:ok': { echo: number; _cid: string };
  }
}

describe('EventBus — emit + on', () => {
  it('calls handler when event emitted', () => {
    const bus = createEventBus();
    const handler = vi.fn();
    bus.on('test:ping', handler);
    bus.emit('test:ping', { value: 42 });
    expect(handler).toHaveBeenCalledWith({ value: 42 });
  });

  it('calls multiple handlers', () => {
    const bus = createEventBus();
    const h1 = vi.fn();
    const h2 = vi.fn();
    bus.on('test:ping', h1);
    bus.on('test:ping', h2);
    bus.emit('test:ping', { value: 1 });
    expect(h1).toHaveBeenCalledTimes(1);
    expect(h2).toHaveBeenCalledTimes(1);
  });

  it('unsubscribe stops handler', () => {
    const bus = createEventBus();
    const handler = vi.fn();
    const unsub = bus.on('test:ping', handler);
    unsub();
    bus.emit('test:ping', { value: 99 });
    expect(handler).not.toHaveBeenCalled();
  });
});

describe('EventBus — FIFO queue', () => {
  it('queues events emitted before any subscriber', () => {
    const bus = createEventBus();
    bus.emit('test:ping', { value: 1 });
    bus.emit('test:ping', { value: 2 });
    const received: number[] = [];
    bus.on('test:ping', ({ value }) => received.push(value));
    expect(received).toEqual([1, 2]);
  });

  it('flushes queue only to first subscriber; later subscribers get fresh events only', () => {
    const bus = createEventBus();
    bus.emit('test:ping', { value: 10 });
    const first: number[] = [];
    const second: number[] = [];
    bus.on('test:ping', ({ value }) => first.push(value));
    bus.on('test:ping', ({ value }) => second.push(value));
    expect(first).toEqual([10]);
    expect(second).toEqual([]);
    bus.emit('test:ping', { value: 20 });
    expect(first).toEqual([10, 20]);
    expect(second).toEqual([20]);
  });
});

describe('EventBus — once', () => {
  it('handler called only once', () => {
    const bus = createEventBus();
    const handler = vi.fn();
    bus.once('test:ping', handler);
    bus.emit('test:ping', { value: 1 });
    bus.emit('test:ping', { value: 2 });
    expect(handler).toHaveBeenCalledTimes(1);
    expect(handler).toHaveBeenCalledWith({ value: 1 });
  });

  it('once flushes queued event immediately on subscribe', () => {
    const bus = createEventBus();
    bus.emit('test:ping', { value: 5 });
    const handler = vi.fn();
    bus.once('test:ping', handler);
    expect(handler).toHaveBeenCalledWith({ value: 5 });
  });

  it('once fires at most once even when queue is flushed then emitted again', () => {
    const bus = createEventBus();
    bus.emit('test:ping', { value: 1 });  // queue before subscriber
    const handler = vi.fn();
    bus.once('test:ping', handler);        // flushes queue synchronously
    bus.emit('test:ping', { value: 2 });  // should NOT call handler again
    expect(handler).toHaveBeenCalledTimes(1);
    expect(handler).toHaveBeenCalledWith({ value: 1 });
  });

  it('once returns a no-op unsubscribe after synchronous flush', () => {
    const bus = createEventBus();
    bus.emit('test:ping', { value: 10 }); // pre-queue event
    const handler = vi.fn();
    const unsub = bus.once('test:ping', handler); // flushes synchronously

    // Calling unsub on an already-fired once() must not throw.
    expect(() => unsub()).not.toThrow();

    // A second once() registration on the same event should work normally.
    const handler2 = vi.fn();
    bus.once('test:ping', handler2);
    bus.emit('test:ping', { value: 20 });
    expect(handler2).toHaveBeenCalledTimes(1);
    expect(handler2).toHaveBeenCalledWith({ value: 20 });
  });
});

describe('EventBus — send', () => {
  it('resolves when :ok emitted with matching _cid', async () => {
    const bus = createEventBus();
    bus.on('test:ping', (payload) => {
      const p = payload as { value: number; _cid: string };
      bus.emit('test:ping:ok', { echo: p.value, _cid: p._cid });
    });
    const result = await bus.send('test:ping', { value: 7 }).wait();
    expect(result.echo).toBe(7);
  });

  it('ignores :ok replies with wrong _cid', async () => {
    const bus = createEventBus();
    bus.on('test:ping', (payload) => {
      const p = payload as { value: number; _cid: string };
      bus.emit('test:ping:ok', { echo: 0, _cid: 'wrong' });
      bus.emit('test:ping:ok', { echo: p.value, _cid: p._cid });
    });
    const result = await bus.send('test:ping', { value: 3 }).wait();
    expect(result.echo).toBe(3);
  });

  it('send rejects with timeout error when no reply arrives', async () => {
    const bus = createEventBus({ sendTimeout: 50 });
    await expect(bus.send('test:ping', { value: 1 }).wait()).rejects.toThrow('timed out');
  });

  it('handle() calls callback when :ok arrives with matching _cid', async () => {
    const bus = createEventBus();
    bus.on('test:ping', (payload) => {
      const p = payload as { value: number; _cid: string };
      bus.emit('test:ping:ok', { echo: p.value, _cid: p._cid });
    });
    await new Promise<void>((resolve) => {
      bus.send('test:ping', { value: 42 }).handle(({ echo }) => {
        expect(echo).toBe(42);
        resolve();
      });
    });
  });

  it('handle() ignores :ok replies with wrong _cid', async () => {
    const bus = createEventBus();
    let callCount = 0;
    bus.on('test:ping', (payload) => {
      const p = payload as { value: number; _cid: string };
      bus.emit('test:ping:ok', { echo: 0, _cid: 'wrong' });
      bus.emit('test:ping:ok', { echo: p.value, _cid: p._cid });
    });
    await new Promise<void>((resolve) => {
      bus.send('test:ping', { value: 5 }).handle(({ echo }) => {
        callCount++;
        expect(echo).toBe(5);
        resolve();
      });
    });
    expect(callCount).toBe(1);
  });
});
