import { AdiPlugin } from '@adi-family/sdk-plugin';

declare module '@adi-family/sdk-plugin' {
  interface PluginApiRegistry {
    'adi.cocoon': {
      createClient(cocoonId: string, signalingUrl: string, rtcConfig?: RTCConfiguration): unknown;
    };
  }
}
import { AdiRouterBusKey } from '@adi-family/plugin-router/bus';
import { AdiSignalingBusKey, type DeviceInfo, type IceServer } from '@adi-family/plugin-signaling/bus';
import type { Connection } from '@adi-family/cocoon-plugin-interface';
import * as api from './generated/adi-client.js';
import { cocoon } from './cocoon.js';
import type { Credential } from './types.js';
import './events.js';

export class CredentialsPlugin extends AdiPlugin {
  readonly id = 'adi.credentials';
  readonly version = '0.1.0';

  private readonly devices = new Map<string, { info: DeviceInfo; signalingUrl: string }>();
  private iceServers: IceServer[] | undefined;

  get api() { return api; }

  private ensureConnection(cocoonId: string): Connection {
    try {
      return cocoon.getConnection(cocoonId);
    } catch {
      const tracked = this.devices.get(cocoonId);
      if (!tracked) throw new Error(`Device '${cocoonId}' not found`);
      const cocoonApi = this.app.api('adi.cocoon');
      const rtcConfig = this.iceServers ? { iceServers: this.iceServers } : undefined;
      cocoonApi.createClient(cocoonId, tracked.signalingUrl, rtcConfig);
      return cocoon.getConnection(cocoonId);
    }
  }

  private onBus<P>(
    event: string,
    handler: (params: P) => Promise<void>,
  ): void {
    this.bus.on(event, async (params: P) => {
      try {
        await handler(params);
      } catch (err) {
        console.error(`[CredentialsPlugin] ${event} error:`, err);
        this.bus.emit('credentials:error', {
          message: err instanceof Error ? err.message : String(err),
          event,
        }, 'credentials');
      }
    }, 'credentials');
  }

  async onRegister(): Promise<void> {
    cocoon.init(this.bus);

    const { AdiCredentialsElement } = await import('./component.js');
    if (!customElements.get('adi-credentials')) {
      customElements.define('adi-credentials', AdiCredentialsElement);
    }

    this.bus.emit(AdiRouterBusKey.RegisterRoute, {
      pluginId: this.id,
      path: '',
      init: () => document.createElement('adi-credentials'),
      label: 'Credentials',
    }, this.id);

    this.bus.emit('adi.actions-feed:nav-add', {
      id: this.id,
      label: 'Credentials',
      path: `/${this.id}`,
    }, this.id);

    this.bus.on(AdiSignalingBusKey.ConnectionInfo, ({ connectionInfo }) => {
      this.iceServers = connectionInfo.ice_servers;
    }, this.id);

    this.bus.on(AdiSignalingBusKey.Devices, ({ url, devices }) => {
      for (const d of devices) {
        if (d.device_type === 'cocoon') {
          this.devices.set(d.device_id, { info: d, signalingUrl: url });
        }
      }
    }, this.id);

    this.onBus<{ credential_type?: any; provider?: string }>('credentials:list', async ({ credential_type, provider }) => {
      const conns = cocoon.connectionsWithPlugin('adi.credentials');
      const results = await Promise.allSettled(
        conns.map(c => api.list(c, { credential_type, provider })),
      );
      const credentials: Credential[] = results.flatMap((r, i) =>
        r.status === 'fulfilled'
          ? r.value.map(cred => ({ ...cred, cocoonId: conns[i].id }))
          : [],
      );
      this.bus.emit('credentials:list-changed', { credentials }, 'credentials');
    });

    this.onBus<{ id: string; cocoonId: string }>('credentials:get', async ({ id, cocoonId }) => {
      const cred = await api.get(this.ensureConnection(cocoonId), id);
      this.bus.emit('credentials:detail-changed', { credential: { ...cred, cocoonId } }, 'credentials');
    });

    this.onBus<{ id: string; cocoonId: string }>('credentials:reveal', async ({ id, cocoonId }) => {
      const cred = await api.getWithData(this.ensureConnection(cocoonId), id);
      this.bus.emit('credentials:data-revealed', { credential: { ...cred, cocoonId } }, 'credentials');
    });

    this.onBus<{ cocoonId: string; [key: string]: any }>('credentials:create', async ({ cocoonId, ...params }) => {
      const cred = await api.create(this.ensureConnection(cocoonId), params);
      this.bus.emit('credentials:mutated', { credential: { ...cred, cocoonId } }, 'credentials');
    });

    this.onBus<{ cocoonId: string; [key: string]: any }>('credentials:update', async ({ cocoonId, ...params }) => {
      const cred = await api.update(this.ensureConnection(cocoonId), params);
      this.bus.emit('credentials:mutated', { credential: { ...cred, cocoonId } }, 'credentials');
    });

    this.onBus<{ id: string; cocoonId: string }>('credentials:delete', async ({ id, cocoonId }) => {
      await api.delete(this.ensureConnection(cocoonId), id);
      this.bus.emit('credentials:deleted', { id, cocoonId }, 'credentials');
    });

    this.onBus<{ id: string; cocoonId: string }>('credentials:verify', async ({ id, cocoonId }) => {
      const result = await api.verify(this.ensureConnection(cocoonId), id);
      this.bus.emit('credentials:verified', { id, result }, 'credentials');
    });

    this.onBus<{ id: string; cocoonId: string }>('credentials:logs', async ({ id, cocoonId }) => {
      const logs = await api.accessLogs(this.ensureConnection(cocoonId), id);
      this.bus.emit('credentials:logs-changed', { id, logs }, 'credentials');
    });
  }
}
