import '@adi-family/plugin-cocoon';
import { AdiPlugin } from '@adi-family/sdk-plugin';
import { AdiRouterBusKey } from '@adi-family/plugin-router';
import { NavBusKey } from '@adi-family/plugin-actions-feed';
import { AdiSignalingBusKey, type DeviceInfo, type IceServer } from '@adi-family/plugin-signaling';
import type { Connection } from '@adi-family/cocoon-plugin-interface';
import * as api from './generated/adi-client.js';
import { cocoon } from './cocoon.js';
import {
  AdiCredentialsBusKey,
  type AdiCredentialsListEvent,
  type AdiCredentialsGetEvent,
  type AdiCredentialsRevealEvent,
  type AdiCredentialsCreateEvent,
  type AdiCredentialsUpdateEvent,
  type AdiCredentialsDeleteEvent,
  type AdiCredentialsVerifyEvent,
  type AdiCredentialsLogsEvent,
} from './generated/bus-types.js';
import './generated/bus-events.js';

export class CredentialsPlugin extends AdiPlugin {
  readonly id = 'adi.credentials';
  readonly version = '0.1.0';

  private readonly devices = new Map<string, { info: DeviceInfo; signalingUrl: string }>();
  private iceServers: IceServer[] | undefined;

  get api() { return api; }

  private async ensureConnection(cocoonId: string): Promise<Connection> {
    try {
      return cocoon.getConnection(cocoonId);
    } catch {
      const tracked = this.devices.get(cocoonId);
      if (!tracked) throw new Error(`Device '${cocoonId}' not found`);
      const cocoonApi = await this.app.api('adi.cocoon');
      const rtcConfig = this.iceServers ? { iceServers: this.iceServers } : undefined;
      await cocoonApi.createClient(cocoonId, tracked.signalingUrl, rtcConfig);
      return cocoon.getConnection(cocoonId);
    }
  }

  private onBus<P>(
    event: AdiCredentialsBusKey,
    handler: (params: P) => Promise<void>,
  ): void {
    this.bus.on(event, async (params: P) => {
      try {
        await handler(params);
      } catch (err) {
        console.error(`[CredentialsPlugin] ${event} error:`, err);
        this.bus.emit(AdiCredentialsBusKey.Error, {
          message: err instanceof Error ? err.message : String(err),
          event,
        }, 'credentials');
      }
    }, 'credentials');
  }

  async onRegister(): Promise<void> {
    cocoon.init(this.bus);
    cocoon.connectProvider = (deviceId: string) => this.ensureConnection(deviceId);

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

    this.bus.emit(NavBusKey.Add, {
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

    this.onBus<AdiCredentialsListEvent>(AdiCredentialsBusKey.List, async ({ credential_type, provider }) => {
      const conns = cocoon.connectionsWithPlugin('adi.credentials');
      const results = await Promise.allSettled(
        conns.map(c => api.list(c, { credential_type, provider })),
      );
      const credentials = results.flatMap((r, i) =>
        r.status === 'fulfilled'
          ? r.value.map(cred => ({ ...cred, cocoonId: conns[i].id }))
          : [],
      );
      this.bus.emit(AdiCredentialsBusKey.ListChanged, { credentials }, 'credentials');
    });

    this.onBus<AdiCredentialsGetEvent>(AdiCredentialsBusKey.Get, async ({ id, cocoonId }) => {
      const cred = await api.get(this.ensureConnection(cocoonId), id);
      this.bus.emit(AdiCredentialsBusKey.DetailChanged, { credential: { ...cred, cocoonId } }, 'credentials');
    });

    this.onBus<AdiCredentialsRevealEvent>(AdiCredentialsBusKey.Reveal, async ({ id, cocoonId }) => {
      const cred = await api.getWithData(this.ensureConnection(cocoonId), id);
      this.bus.emit(AdiCredentialsBusKey.DataRevealed, { credential: { ...cred, cocoonId } }, 'credentials');
    });

    this.onBus<AdiCredentialsCreateEvent>(AdiCredentialsBusKey.Create, async ({ cocoonId, ...params }) => {
      const cred = await api.create(this.ensureConnection(cocoonId), params);
      this.bus.emit(AdiCredentialsBusKey.Mutated, { credential: { ...cred, cocoonId } }, 'credentials');
    });

    this.onBus<AdiCredentialsUpdateEvent>(AdiCredentialsBusKey.Update, async ({ cocoonId, ...params }) => {
      const cred = await api.update(this.ensureConnection(cocoonId), params);
      this.bus.emit(AdiCredentialsBusKey.Mutated, { credential: { ...cred, cocoonId } }, 'credentials');
    });

    this.onBus<AdiCredentialsDeleteEvent>(AdiCredentialsBusKey.Delete, async ({ id, cocoonId }) => {
      await api.delete_(this.ensureConnection(cocoonId), id);
      this.bus.emit(AdiCredentialsBusKey.Deleted, { id, cocoonId }, 'credentials');
    });

    this.onBus<AdiCredentialsVerifyEvent>(AdiCredentialsBusKey.Verify, async ({ id, cocoonId }) => {
      const result = await api.verify(this.ensureConnection(cocoonId), id);
      this.bus.emit(AdiCredentialsBusKey.Verified, { id, result }, 'credentials');
    });

    this.onBus<AdiCredentialsLogsEvent>(AdiCredentialsBusKey.Logs, async ({ id, cocoonId }) => {
      const logs = await api.accessLogs(this.ensureConnection(cocoonId), id);
      this.bus.emit(AdiCredentialsBusKey.LogsChanged, { id, logs }, 'credentials');
    });
  }
}
