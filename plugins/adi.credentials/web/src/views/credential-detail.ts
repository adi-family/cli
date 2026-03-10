import { html, nothing, type TemplateResult } from 'lit';
import type { Credential, CredentialWithData, CredentialAccessLog, VerifyResult } from '../types.js';
import { TYPE_COLORS, TYPE_LABELS, formatDate, isExpired } from './shared.js';

interface CredentialDetailProps {
  credential: Credential;
  revealedData: CredentialWithData | null;
  verifyResult: VerifyResult | null;
  accessLogs: CredentialAccessLog[];
  submitting: boolean;
  confirmingDelete: boolean;
  onBack(): void;
  onReveal(): void;
  onHide(): void;
  onVerify(): void;
  onLoadLogs(): void;
  onDelete(): void;
  onCancelDelete(): void;
  onEdit(): void;
}

const dataEntry = (key: string, value: unknown) => html`
  <div class="flex items-start gap-2 py-1">
    <span class="text-xs text-gray-500 font-mono shrink-0 min-w-[100px]">${key}</span>
    <span class="text-xs text-gray-300 font-mono break-all">${String(value)}</span>
  </div>
`;

const logRow = (log: CredentialAccessLog) => html`
  <div class="flex items-center gap-3 py-2 border-b border-white/5 last:border-0">
    <span class="text-xs font-medium px-2 py-0.5 rounded ${
      log.action === 'read' ? 'bg-blue-500/20 text-blue-300' :
      log.action === 'update' ? 'bg-yellow-500/20 text-yellow-300' :
      log.action === 'delete' ? 'bg-red-500/20 text-red-300' :
      'bg-gray-500/20 text-gray-300'
    }">${log.action}</span>
    <span class="text-xs text-gray-500 flex-1">${formatDate(log.created_at)}</span>
    ${log.ip_address ? html`<span class="text-xs text-gray-600 font-mono">${log.ip_address}</span>` : nothing}
  </div>
`;

