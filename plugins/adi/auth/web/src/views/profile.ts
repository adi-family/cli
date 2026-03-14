import { html, nothing, type TemplateResult } from 'lit';
import type { UserInfo } from '../types.js';

export interface ProfileProps {
  user: UserInfo;
  loading: boolean;
  inline: boolean;
  onLogout(): void;
}

export function renderProfile(props: ProfileProps): TemplateResult {
  const { user, loading, inline, onLogout } = props;

  if (inline) {
    return html`
      <div style="display:flex;align-items:center" class="gap-2">
        <span class="auth-inline-status text-xs">Signed in as</span>
        <span class="auth-profile-email text-xs">${user.email}</span>
        <button
          style="margin-inline-start:auto"
          class="auth-btn auth-btn-danger text-xs px-2 py-1"
          ?disabled=${loading}
          @click=${onLogout}
        >Log out</button>
      </div>
    `;
  }

  return html`
    <div class="auth-container space-y-4">
      <h2 class="auth-heading text-lg">Profile</h2>

      <div class="auth-profile-card px-4 py-4 space-y-2">
        <div style="display:flex;align-items:center" class="gap-3">
          <div class="auth-avatar size-12 text-lg">
            ${user.email.charAt(0).toUpperCase()}
          </div>
          <div>
            <div class="auth-profile-email text-sm">${user.email}</div>
            <div class="auth-profile-id text-xs">${user.id}</div>
          </div>
        </div>

        <div class="auth-profile-meta py-2 space-y-1">
          ${user.isAdmin ? html`<div class="text-xs"><span class="auth-profile-meta-label">Role:</span> <span class="auth-profile-meta-accent">Admin</span></div>` : nothing}
          <div class="text-xs"><span class="auth-profile-meta-label">Joined:</span> <span class="auth-profile-meta-value">${user.createdAt}</span></div>
          ${user.lastLoginAt ? html`<div class="text-xs"><span class="auth-profile-meta-label">Last login:</span> <span class="auth-profile-meta-value">${user.lastLoginAt}</span></div>` : nothing}
        </div>
      </div>

      <button
        class="auth-btn auth-btn-danger text-sm px-4 py-2"
        ?disabled=${loading}
        @click=${onLogout}
      >${loading ? 'Logging out...' : 'Log out'}</button>
    </div>
  `;
}
