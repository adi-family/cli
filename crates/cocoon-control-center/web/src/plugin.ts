import '@adi-family/plugin-cocoon';
import '@adi-family/plugin-signaling';
import { AdiPlugin } from '@adi-family/sdk-plugin';
import { AdiSignalingBusKey, type DeviceInfo, type IceServer } from '@adi-family/plugin-signaling/bus';
import { AdiRouterBusKey } from '@adi-family/plugin-router/bus';
import { AdiDebugScreenBusKey } from '@adi-family/plugin-debug-screen/bus';
import { PLUGIN_ID, PLUGIN_VERSION } from './config';
import type { AdiCocoonControlCenterElement, ControlCenterCocoon } from './component';
import type { AdiCocoonControlCenterDebugElement } from './debug-section';
import './bus';

interface TrackedDevice {
  info: DeviceInfo;
  signalingUrl: string;
}

export class CocoonControlCenterPlugin extends AdiPlugin {
  readonly id = PLUGIN_ID;
  readonly version = PLUGIN_VERSION;

  get api() { return this; }

  private controlEl: AdiCocoonControlCenterElement | null = null;
  private debugEl: AdiCocoonControlCenterDebugElement | null = null;
  private readonly devices = new Map<string, TrackedDevice>();
  private readonly unsubs: (() => void)[] = [];
  private iceServers: IceServer[] | undefined;

  override async onRegister(): Promise<void> {
    const [{ AdiCocoonControlCenterElement }, { AdiCocoonTerminalElement }, { AdiCocoonControlCenterDebugElement }] = await Promise.all([
      import('./component.js'),
      import('./terminal.js'),
      import('./debug-section.js'),
    ]);

    if (!customElements.get('adi-cocoon-terminal')) {
      customElements.define('adi-cocoon-terminal', AdiCocoonTerminalElement);
    }
    if (!customElements.get('adi-cocoon-control-center')) {
      customElements.define('adi-cocoon-control-center', AdiCocoonControlCenterElement);
    }
    if (!customElements.get('adi-cocoon-control-center-debug')) {
      customElements.define('adi-cocoon-control-center-debug', AdiCocoonControlCenterDebugElement);
    }

    this.bus.emit(AdiDebugScreenBusKey.RegisterSection, {
      pluginId: PLUGIN_ID,
      init: () => {
        this.debugEl = document.createElement('adi-cocoon-control-center-debug') as AdiCocoonControlCenterDebugElement;
        this.debugEl.clientProvider = (deviceId) => this.resolveClient(deviceId);
        this.syncToDebug();
        return this.debugEl;
      },
      label: 'Control Center',
    }, PLUGIN_ID);

    this.bus.emit(AdiRouterBusKey.RegisterRoute, {
      pluginId: PLUGIN_ID,
      path: '',
      init: () => {
        this.controlEl = document.createElement('adi-cocoon-control-center') as AdiCocoonControlCenterElement;
        this.controlEl.clientProvider = (deviceId) => this.resolveClient(deviceId);
        this.syncToComponent();
        return this.controlEl;
      },
      label: 'Terminal',
    }, PLUGIN_ID);

    this.unsubs.push(
      this.bus.on(AdiSignalingBusKey.ConnectionInfo, ({ connectionInfo }) => {
        this.iceServers = connectionInfo.ice_servers;
      }, PLUGIN_ID),
      this.bus.on(AdiSignalingBusKey.Devices, ({ url, devices }) => {
        for (const d of devices) {
          if (d.device_type === 'cocoon') {
            this.devices.set(d.device_id, { info: d, signalingUrl: url });
          }
        }
        this.syncToComponent();
      }, PLUGIN_ID),
      this.bus.on(AdiSignalingBusKey.DeviceUpdated, ({ url, deviceId, tags, deviceConfig }) => {
        const existing = this.devices.get(deviceId);
        if (existing) {
          this.devices.set(deviceId, {
            signalingUrl: url,
            info: { ...existing.info, tags, device_config: deviceConfig },
          });
          this.syncToComponent();
        }
      }, PLUGIN_ID),
      this.bus.on(AdiSignalingBusKey.DeviceDeregistered, ({ deviceId }) => {
        const tracked = this.devices.get(deviceId);
        if (tracked) {
          this.devices.set(deviceId, { ...tracked, info: { ...tracked.info, online: false } });
          this.syncToComponent();
        }
      }, PLUGIN_ID),
    );
  }

  override onUnregister(): void {
    this.unsubs.forEach(fn => fn());
    this.unsubs.length = 0;
    this.devices.clear();
    this.controlEl = null;
    this.debugEl = null;
  }

  private buildCocoonList(): ControlCenterCocoon[] {
    const items: ControlCenterCocoon[] = [];
    for (const [deviceId, { info, signalingUrl }] of this.devices) {
      items.push({ deviceId, signalingUrl, online: info.online, name: info.tags?.name });
    }
    items.sort((a, b) => {
      if (a.online !== b.online) return a.online ? -1 : 1;
      return (a.name ?? a.deviceId).localeCompare(b.name ?? b.deviceId);
    });
    return items;
  }

  private syncToDebug(): void {
    if (!this.debugEl) return;
    this.debugEl.cocoons = this.buildCocoonList();
  }

  private resolveClient(deviceId: string) {
    const cocoonApi = this.app.api('adi.cocoon');
    const existing = cocoonApi.getClient(deviceId);
    if (existing) return existing;
    const tracked = this.devices.get(deviceId);
    if (!tracked) return undefined;
    const rtcConfig = this.iceServers ? { iceServers: this.iceServers } : undefined;
    return cocoonApi.createClient(deviceId, tracked.signalingUrl, rtcConfig);
  }

  private syncToComponent(): void {
    const items = this.buildCocoonList();
    if (this.controlEl) this.controlEl.cocoons = items;
    if (this.debugEl) this.debugEl.cocoons = items;
  }
}
