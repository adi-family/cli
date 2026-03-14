import { html, nothing, type TemplateResult } from 'lit';

export interface LoginFormProps {
  email: string;
  code: string;
  step: 'email' | 'code';
  loading: boolean;
  error: string | null;
  authUrl: string;
  inline: boolean;
  allowGuest: boolean;
  onEmailChange(email: string): void;
  onCodeChange(code: string): void;
  onAuthUrlChange(url: string): void;
  onRequestCode(): void;
  onVerify(): void;
  onBack(): void;
  onAnonymousLogin(): void;
}

export function renderLoginForm(props: LoginFormProps): TemplateResult {
  const {
    email, code, step, loading, error, authUrl, inline, allowGuest,
    onEmailChange, onCodeChange, onAuthUrlChange, onRequestCode, onVerify, onBack, onAnonymousLogin,
  } = props;

  const wrapper = inline ? '' : 'auth-container';

  return html`
    <div class="${wrapper} space-y-4">
      ${inline ? nothing : html`<h2 class="auth-heading text-lg">Sign in</h2>`}

      ${error ? html`<div class="auth-error text-xs px-2 py-1">${error}</div>` : nothing}

      ${step === 'email' ? html`
        <div class="space-y-3">
          <label class="auth-label">
            <span class="text-xs">Auth server URL</span>
            <input
              type="url"
              .value=${authUrl}
              @input=${(e: Event) => onAuthUrlChange((e.target as HTMLInputElement).value)}
              placeholder="https://auth.example.com"
              class="auth-input text-sm px-3 py-2 mt-1"
              ?disabled=${loading}
            />
          </label>
          <label class="auth-label">
            <span class="text-xs">Email address</span>
            <input
              type="email"
              .value=${email}
              @input=${(e: Event) => onEmailChange((e.target as HTMLInputElement).value)}
              @keydown=${(e: KeyboardEvent) => { if (e.key === 'Enter' && email && authUrl) onRequestCode(); }}
              placeholder="you@example.com"
              class="auth-input text-sm px-3 py-2 mt-1"
              ?disabled=${loading}
            />
          </label>
          <button
            class="auth-btn auth-btn-primary text-sm px-4 py-2"
            ?disabled=${loading || !email || !authUrl}
            @click=${onRequestCode}
          >${loading ? 'Sending...' : 'Send verification code'}</button>
          ${allowGuest ? html`
            <div class="auth-divider gap-2 my-2">
              <hr /><span class="text-xs">or</span><hr />
            </div>
            <button
              class="auth-btn auth-btn-ghost text-sm px-4 py-2"
              ?disabled=${loading || !authUrl}
              @click=${onAnonymousLogin}
            >${loading ? 'Connecting...' : 'Continue as guest'}</button>
          ` : nothing}
        </div>
      ` : html`
        <div class="space-y-3">
          <p class="auth-hint text-xs">Code sent to <strong>${email}</strong></p>
          <label class="auth-label">
            <span class="text-xs">Verification code</span>
            <input
              type="text"
              .value=${code}
              @input=${(e: Event) => onCodeChange((e.target as HTMLInputElement).value)}
              @keydown=${(e: KeyboardEvent) => { if (e.key === 'Enter' && code) onVerify(); }}
              placeholder="123456"
              class="auth-input auth-code-input text-sm px-3 py-2 mt-1"
              ?disabled=${loading}
              autocomplete="one-time-code"
            />
          </label>
          <div style="display:flex" class="gap-2">
            <button
              class="auth-btn auth-btn-secondary text-sm px-4 py-2"
              @click=${onBack}
              ?disabled=${loading}
            >Back</button>
            <button
              class="auth-btn auth-btn-verify text-sm px-4 py-2"
              ?disabled=${loading || !code}
              @click=${onVerify}
            >${loading ? 'Verifying...' : 'Verify'}</button>
          </div>
        </div>
      `}
    </div>
  `;
}
