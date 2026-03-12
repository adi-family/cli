import { expect, oneEvent, fixtureCleanup } from '@open-wc/testing';
import { CocoonPluginInterface } from './cocoon-interface.js';
import { CocoonBusKey, type Connection } from './bus-keys.js';
import { AdiSignalingBusKey } from '@adi-family/plugin-signaling';
import type { EventBus } from '@adi-family/sdk-plugin';
import { CocoonSelectElement } from './cocoon-select.js';

if (!customElements.get('cocoon-select')) {
  customElements.define('cocoon-select', CocoonSelectElement);
}

type Handler = (payload: any) => void;

function makeBus(): EventBus {
  const handlers = new Map<string, Handler[]>();
  return {
    on(key: string, handler: Handler, _producer: string) {
      const list = handlers.get(key) ?? [];
      list.push(handler);
      handlers.set(key, list);
      return () => {
        const idx = list.indexOf(handler);
        if (idx >= 0) list.splice(idx, 1);
      };
    },
    emit(key: string, payload: unknown, _producer: string) {
      for (const h of handlers.get(key) ?? []) h(payload);
    },
  } as unknown as EventBus;
}

function makeConnection(id: string, plugins: string[] = []): Connection {
  return {
    id,
    plugins,
    request: () => Promise.resolve(),
    stream: async function* () {},
    httpProxy: () => Promise.resolve(new Response()),
    httpDirect: () => Promise.resolve(new Response()),
    refreshPlugins: () => Promise.resolve(plugins),
    installPlugin: () => Promise.resolve(),
    dispose: () => {},
  } as unknown as Connection;
}

interface SetupOpts {
  withPlugin?: string;
  value?: string;
  label?: string;
  devices?: Array<{ device_id: string; online: boolean; device_type?: string }>;
  connections?: Array<{ id: string; plugins?: string[] }>;
}

async function createElement(opts: SetupOpts = {}): Promise<{ el: CocoonSelectElement; bus: EventBus; iface: CocoonPluginInterface }> {
  const bus = makeBus();
  const iface = CocoonPluginInterface.create('test');
  iface.init(bus);

  const el = document.createElement('cocoon-select') as CocoonSelectElement;
  el.cocoonInterface = iface;
  if (opts.withPlugin) el.withPlugin = opts.withPlugin;
  if (opts.value) el.value = opts.value;
  if (opts.label) el.label = opts.label;

  if (opts.devices) {
    bus.emit(AdiSignalingBusKey.Devices, {
      url: 'wss://test',
      devices: opts.devices.map(d => ({
        device_id: d.device_id,
        tags: {},
        online: d.online,
        device_type: d.device_type ?? 'cocoon',
      })),
    }, 'test');
  }

  if (opts.connections) {
    for (const c of opts.connections) {
      const conn = makeConnection(c.id, c.plugins ?? []);
      bus.emit(CocoonBusKey.ConnectionAdded, { id: c.id, connection: conn }, 'test');
    }
  }

  document.body.appendChild(el);
  await el.updateComplete;
  return { el, bus, iface };
}

afterEach(() => {
  fixtureCleanup();
  document.body.innerHTML = '';
});

describe('CocoonSelectElement — defaults', () => {
  it('renders button with default label', async () => {
    const { el } = await createElement();
    const button = el.querySelector('button')!;
    expect(button).to.exist;
    expect(button.textContent).to.contain('Select Cocoon');
  });

  it('renders custom label', async () => {
    const { el } = await createElement({ label: 'Pick a Cocoon' });
    const button = el.querySelector('button')!;
    expect(button.textContent).to.contain('Pick a Cocoon');
  });

  it('renders no modal when closed', async () => {
    const { el } = await createElement();
    expect(el.querySelector('.overlay-backdrop')).to.be.null;
  });

  it('shows selected value in button with green dot', async () => {
    const { el } = await createElement({ value: 'device-1' });
    const button = el.querySelector('button')!;
    expect(button.textContent).to.contain('device-1');
    expect(button.querySelector('.text-green-400')).to.exist;
  });
});

