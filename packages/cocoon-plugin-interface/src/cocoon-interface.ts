import type { EventBus } from '@adi-family/sdk-plugin';
import type { Connection } from '@adi/signaling-web-plugin/bus';
import { CocoonBusKey } from './bus-keys.js';

export class CocoonPluginInterface {
  private readonly connections = new Map<string, Connection>();
  private readonly pluginId: string;
  private _bus: EventBus | undefined;
  private unsubAdded: (() => void) | undefined;
  private unsubRemoved: (() => void) | undefined;

  private constructor(pluginId: string) {
    this.pluginId = pluginId;
  }

  static create(pluginId: string): CocoonPluginInterface {
    return new CocoonPluginInterface(pluginId);
  }

  init(bus: EventBus): void {
    this._bus = bus;

    this.unsubAdded = bus.on(CocoonBusKey.ConnectionAdded, ({ id, connection }) => {
      this.connections.set(id, connection);
    }, this.pluginId);

    this.unsubRemoved = bus.on(CocoonBusKey.ConnectionRemoved, ({ id }) => {
      this.connections.delete(id);
    }, this.pluginId);
  }

  destroy(): void {
    this.unsubAdded?.();
    this.unsubRemoved?.();
    this.connections.clear();
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

  connectionsWithService(serviceName: string): Connection[] {
    return [...this.connections.values()]
      .filter(c => c.services.includes(serviceName));
  }

  allConnections(): Connection[] {
    return [...this.connections.values()];
  }
}
