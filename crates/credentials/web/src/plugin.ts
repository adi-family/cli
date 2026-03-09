import { AdiPlugin } from '@adi-family/sdk-plugin';
import { AdiRouterBusKey } from '@adi/router-web-plugin/bus';
import { AdiSignalingBusKey, type DeviceInfo, type IceServer } from '@adi/signaling-web-plugin/bus';
import type { Connection } from '@adi-family/cocoon-plugin-interface';
import * as api from './api.js';
import { cocoon } from './cocoon.js';
import type { Credential } from './types.js';
import './events.js';

export class CredentialsPlugin extends AdiPlugin {
  readonly id = 'adi.credentials';
  readonly version = '0.1.0';

  private readonly devices = new Map<string, { info: DeviceInfo; signalingUrl: string }>();
  private iceServers: IceServer[] | undefined;

  get api() { return api; }

  /** Ensures cocoon client+connection exist for this device, returns the Connection. */
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

    this.bus.emit('nav:add', {
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

    this.bus.on('credentials:list', async ({ credential_type, provider }) => {
      try {
        const conns = cocoon.connectionsWithService('credentials');
        const results = await Promise.allSettled(
          conns.map(c => api.listCredentials(c, { credential_type, provider })),
        );
        const credentials: Credential[] = results.flatMap((r, i) =>
          r.status === 'fulfilled'
            ? r.value.map(cred => ({ ...cred, cocoonId: conns[i].id }))
            : [],
        );
        this.bus.emit('credentials:list-changed', { credentials }, 'credentials');
      } catch (err) {
        console.error('[CredentialsPlugin] list error:', err);
        this.bus.emit('credentials:list-changed', { credentials: [] }, 'credentials');
      }
    }, 'credentials');

    this.bus.on('credentials:get', async ({ id, cocoonId }) => {
      try {
        const cred = await api.getCredential(this.ensureConnection(cocoonId), id);
        this.bus.emit('credentials:detail-changed', {
          credential: { ...cred, cocoonId },
        }, 'credentials');
      } catch (err) {
        console.error('[CredentialsPlugin] get error:', err);
      }
    }, 'credentials');

    this.bus.on('credentials:reveal', async ({ id, cocoonId }) => {
      try {
        const cred = await api.getCredentialWithData(this.ensureConnection(cocoonId), id);
        this.bus.emit('credentials:data-revealed', {
          credential: { ...cred, cocoonId },
        }, 'credentials');
      } catch (err) {
        console.error('[CredentialsPlugin] reveal error:', err);
      }
    }, 'credentials');

    this.bus.on('credentials:create', async ({ cocoonId, ...params }) => {
      try {
        const cred = await api.createCredential(this.ensureConnection(cocoonId), params);
        this.bus.emit('credentials:mutated', { credential: { ...cred, cocoonId } }, 'credentials');
      } catch (err) {
        console.error('[CredentialsPlugin] create error:', err);
      }
    }, 'credentials');

    this.bus.on('credentials:update', async ({ cocoonId, ...params }) => {
      try {
        const cred = await api.updateCredential(this.ensureConnection(cocoonId), params);
        this.bus.emit('credentials:mutated', { credential: { ...cred, cocoonId } }, 'credentials');
      } catch (err) {
        console.error('[CredentialsPlugin] update error:', err);
      }
    }, 'credentials');

    this.bus.on('credentials:delete', async ({ id, cocoonId }) => {
      try {
        await api.deleteCredential(this.ensureConnection(cocoonId), id);
        this.bus.emit('credentials:deleted', { id, cocoonId }, 'credentials');
      } catch (err) {
        console.error('[CredentialsPlugin] delete error:', err);
      }
    }, 'credentials');

    this.bus.on('credentials:verify', async ({ id, cocoonId }) => {
      try {
        const result = await api.verifyCredential(this.ensureConnection(cocoonId), id);
        this.bus.emit('credentials:verified', { id, result }, 'credentials');
      } catch (err) {
        console.error('[CredentialsPlugin] verify error:', err);
      }
    }, 'credentials');

    this.bus.on('credentials:logs', async ({ id, cocoonId }) => {
      try {
        const logs = await api.getAccessLogs(this.ensureConnection(cocoonId), id);
        this.bus.emit('credentials:logs-changed', { id, logs }, 'credentials');
      } catch (err) {
        console.error('[CredentialsPlugin] logs error:', err);
      }
    }, 'credentials');
  }
}