describe('CocoonSelectElement — refresh', () => {
  it('builds items from devices and connections', async () => {
    const { el } = await createElement({
      devices: [
        { device_id: 'd1', online: true },
        { device_id: 'd2', online: false },
      ],
      connections: [{ id: 'd1', plugins: ['my-plugin'] }],
      withPlugin: 'my-plugin',
    });

    el.show();
    await el.updateComplete;

    const items = el.querySelectorAll('.font-mono');
    expect(items.length).to.equal(2);
  });

  it('shows empty state when no devices exist', async () => {
    const { el } = await createElement();
    el.show();
    await el.updateComplete;

    const emptyMsg = el.querySelector('.text-center');
    expect(emptyMsg).to.exist;
    expect(emptyMsg!.textContent).to.contain('No cocoons available');
  });

  it('clears items when interface is removed', async () => {
    const { el } = await createElement({
      devices: [{ device_id: 'd1', online: true }],
    });

    el.cocoonInterface = null;
    el.refresh();
    el.show();
    await el.updateComplete;

    const emptyMsg = el.querySelector('.text-center');
    expect(emptyMsg).to.exist;
  });
});

describe('CocoonSelectElement — show / close', () => {
  it('show() renders the modal', async () => {
    const { el } = await createElement();
    el.show();
    await el.updateComplete;

    expect(el.querySelector('.overlay-backdrop')).to.exist;
    expect(el.querySelector('.overlay-panel')).to.exist;
  });

  it('close() hides the modal', async () => {
    const { el } = await createElement();
    el.show();
    await el.updateComplete;
    expect(el.querySelector('.overlay-backdrop')).to.exist;

    el.close();
    await el.updateComplete;
    expect(el.querySelector('.overlay-backdrop')).to.be.null;
  });

  it('clicking backdrop closes modal', async () => {
    const { el } = await createElement();
    el.show();
    await el.updateComplete;

    const backdrop = el.querySelector('.overlay-backdrop') as HTMLElement;
    backdrop.click();
    await el.updateComplete;

    expect(el.querySelector('.overlay-backdrop')).to.be.null;
  });

  it('clicking close button closes modal', async () => {
    const { el } = await createElement();
    el.show();
    await el.updateComplete;

    const closeBtn = el.querySelector('.overlay-panel button') as HTMLElement;
    closeBtn.click();
    await el.updateComplete;

    expect(el.querySelector('.overlay-backdrop')).to.be.null;
  });
});

describe('CocoonSelectElement — select', () => {
  it('dispatches cocoon-selected with cocoonId and connection', async () => {
    const { el } = await createElement({
      devices: [{ device_id: 'd1', online: true }],
      connections: [{ id: 'd1', plugins: ['my-plugin'] }],
      withPlugin: 'my-plugin',
    });

    el.show();
    await el.updateComplete;

    const listener = oneEvent(el, 'cocoon-selected');
    const itemRow = el.querySelectorAll('.cursor-pointer')[0] as HTMLElement;
    itemRow.click();

    const event = await listener;
    expect(event.detail.cocoonId).to.equal('d1');
    expect(event.detail.connection).to.exist;
    expect(event.detail.connection.id).to.equal('d1');
  });

  it('sets value after selection', async () => {
    const { el } = await createElement({
      devices: [{ device_id: 'd1', online: true }],
      connections: [{ id: 'd1', plugins: ['my-plugin'] }],
      withPlugin: 'my-plugin',
    });

    el.show();
    await el.updateComplete;

    const itemRow = el.querySelectorAll('.cursor-pointer')[0] as HTMLElement;
    itemRow.click();
    await el.updateComplete;

    expect(el.value).to.equal('d1');
  });

  it('closes modal after selection', async () => {
    const { el } = await createElement({
      devices: [{ device_id: 'd1', online: true }],
      connections: [{ id: 'd1', plugins: ['my-plugin'] }],
      withPlugin: 'my-plugin',
    });

    el.show();
    await el.updateComplete;

    const itemRow = el.querySelectorAll('.cursor-pointer')[0] as HTMLElement;
    itemRow.click();
    await el.updateComplete;

    expect(el.querySelector('.overlay-backdrop')).to.be.null;
  });

  it('does not dispatch for offline items', async () => {
    const { el } = await createElement({
      devices: [{ device_id: 'd1', online: false }],
    });

    el.show();
    await el.updateComplete;

    let fired = false;
    el.addEventListener('cocoon-selected', () => { fired = true; });

    const items = el.querySelectorAll('.flex.items-center.justify-between.px-4');
    (items[0] as HTMLElement).click();
    await el.updateComplete;

    expect(fired).to.be.false;
  });

  it('does not dispatch when plugin not installed', async () => {
    const { el } = await createElement({
      devices: [{ device_id: 'd1', online: true }],
      connections: [{ id: 'd1', plugins: [] }],
      withPlugin: 'my-plugin',
    });

    el.show();
    await el.updateComplete;

    let fired = false;
    el.addEventListener('cocoon-selected', () => { fired = true; });

    const items = el.querySelectorAll('.flex.items-center.justify-between.px-4');
    (items[0] as HTMLElement).click();
    await el.updateComplete;

    expect(fired).to.be.false;
  });
});

