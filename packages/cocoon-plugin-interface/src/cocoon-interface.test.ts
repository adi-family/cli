import { describe, it, expect, mock } from 'bun:test';
import { CocoonPluginInterface } from './cocoon-interface.js';
import { CocoonBusKey } from './bus-keys.js';
import { AdiSignalingBusKey } from '@adi-family/plugin-signaling/bus';
import type { Connection } from '@adi-family/plugin-signaling/bus';
import type { EventBus } from '@adi-family/sdk-plugin';

type Handler = (payload: any) => void;

function makeBus(): EventBus {
  const handlers = new Map<string, Handler[]>();
  return {
    on: mock((key: string, handler: Handler, _producer: string) => {
      const list = handlers.get(key) ?? [];
      list.push(handler);
      handlers.set(key, list);
      return () => {
        const idx = list.indexOf(handler);
        if (idx >= 0) list.splice(idx, 1);
      };
    }),
    emit: mock((key: string, payload: unknown, _producer: string) => {
      for (const h of handlers.get(key) ?? []) h(payload);
    }),
  } as unknown as EventBus;
}

function makeConnection(id: string, plugins: string[] = []): Connection {
  return {
    id,
    plugins,
    request: mock(),
    stream: mock(),
    httpProxy: mock(),
    httpDirect: mock(),
  } as unknown as Connection;
}

describe('CocoonPluginInterface — create + bus', () => {
  it('creates instance via static factory', () => {
    const iface = CocoonPluginInterface.create('test-plugin');
    expect(iface).toBeInstanceOf(CocoonPluginInterface);
  });

  it('throws when accessing bus before init', () => {
    const iface = CocoonPluginInterface.create('my-plugin');
    expect(() => iface.bus).toThrow('my-plugin: bus not initialized');
  });

  it('exposes bus after init', () => {
    const bus = makeBus();
    const iface = CocoonPluginInterface.create('p');
    iface.init(bus);
    expect(iface.bus).toBe(bus);
  });
});

describe('CocoonPluginInterface — connections', () => {
  it('tracks added connections', () => {
    const bus = makeBus();
    const iface = CocoonPluginInterface.create('p');
    iface.init(bus);

    const conn = makeConnection('c1', ['adi.svc-a']);
    bus.emit(
      CocoonBusKey.ConnectionAdded,
      { id: 'c1', connection: conn },
      'test',
    );

    expect(iface.getConnection('c1')).toBe(conn);
    expect(iface.allConnections()).toEqual([conn]);
  });

  it('throws when getting a missing connection', () => {
    const bus = makeBus();
    const iface = CocoonPluginInterface.create('p');
    iface.init(bus);

    expect(() => iface.getConnection('missing')).toThrow(
      "Connection 'missing' not found",
    );
  });

  it('removes connections on ConnectionRemoved event', () => {
    const bus = makeBus();
    const iface = CocoonPluginInterface.create('p');
    iface.init(bus);

    const conn = makeConnection('c1');
    bus.emit(
      CocoonBusKey.ConnectionAdded,
      { id: 'c1', connection: conn },
      'test',
    );
    bus.emit(CocoonBusKey.ConnectionRemoved, { id: 'c1' }, 'test');

    expect(iface.allConnections()).toEqual([]);
    expect(() => iface.getConnection('c1')).toThrow();
  });

  it('filters connections by plugin id', () => {
    const bus = makeBus();
    const iface = CocoonPluginInterface.create('p');
    iface.init(bus);

    const c1 = makeConnection('c1', ['adi.editor', 'adi.terminal']);
    const c2 = makeConnection('c2', ['adi.terminal']);
    const c3 = makeConnection('c3', ['adi.editor']);
    bus.emit(
      CocoonBusKey.ConnectionAdded,
      { id: 'c1', connection: c1 },
      'test',
    );
    bus.emit(
      CocoonBusKey.ConnectionAdded,
      { id: 'c2', connection: c2 },
      'test',
    );
    bus.emit(
      CocoonBusKey.ConnectionAdded,
      { id: 'c3', connection: c3 },
      'test',
    );

    expect(iface.connectionsWithPlugin('adi.terminal')).toEqual([c1, c2]);
    expect(iface.connectionsWithPlugin('adi.editor')).toEqual([c1, c3]);
    expect(iface.connectionsWithPlugin('unknown')).toEqual([]);
  });
});

describe('CocoonPluginInterface — devices', () => {
  it('tracks devices from signaling Devices event', () => {
    const bus = makeBus();
    const iface = CocoonPluginInterface.create('p');
    iface.init(bus);

    bus.emit(
      AdiSignalingBusKey.Devices,
      {
        url: 'wss://example.com',
        devices: [
          { device_id: 'd1', tags: {}, online: true, device_type: 'cocoon' },
          { device_id: 'd2', tags: {}, online: true, device_type: 'browser' },
        ],
      },
      'test',
    );

    expect(iface.allDevices()).toEqual([
      { device_id: 'd1', tags: {}, online: true, device_type: 'cocoon' },
      { device_id: 'd2', tags: {}, online: true, device_type: 'browser' },
    ]);
  });

  it('replaces devices on subsequent Devices event', () => {
    const bus = makeBus();
    const iface = CocoonPluginInterface.create('p');
    iface.init(bus);

    bus.emit(
      AdiSignalingBusKey.Devices,
      {
        url: 'wss://x',
        devices: [{ device_id: 'd1', tags: {}, online: true }],
      },
      'test',
    );
    bus.emit(
      AdiSignalingBusKey.Devices,
      {
        url: 'wss://x',
        devices: [{ device_id: 'd2', tags: {}, online: false }],
      },
      'test',
    );

    expect(iface.allDevices()).toEqual([
      { device_id: 'd2', tags: {}, online: false },
    ]);
  });

  it('filters cocoon devices', () => {
    const bus = makeBus();
    const iface = CocoonPluginInterface.create('p');
    iface.init(bus);

    bus.emit(
      AdiSignalingBusKey.Devices,
      {
        url: 'wss://x',
        devices: [
          { device_id: 'd1', tags: {}, online: true, device_type: 'cocoon' },
          { device_id: 'd2', tags: {}, online: true, device_type: 'browser' },
          { device_id: 'd3', tags: {}, online: true, device_type: 'cocoon' },
        ],
      },
      'test',
    );

    const cocoons = iface.cocoonDevices();
    expect(cocoons).toHaveLength(2);
    expect(cocoons.every((d) => d.device_type === 'cocoon')).toBe(true);
  });
});

describe('CocoonPluginInterface — destroy', () => {
  it('clears connections and devices on destroy', () => {
    const bus = makeBus();
    const iface = CocoonPluginInterface.create('p');
    iface.init(bus);

    bus.emit(
      CocoonBusKey.ConnectionAdded,
      { id: 'c1', connection: makeConnection('c1') },
      'test',
    );
    bus.emit(
      AdiSignalingBusKey.Devices,
      {
        url: 'wss://x',
        devices: [{ device_id: 'd1', tags: {}, online: true }],
      },
      'test',
    );

    iface.destroy();

    expect(iface.allConnections()).toEqual([]);
    expect(iface.allDevices()).toEqual([]);
    expect(() => iface.bus).toThrow('bus not initialized');
  });

  it('unsubscribes from bus events on destroy', () => {
    const bus = makeBus();
    const iface = CocoonPluginInterface.create('p');
    iface.init(bus);

    iface.destroy();

    bus.emit(
      CocoonBusKey.ConnectionAdded,
      { id: 'c1', connection: makeConnection('c1') },
      'test',
    );
    expect(iface.allConnections()).toEqual([]);
  });
});
