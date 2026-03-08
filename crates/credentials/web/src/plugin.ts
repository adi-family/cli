import { AdiPlugin } from '@adi-family/sdk-plugin';
import { AdiRouterBusKey } from '@adi/router-web-plugin/bus';
import * as api from './api.js';
import { cocoon } from './cocoon.js';
import type { Credential } from './types.js';
import './events.js';

export class CredentialsPlugin extends AdiPlugin {
  readonly id = 'adi.credentials';
  readonly version = '0.1.0';

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
        const cred = await api.getCredential(cocoon.getConnection(cocoonId), id);
        this.bus.emit('credentials:detail-changed', {
          credential: { ...cred, cocoonId },
        }, 'credentials');
      } catch (err) {
        console.error('[CredentialsPlugin] get error:', err);
      }
    }, 'credentials');

    this.bus.on('credentials:reveal', async ({ id, cocoonId }) => {
      try {
        const cred = await api.getCredentialWithData(cocoon.getConnection(cocoonId), id);
        this.bus.emit('credentials:data-revealed', {
          credential: { ...cred, cocoonId },
        }, 'credentials');
      } catch (err) {
        console.error('[CredentialsPlugin] reveal error:', err);
      }
    }, 'credentials');

    this.bus.on('credentials:create', async ({ cocoonId, ...params }) => {
      try {
        const cred = await api.createCredential(cocoon.getConnection(cocoonId), params);
        this.bus.emit('credentials:mutated', { credential: { ...cred, cocoonId } }, 'credentials');
      } catch (err) {
        console.error('[CredentialsPlugin] create error:', err);
      }
    }, 'credentials');

    this.bus.on('credentials:update', async ({ cocoonId, ...params }) => {
      try {
        const cred = await api.updateCredential(cocoon.getConnection(cocoonId), params);
        this.bus.emit('credentials:mutated', { credential: { ...cred, cocoonId } }, 'credentials');
      } catch (err) {
        console.error('[CredentialsPlugin] update error:', err);
      }
    }, 'credentials');

    this.bus.on('credentials:delete', async ({ id, cocoonId }) => {
      try {
        await api.deleteCredential(cocoon.getConnection(cocoonId), id);
        this.bus.emit('credentials:deleted', { id, cocoonId }, 'credentials');
      } catch (err) {
        console.error('[CredentialsPlugin] delete error:', err);
      }
    }, 'credentials');

    this.bus.on('credentials:verify', async ({ id, cocoonId }) => {
      try {
        const result = await api.verifyCredential(cocoon.getConnection(cocoonId), id);
        this.bus.emit('credentials:verified', { id, result }, 'credentials');
      } catch (err) {
        console.error('[CredentialsPlugin] verify error:', err);
      }
    }, 'credentials');

    this.bus.on('credentials:logs', async ({ id, cocoonId }) => {
      try {
        const logs = await api.getAccessLogs(cocoon.getConnection(cocoonId), id);
        this.bus.emit('credentials:logs-changed', { id, logs }, 'credentials');
      } catch (err) {
        console.error('[CredentialsPlugin] logs error:', err);
      }
    }, 'credentials');
  }
}