describe('CocoonSelectElement — installPlugin', () => {
  it('shows Install Plugin button when plugin not installed', async () => {
    const { el } = await createElement({
      devices: [{ device_id: 'd1', online: true }],
      connections: [{ id: 'd1', plugins: [] }],
      withPlugin: 'my-plugin',
    });

    el.show();
    await el.updateComplete;

    const installBtn = Array.from(el.querySelectorAll('button')).find(
      b => b.textContent?.includes('Install Plugin'),
    );
    expect(installBtn).to.exist;
  });

  it('calls installPlugin on connection when clicked', async () => {
    const bus = makeBus();
    const iface = CocoonPluginInterface.create('test');
    iface.init(bus);

    const conn = makeConnection('d1', []);
    let installCalled = false;
    let installedPluginId = '';
    conn.installPlugin = (pluginId: string) => {
      installCalled = true;
      installedPluginId = pluginId;
      return Promise.resolve();
    };
    conn.refreshPlugins = () => Promise.resolve([]);

    bus.emit(AdiSignalingBusKey.Devices, {
      url: 'wss://test',
      devices: [{ device_id: 'd1', tags: {}, online: true, device_type: 'cocoon' }],
    }, 'test');
    bus.emit(CocoonBusKey.ConnectionAdded, { id: 'd1', connection: conn }, 'test');

    const el = document.createElement('cocoon-select') as CocoonSelectElement;
    el.cocoonInterface = iface;
    el.withPlugin = 'my-plugin';
    document.body.appendChild(el);
    await el.updateComplete;

    el.show();
    await el.updateComplete;

    const installBtn = Array.from(el.querySelectorAll('button')).find(
      b => b.textContent?.includes('Install Plugin'),
    )!;
    installBtn.click();

    // Wait for async installPlugin to complete
    await new Promise(r => setTimeout(r, 50));
    await el.updateComplete;

    expect(installCalled).to.be.true;
    expect(installedPluginId).to.equal('my-plugin');
  });

  it('shows error on install failure', async () => {
    const bus = makeBus();
    const iface = CocoonPluginInterface.create('test');
    iface.init(bus);

    const conn = makeConnection('d1', []);
    conn.installPlugin = () => Promise.reject(new Error('Network error'));
    conn.refreshPlugins = () => Promise.resolve([]);

    bus.emit(AdiSignalingBusKey.Devices, {
      url: 'wss://test',
      devices: [{ device_id: 'd1', tags: {}, online: true, device_type: 'cocoon' }],
    }, 'test');
    bus.emit(CocoonBusKey.ConnectionAdded, { id: 'd1', connection: conn }, 'test');

    const el = document.createElement('cocoon-select') as CocoonSelectElement;
    el.cocoonInterface = iface;
    el.withPlugin = 'my-plugin';
    document.body.appendChild(el);
    await el.updateComplete;

    el.show();
    await el.updateComplete;

    const installBtn = Array.from(el.querySelectorAll('button')).find(
      b => b.textContent?.includes('Install Plugin'),
    )!;
    installBtn.click();

    await new Promise(r => setTimeout(r, 50));
    await el.updateComplete;

    const errorEl = el.querySelector('.text-red-400');
    expect(errorEl).to.exist;
    expect(errorEl!.textContent).to.contain('Network error');
  });

  it('shows offline status for disconnected items', async () => {
    const { el } = await createElement({
      devices: [{ device_id: 'd1', online: false }],
    });

    el.show();
    await el.updateComplete;

    const offlineSpan = Array.from(el.querySelectorAll('span')).find(
      s => s.textContent?.includes('offline'),
    );
    expect(offlineSpan).to.exist;
  });
});

