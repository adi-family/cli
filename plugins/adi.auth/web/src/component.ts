import { LitElement } from 'lit';
import { state } from 'lit/decorators.js';
import type { UserInfo } from './types.js';
import { getBus } from './context.js';
import { renderLoginForm } from './views/login-form.js';
import { renderProfile } from './views/profile.js';
import './styles.css';

type View = 'login' | 'verify' | 'profile';

export class AdiAuthElement extends LitElement {
  @state() private user: UserInfo | null = null;
  @state() private loading = false;
  @state() private error: string | null = null;
  @state() private view: View = 'login';
  @state() private email = '';
  @state() private code = '';
  @state() private authUrl = '';

  private unsubStateChanged: (() => void) | null = null;

  /** Compact mode when embedded in action cards */
  private get inline(): boolean {
    return this.hasAttribute('data-inline');
  }

  /** Whether authentication is required (hides guest login) */
  private get authRequired(): boolean {
    return this.hasAttribute('data-auth-required');
  }

  override createRenderRoot() { return this; }

  private get bus() { return getBus(); }

  override connectedCallback(): void {
    super.connectedCallback();

    // Pre-fill auth URL from attribute (set by action renderer)
    const attrUrl = this.getAttribute('data-auth-url');
    if (attrUrl) this.authUrl = attrUrl;

    this.unsubStateChanged = this.bus.on('auth:state-changed', ({ user }) => {
      this.user = user;
      this.view = user ? 'profile' : 'login';
    }, 'auth-ui');
    this.bus.emit('auth:me', {}, 'auth-ui');
  }

  override disconnectedCallback(): void {
    super.disconnectedCallback();
    this.unsubStateChanged?.();
    this.unsubStateChanged = null;
  }

  private async handleRequestCode(): Promise<void> {
    this.loading = true;
    this.error = null;
    try {
      this.bus.emit('auth:login', { email: this.email, authUrl: this.authUrl }, 'auth-ui');
      this.view = 'verify';
    } catch (err) {
      this.error = err instanceof Error ? err.message : 'Failed to send code';
    } finally {
      this.loading = false;
    }
  }

  private async handleVerify(): Promise<void> {
    this.loading = true;
    this.error = null;
    try {
      this.bus.emit('auth:verify', {
        email: this.email,
        code: this.code.trim(),
        authUrl: this.authUrl,
      }, 'auth-ui');
      this.code = '';
    } catch (err) {
      this.error = err instanceof Error ? err.message : 'Verification failed';
    } finally {
      this.loading = false;
    }
  }

  private async handleAnonymousLogin(): Promise<void> {
    this.loading = true;
    this.error = null;
    try {
      this.bus.emit('auth:login-anonymous', { authUrl: this.authUrl }, 'auth-ui');
    } catch (err) {
      this.error = err instanceof Error ? err.message : 'Anonymous login failed';
    } finally {
      this.loading = false;
    }
  }

  private async handleLogout(): Promise<void> {
    this.loading = true;
    try {
      this.bus.emit('auth:logout', {}, 'auth-ui');
      this.user = null;
      this.view = 'login';
      this.email = '';
      this.code = '';
    } finally {
      this.loading = false;
    }
  }

  override render() {
    if (this.view === 'profile' && this.user) {
      return renderProfile({
        user: this.user,
        loading: this.loading,
        inline: this.inline,
        onLogout: () => this.handleLogout(),
      });
    }

    return renderLoginForm({
      email: this.email,
      code: this.code,
      step: this.view === 'verify' ? 'code' : 'email',
      loading: this.loading,
      error: this.error,
      authUrl: this.authUrl,
      inline: this.inline,
      allowGuest: !this.authRequired,
      onEmailChange: (v) => { this.email = v; },
      onCodeChange: (v) => { this.code = v; },
      onAuthUrlChange: (v) => { this.authUrl = v; },
      onRequestCode: () => this.handleRequestCode(),
      onVerify: () => this.handleVerify(),
      onBack: () => { this.view = 'login'; this.code = ''; this.error = null; },
      onAnonymousLogin: () => this.handleAnonymousLogin(),
    });
  }
}
