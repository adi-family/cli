import { AdiPlugin } from '@adi-family/sdk-plugin';
import { AdiRouterBusKey } from '@adi-family/plugin-router';
import * as api from './api.js';
import * as store from './store.js';
import type { UserInfo } from './types.js';
import { setBus } from './context.js';
import './events.js';

const escapeHtml = (s: unknown): string =>
  String(s).replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');

export interface AuthApi {
  getToken(authDomain: string): Promise<string | null>;
  getUser(): UserInfo | null;
}

export class AuthPlugin extends AdiPlugin {
  readonly id = 'adi.auth';
  readonly version = '0.1.0';

  private currentUser: UserInfo | null = null;
  private pendingAuthActions = new Set<string>();

  get api(): AuthApi {
    return {
      getToken: (authDomain: string) => this.getToken(authDomain),
      getUser: () => this.currentUser,
    };
  }

  async onRegister(): Promise<void> {
    setBus(this.bus);
    store.init(this.app.storage(this.id));
    await store.migrateFromLocalStorage();

    const { AdiAuthElement } = await import('./component.js');
    if (!customElements.get('adi-auth')) {
      customElements.define('adi-auth', AdiAuthElement);
    }

    this.bus.emit(AdiRouterBusKey.RegisterRoute, { pluginId: this.id, path: '', init: () => document.createElement('adi-auth'), label: 'Auth' }, this.id);
    this.bus.emit('adi.actions-feed:nav-add', { id: this.id, label: 'Auth', path: `/${this.id}` }, this.id);

    this.bus.emit('adi.actions-feed:register-kind', {
      plugin: 'adi.auth',
      kind: 'auth-required',
      mode: 'exclusive',
    }, 'auth');

    this.registerActionRenderer();
    this.registerEventHandlers();
    await this.restoreSession();
  }

  private dismissPendingAuthActions(): void {
    for (const id of this.pendingAuthActions) {
      this.bus.emit('adi.actions-feed:dismiss', { id }, 'auth');
    }
    this.pendingAuthActions.clear();
  }

  private registerActionRenderer(): void {
    this.bus.emit('adi.actions-feed:register-renderer', {
      plugin: 'adi.auth',
      kind: 'auth-required',
      render: (data, actionId) => {
        const reason = escapeHtml(data.reason ?? 'Authentication required');
        const domain = escapeHtml(data.authDomain ?? '');
        const method = escapeHtml(data.authKind ?? 'unknown');

        return `
          <div class="space-y-2 px-4">
            <div class="auth-action-label text-xs">Auth Required</div>
            <div class="text-xs" style="color:var(--adi-text)">${reason}</div>
            ${domain ? `<div class="auth-action-detail text-xs">Domain: <span class="auth-action-mono">${domain}</span></div>` : ''}
            ${method !== 'unknown' ? `<div class="auth-action-detail text-xs">Method: <span class="auth-action-mono">${method}</span></div>` : ''}
            <adi-auth data-inline data-auth-url="${domain}"${data.authRequirement === 'required' ? ' data-auth-required' : ''}></adi-auth>
          </div>
        `;
      },
    }, 'auth');
  }

  private registerEventHandlers(): void {
    this.bus.on('auth:login', async ({ email, authUrl }) => {
      try {
        await api.requestCode(authUrl, email);
      } catch (err) {
        console.error('[AuthPlugin] auth:login error:', err);
      }
    }, 'auth');

    this.bus.on('auth:login-anonymous', async ({ authUrl }) => {
      try {
        const result = await api.loginAnonymous(authUrl);
        await store.save({
          accessToken: result.accessToken,
          email: '',
          expiresAt: Date.now() + result.expiresIn * 1000,
          authUrl,
        });
        const user = await api.getMe(authUrl, result.accessToken);
        this.currentUser = user;
        this.dismissPendingAuthActions();
        this.bus.emit('auth:state-changed', { user }, 'auth');
      } catch (err) {
        console.error('[AuthPlugin] auth:login-anonymous error:', err);
      }
    }, 'auth');

    this.bus.on('auth:verify', async ({ email, code, authUrl }) => {
      try {
        const token = await api.verifyCode(authUrl, email, code);
        await store.save({
          accessToken: token.accessToken,
          email,
          expiresAt: Date.now() + token.expiresIn * 1000,
          authUrl,
        });
        const user = await api.getMe(authUrl, token.accessToken);
        this.currentUser = user;
        this.dismissPendingAuthActions();
        this.bus.emit('auth:state-changed', { user }, 'auth');
      } catch (err) {
        console.error('[AuthPlugin] auth:verify error:', err);
      }
    }, 'auth');

    this.bus.on('auth:session-save', async (p: { accessToken: string; email: string; expiresAt: number; authUrl: string }) => {
      await store.save(p);
      this.dismissPendingAuthActions();
      this.bus.emit('auth:state-changed', { user: null }, 'auth');
    }, 'auth');

    this.bus.on('auth:logout', async ({ authUrl }) => {
      if (authUrl) {
        await store.clear(authUrl);
      } else {
        await store.clearAll();
      }
      this.currentUser = null;
      this.bus.emit('auth:state-changed', { user: null }, 'auth');
    }, 'auth');

    this.bus.on('auth:me', async () => {
      this.bus.emit('auth:state-changed', { user: this.currentUser }, 'auth');
    }, 'auth');

  }

  private async getToken(authDomain: string): Promise<string | null> {
    const session = await store.loadValid(authDomain);
    return session?.accessToken ?? null;
  }

  private async restoreSession(): Promise<void> {
    const domains = await store.listDomains();
    for (const domain of domains) {
      const session = await store.loadValid(domain);
      if (!session) continue;

      if (!session.email) {
        try {
          const user = await api.getMe(session.authUrl, session.accessToken);
          this.currentUser = user;
          this.dismissPendingAuthActions();
          this.bus.emit('auth:state-changed', { user }, 'auth');
          return;
        } catch {
          await store.clear(domain);
          continue;
        }
      }

      try {
        const user = await api.getMe(session.authUrl, session.accessToken);
        this.currentUser = user;
        this.dismissPendingAuthActions();
        this.bus.emit('auth:state-changed', { user }, 'auth');
        return;
      } catch {
        await store.clear(domain);
      }
    }
    this.bus.emit('auth:state-changed', { user: null }, 'auth');
  }
}