describe('CocoonSelectElement — requestSetup', () => {
  it('dispatches cocoon-setup-requested when setup button clicked', async () => {
    const { el } = await createElement();
    el.show();
    await el.updateComplete;

    const listener = oneEvent(el, 'cocoon-setup-requested');

    const setupBtn = Array.from(el.querySelectorAll('button')).find(
      b => b.textContent?.includes('Setup New Cocoon'),
    )!;
    setupBtn.click();

    const event = await listener;
    expect(event).to.exist;
    expect(event.type).to.equal('cocoon-setup-requested');
  });
});

describe('CocoonSelectElement — bus subscriptions', () => {
  it('refreshes when ConnectionAdded fires', async () => {
    const { el, bus } = await createElement({
      devices: [{ device_id: 'd1', online: true }],
    });

    el.show();
    await el.updateComplete;

    // Initially no connections, item shows offline
    const conn = makeConnection('d1', ['my-plugin']);
    bus.emit(CocoonBusKey.ConnectionAdded, { id: 'd1', connection: conn }, 'test');
    await el.updateComplete;

    // After connection added, the item should reflect connected state
    const offlineSpan = Array.from(el.querySelectorAll('span')).find(
      s => s.textContent?.trim() === 'offline',
    );
    expect(offlineSpan).to.be.undefined;
  });

  it('refreshes when ConnectionRemoved fires', async () => {
    const { el, bus } = await createElement({
      devices: [{ device_id: 'd1', online: true }],
      connections: [{ id: 'd1', plugins: ['my-plugin'] }],
      withPlugin: 'my-plugin',
    });

    el.show();
    await el.updateComplete;

    bus.emit(CocoonBusKey.ConnectionRemoved, { id: 'd1' }, 'test');
    await el.updateComplete;

    const offlineSpan = Array.from(el.querySelectorAll('span')).find(
      s => s.textContent?.trim() === 'offline',
    );
    expect(offlineSpan).to.exist;
  });

  it('refreshes when Devices event fires', async () => {
    const { el, bus } = await createElement();

    el.show();
    await el.updateComplete;
    expect(el.querySelector('.text-center')!.textContent).to.contain('No cocoons available');

    bus.emit(AdiSignalingBusKey.Devices, {
      url: 'wss://test',
      devices: [{ device_id: 'new-d', tags: {}, online: true, device_type: 'cocoon' }],
    }, 'test');
    await el.updateComplete;

    expect(el.querySelector('.text-center')).to.be.null;
    const deviceText = el.querySelector('.font-mono');
    expect(deviceText).to.exist;
    expect(deviceText!.textContent).to.contain('new-d');
  });

  it('unsubscribes when removed from DOM', async () => {
    const { el, bus, iface } = await createElement({
      devices: [{ device_id: 'd1', online: true }],
    });

    el.show();
    await el.updateComplete;

    // Remove from DOM — should unsubscribe from bus
    el.remove();

    // Emit new devices after removal
    bus.emit(AdiSignalingBusKey.Devices, {
      url: 'wss://test',
      devices: [
        { device_id: 'd1', tags: {}, online: true, device_type: 'cocoon' },
        { device_id: 'd2', tags: {}, online: true, device_type: 'cocoon' },
      ],
    }, 'test');

    // The interface received the update, but the element should NOT have refreshed
    // because it unsubscribed. Check the element's internal items stayed at 1.
    expect(iface.cocoonDevices()).to.have.length(2);
    // Element still has old items (refresh was not called by bus handler)
    el.show(); // re-trigger render with stale items
    expect(el.querySelectorAll('.font-mono').length).to.equal(1);
  });
});

describe('CocoonSelectElement — selected state', () => {
  it('highlights selected item in modal', async () => {
    const { el } = await createElement({
      devices: [{ device_id: 'd1', online: true }],
      connections: [{ id: 'd1', plugins: ['my-plugin'] }],
      withPlugin: 'my-plugin',
      value: 'd1',
    });

    el.show();
    await el.updateComplete;

    const selectedRow = el.querySelector('.bg-white\\/5.cursor-pointer');
    expect(selectedRow).to.exist;

    const selectedLabel = Array.from(el.querySelectorAll('span')).find(
      s => s.textContent?.trim() === 'selected',
    );
    expect(selectedLabel).to.exist;
  });
});
