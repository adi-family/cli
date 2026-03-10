import { html, nothing, type TemplateResult } from 'lit';
import type { Credential, CredentialType } from '../types.js';
import { TYPE_COLORS, TYPE_LABELS, ALL_TYPES, timeAgo, isExpired } from './shared.js';

interface CredentialListProps {
  credentials: Credential[];
  filter: CredentialType | undefined;
  searchQuery: string;
  loading: boolean;
  error: string | null;
  onSelect(credential: Credential): void;
  onFilterChange(type: CredentialType | undefined): void;
  onSearch(query: string): void;
  onNew(): void;
}

const filterTabs = (current: CredentialType | undefined, onChange: (t: CredentialType | undefined) => void) => {
  const tabs: Array<{ label: string; value: CredentialType | undefined }> = [
    { label: 'All', value: undefined },
    ...ALL_TYPES.map(t => ({ label: TYPE_LABELS[t], value: t as CredentialType | undefined })),
  ];

  return html`
    <div class="flex gap-1 mb-4 flex-wrap">
      ${tabs.map(t => html`
        <button
          class="px-3 py-1 rounded-full text-sm transition-colors ${
            current === t.value
              ? 'bg-purple-500/30 text-purple-200 font-medium'
              : 'bg-white/5 text-gray-400 hover:bg-white/10 hover:text-gray-200'
          }"
          @click=${() => onChange(t.value)}
        >${t.label}</button>
      `)}
    </div>
  `;
};

const credentialRow = (cred: Credential, onSelect: (c: Credential) => void) => {
  const expired = isExpired(cred.expires_at);

  return html`
    <button
      class="w-full text-left p-3 rounded-lg bg-white/5 hover:bg-white/10 transition-colors flex items-start gap-3 group"
      @click=${() => onSelect(cred)}
    >
      <span class="inline-flex px-2 py-0.5 rounded text-xs font-medium shrink-0 mt-0.5 ${TYPE_COLORS[cred.credential_type]}">
        ${TYPE_LABELS[cred.credential_type]}
      </span>
      <div class="flex-1 min-w-0">
        <div class="text-sm text-gray-200 group-hover:text-white truncate">${cred.name}</div>
        ${cred.description
          ? html`<div class="text-xs text-gray-500 mt-0.5 truncate">${cred.description}</div>`
          : nothing}
        ${cred.provider
          ? html`<div class="text-xs text-gray-600 mt-0.5">${cred.provider}</div>`
          : nothing}
      </div>
      <div class="flex flex-col items-end gap-1 shrink-0">
        <span class="text-xs text-gray-600">${timeAgo(cred.updated_at)}</span>
        ${expired
          ? html`<span class="text-xs text-red-400 font-medium">Expired</span>`
          : nothing}
      </div>
    </button>
  `;
};

const renderListContent = (filtered: Credential[], props: CredentialListProps): TemplateResult => {
  if (props.error)
    return html`<div class="p-3 rounded-lg bg-red-500/10 border border-red-500/20 text-sm text-red-300">${props.error}</div>`;
  if (props.loading)
    return html`<div class="text-center py-8 text-gray-500 text-sm">Loading...</div>`;
  if (filtered.length === 0)
    return html`<div class="text-center py-8 text-gray-500 text-sm">No credentials found</div>`;
  return html`<div class="space-y-2">${filtered.map(c => credentialRow(c, props.onSelect))}</div>`;
};

export function renderCredentialList(props: CredentialListProps): TemplateResult {
  const query = props.searchQuery.trim().toLowerCase();
  const filtered = query
    ? props.credentials.filter(c =>
        c.name.toLowerCase().includes(query) ||
        (c.description?.toLowerCase().includes(query) ?? false) ||
        (c.provider?.toLowerCase().includes(query) ?? false)
      )
    : props.credentials;

  return html`
    <div class="space-y-3">
      <div class="flex items-center justify-between mb-2">
        <h2 class="text-lg font-semibold text-gray-200">Credentials</h2>
        <button
          class="px-3 py-1.5 rounded-lg bg-purple-500/20 text-purple-200 hover:bg-purple-500/30 transition-colors text-sm font-medium"
          @click=${props.onNew}
        >+ New Credential</button>
      </div>

      ${filterTabs(props.filter, props.onFilterChange)}

      <div class="relative mb-3">
        <input
          type="text"
          placeholder="Search credentials..."
          .value=${props.searchQuery}
          @input=${(e: InputEvent) => props.onSearch((e.target as HTMLInputElement).value)}
          class="w-full px-3 py-2 rounded-lg bg-white/5 border border-white/10 text-sm text-gray-200 placeholder-gray-500 focus:outline-none focus:border-purple-500/50"
        />
      </div>

      ${renderListContent(filtered, props)}
    </div>
  `;
}