export function renderCredentialDetail(props: CredentialDetailProps): TemplateResult {
  const { credential: cred, revealedData, verifyResult, accessLogs, submitting, confirmingDelete } = props;
  const expired = isExpired(cred.expires_at);

  return html`
    <div class="space-y-4">
      <button class="text-sm text-gray-400 hover:text-gray-200 transition-colors" @click=${props.onBack}>
        &larr; Back to list
      </button>

      <div class="bg-white/5 rounded-xl p-4 space-y-4">
        <div class="flex items-start justify-between gap-3">
          <div>
            <h2 class="text-lg font-semibold text-gray-100">${cred.name}</h2>
            ${cred.description
              ? html`<p class="text-sm text-gray-400 mt-1">${cred.description}</p>`
              : nothing}
          </div>
          <span class="inline-flex px-2 py-0.5 rounded text-xs font-medium shrink-0 ${TYPE_COLORS[cred.credential_type]}">
            ${TYPE_LABELS[cred.credential_type]}
          </span>
        </div>

        <div class="grid grid-cols-2 gap-3 text-xs">
          ${cred.provider ? html`
            <div>
              <span class="text-gray-500 uppercase tracking-wider">Provider</span>
              <div class="text-gray-300 mt-0.5">${cred.provider}</div>
            </div>
          ` : nothing}
          <div>
            <span class="text-gray-500 uppercase tracking-wider">Created</span>
            <div class="text-gray-300 mt-0.5">${formatDate(cred.created_at)}</div>
          </div>
          <div>
            <span class="text-gray-500 uppercase tracking-wider">Updated</span>
            <div class="text-gray-300 mt-0.5">${formatDate(cred.updated_at)}</div>
          </div>
          ${cred.last_used_at ? html`
            <div>
              <span class="text-gray-500 uppercase tracking-wider">Last used</span>
              <div class="text-gray-300 mt-0.5">${formatDate(cred.last_used_at)}</div>
            </div>
          ` : nothing}
          ${cred.expires_at ? html`
            <div>
              <span class="text-gray-500 uppercase tracking-wider">Expires</span>
              <div class="mt-0.5 ${expired ? 'text-red-400 font-medium' : 'text-gray-300'}">${formatDate(cred.expires_at)}${expired ? ' (expired)' : ''}</div>
            </div>
          ` : nothing}
        </div>

        <!-- Secret data reveal -->
        <div class="border-t border-white/10 pt-3">
          <div class="flex items-center justify-between mb-2">
            <h3 class="text-xs text-gray-500 uppercase tracking-wider">Secret Data</h3>
            ${revealedData
              ? html`<button class="text-xs text-gray-400 hover:text-gray-200 transition-colors" @click=${props.onHide}>Hide</button>`
              : html`<button class="text-xs text-purple-300 hover:text-purple-200 transition-colors" @click=${props.onReveal}>Reveal</button>`}
          </div>
          ${revealedData
            ? html`
              <div class="bg-black/30 rounded-lg p-3 space-y-1">
                ${Object.entries(revealedData.data).map(([k, v]) => dataEntry(k, v))}
                ${Object.keys(revealedData.data).length === 0
                  ? html`<span class="text-xs text-gray-600">No data</span>`
                  : nothing}
              </div>`
            : html`<div class="bg-black/30 rounded-lg p-3 text-xs text-gray-600">Click reveal to view secret data</div>`}
        </div>

        <!-- Verify -->
        <div class="border-t border-white/10 pt-3">
          <div class="flex items-center gap-3">
            <button
              class="px-3 py-1 rounded text-xs bg-cyan-500/20 text-cyan-300 hover:bg-cyan-500/30 transition-colors"
              ?disabled=${submitting}
              @click=${props.onVerify}
            >Verify</button>
            ${verifyResult ? html`
              <span class="text-xs ${verifyResult.valid ? 'text-green-400' : 'text-red-400'}">
                ${verifyResult.valid ? 'Valid' : 'Invalid'}${verifyResult.is_expired ? ' (expired)' : ''}
              </span>
            ` : nothing}
          </div>
        </div>

        <!-- Access logs -->
        <div class="border-t border-white/10 pt-3">
          <div class="flex items-center justify-between mb-2">
            <h3 class="text-xs text-gray-500 uppercase tracking-wider">Access Logs</h3>
            <button
              class="text-xs text-gray-400 hover:text-gray-200 transition-colors"
              @click=${props.onLoadLogs}
            >Refresh</button>
          </div>
          ${accessLogs.length > 0
            ? html`<div class="bg-black/20 rounded-lg p-2">${accessLogs.map(logRow)}</div>`
            : html`<div class="text-xs text-gray-600">No access logs yet. Click refresh to load.</div>`}
        </div>

        <!-- Actions -->
        <div class="border-t border-white/10 pt-3 flex items-center gap-2">
          <button
            class="px-3 py-1 rounded text-sm bg-purple-500/20 text-purple-300 hover:bg-purple-500/30 transition-colors"
            ?disabled=${submitting}
            @click=${props.onEdit}
          >Edit</button>

          ${confirmingDelete
            ? html`
              <span class="text-sm text-red-400">Delete this credential?</span>
              <button
                class="px-3 py-1 rounded text-sm bg-red-500/20 text-red-300 hover:bg-red-500/30 transition-colors"
                ?disabled=${submitting}
                @click=${props.onDelete}
              >Confirm</button>
              <button
                class="px-3 py-1 rounded text-sm bg-white/5 text-gray-400 hover:bg-white/10 transition-colors"
                @click=${props.onCancelDelete}
              >Cancel</button>
            `
            : html`
              <button
                class="px-3 py-1 rounded text-sm bg-red-500/10 text-red-400 hover:bg-red-500/20 transition-colors"
                ?disabled=${submitting}
                @click=${props.onDelete}
              >Delete</button>
            `}
        </div>
      </div>
    </div>
  `;
}
