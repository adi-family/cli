import type { EventBus } from '@adi-family/sdk-plugin';
import type { Connection, DeviceInfo } from '@adi-family/plugin-signaling';
import { AdiSignalingBusKey } from '@adi-family/plugin-signaling';
import { CocoonBusKey, type ConnectionSettings } from './bus-keys.js';

export class CocoonPluginInterface {
  private readonly connections = new Map<string, Connection>();
  private readonly devices = new Map<string, DeviceInfo>();
  private readonly settings = new Map<string, ConnectionSettings>();
  private readonly pluginId: string;
  private _bus: EventBus | undefined;
  private unsubs: Array<() => void> = [];

  private constructor(pluginId: string) {
    this.pluginId = pluginId;
  }

  static create(pluginId: string): CocoonPluginInterface {
    return new CocoonPluginInterface(pluginId);
  }

  init(bus: EventBus): void {
    this._bus = bus;

    this.unsubs.push(
      bus.on(CocoonBusKey.ConnectionAdded, ({ id, connection }) => {
        this.connections.set(id, connection);
      }, this.pluginId),

      bus.on(CocoonBusKey.ConnectionRemoved, ({ id }) => {
        this.connections.delete(id);
      }, this.pluginId),

      bus.on(AdiSignalingBusKey.Devices, ({ devices }) => {
        this.devices.clear();
        for (const d of devices) {
          this.devices.set(d.device_id, d);
        }
      }, this.pluginId),

      bus.on(CocoonBusKey.SettingsChanged, ({ id, settings }) => {
        this.settings.set(id, settings);
      }, this.pluginId),
    );

  }

  destroy(): void {
    this.unsubs.forEach(fn => fn());
    this.unsubs = [];
    this.connections.clear();
    this.devices.clear();
    this.settings.clear();
    this._bus = undefined;
  }

  get bus(): EventBus {
    if (!this._bus) throw new Error(`${this.pluginId}: bus not initialized`);
    return this._bus;
  }

  getConnection(cocoonId: string): Connection {
    const c = this.connections.get(cocoonId);
    if (!c) throw new Error(`Connection '${cocoonId}' not found`);
    return c;
  }

  getSettings(cocoonId: string): ConnectionSettings {
    return this.settings.get(cocoonId) ?? {};
  }

  connectionsWithPlugin(pluginId: string): Connection[] {
    return [...this.connections.values()]
      .filter(c => c.plugins.includes(pluginId));
  }

  allConnections(): Connection[] {
    return [...this.connections.values()];
  }

  allDevices(): DeviceInfo[] {
    return [...this.devices.values()];
  }

  cocoonDevices(): DeviceInfo[] {
    return this.allDevices().filter(d => d.device_type === 'cocoon');
  }
}
