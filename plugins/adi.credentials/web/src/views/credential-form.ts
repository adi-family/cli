import { html, nothing, type TemplateResult } from 'lit';
import type { Credential, CredentialType } from '../types.js';
import { ALL_TYPES, TYPE_LABELS } from './shared.js';

export interface CocoonOption {
  id: string;
  installed: boolean;
}

interface CredentialFormProps {
  cocoons: CocoonOption[];
  submitting: boolean;
  editing: Credential | null;
  onBack(): void;
  onCreate(data: {
    cocoonId: string;
    name: string;
    credential_type: CredentialType;
    data: Record<string, unknown>;
    description?: string;
    provider?: string;
    expires_at?: string;
  }): void;
  onUpdate(data: {
    cocoonId: string;
    id: string;
    name?: string;
    description?: string;
    data?: Record<string, unknown>;
    provider?: string;
    expires_at?: string;
  }): void;
}

const parseDataFields = (form: HTMLFormElement): Record<string, unknown> => {
  const keys = form.querySelectorAll<HTMLInputElement>('[name="data_key"]');
  const values = form.querySelectorAll<HTMLInputElement>('[name="data_value"]');
  const result: Record<string, unknown> = {};
  keys.forEach((keyEl, i) => {
    const k = keyEl.value.trim();
    if (k && values[i]) result[k] = values[i].value;
  });
  return result;
};

const dataFieldPair = () => html`
  <div class="flex gap-2 items-center data-pair">
    <input
      type="text"
      name="data_key"
      placeholder="Key"
      class="flex-1 px-3 py-2 rounded-lg bg-white/5 border border-white/10 text-sm text-gray-200 placeholder-gray-500 focus:outline-none focus:border-purple-500/50 font-mono"
    />
    <input
      type="text"
      name="data_value"
      placeholder="Value"
      class="flex-1 px-3 py-2 rounded-lg bg-white/5 border border-white/10 text-sm text-gray-200 placeholder-gray-500 focus:outline-none focus:border-purple-500/50 font-mono"
    />
  </div>
`;

export function renderCredentialForm(props: CredentialFormProps): TemplateResult {
  const { cocoons, submitting, editing, onBack, onCreate, onUpdate } = props;
  const isEdit = editing !== null;

  const handleSubmit = (e: Event) => {
    e.preventDefault();
    const form = e.target as HTMLFormElement;
    const fd = new FormData(form);

    const cocoonId = fd.get('cocoonId') as string;
    const name = (fd.get('name') as string ?? '').trim();
    const description = (fd.get('description') as string ?? '').trim();
    const provider = (fd.get('provider') as string ?? '').trim();
    const expiresAt = (fd.get('expires_at') as string ?? '').trim();
    const data = parseDataFields(form);

    if (isEdit) {
      onUpdate({
        cocoonId,
        id: editing.id,
        name: name || undefined,
        description: description || undefined,
        data: Object.keys(data).length > 0 ? data : undefined,
        provider: provider || undefined,
        expires_at: expiresAt || undefined,
      });
    } else {
      const credType = fd.get('credential_type') as CredentialType;
      if (!name || !credType || !cocoonId) return;
      onCreate({
        cocoonId,
        name,
        credential_type: credType,
        data,
        description: description || undefined,
        provider: provider || undefined,
        expires_at: expiresAt || undefined,
      });
    }
  };

  const addDataField = () => {
    const container = document.querySelector('#data-fields');
    if (!container) return;
    const div = document.createElement('div');
    div.innerHTML = `
      <div class="flex gap-2 items-center data-pair">
        <input type="text" name="data_key" placeholder="Key"
          class="flex-1 px-3 py-2 rounded-lg bg-white/5 border border-white/10 text-sm text-gray-200 placeholder-gray-500 focus:outline-none focus:border-purple-500/50 font-mono" />
        <input type="text" name="data_value" placeholder="Value"
          class="flex-1 px-3 py-2 rounded-lg bg-white/5 border border-white/10 text-sm text-gray-200 placeholder-gray-500 focus:outline-none focus:border-purple-500/50 font-mono" />
      </div>`;
    container.appendChild(div.firstElementChild!);
  };

  return html`
    <div class="space-y-3">
      <button class="text-sm text-gray-400 hover:text-gray-200 transition-colors" @click=${onBack}>
        &larr; Back
      </button>

      <div class="bg-white/5 rounded-xl p-4">
        <h2 class="text-lg font-semibold text-gray-200 mb-4">${isEdit ? 'Edit Credential' : 'New Credential'}</h2>

        <form @submit=${handleSubmit} class="space-y-4">
          <div>
            <label class="block text-xs text-gray-400 uppercase tracking-wider mb-1">Connection</label>
            <select
              name="cocoonId"
              required
              ?disabled=${submitting}
              class="w-full px-3 py-2 rounded-lg bg-white/5 border border-white/10 text-sm text-gray-200 focus:outline-none focus:border-purple-500/50 disabled:opacity-50"
            >
              ${cocoons.map((c: CocoonOption) => html`
                <option value=${c.id} ?selected=${isEdit && c.id === editing?.cocoonId}>
                  ${c.id} — ${c.installed ? 'already installed on cocoon' : 'will be installed on cocoon'}
                </option>
              `)}
            </select>
          </div>

          <div>
            <label class="block text-xs text-gray-400 uppercase tracking-wider mb-1">Name</label>
            <input
              type="text"
              name="name"
              ?required=${!isEdit}
              ?disabled=${submitting}
              .value=${editing?.name ?? ''}
              placeholder="e.g. Production API Key"
              class="w-full px-3 py-2 rounded-lg bg-white/5 border border-white/10 text-sm text-gray-200 placeholder-gray-500 focus:outline-none focus:border-purple-500/50 disabled:opacity-50"
            />
          </div>

          ${!isEdit ? html`
            <div>
              <label class="block text-xs text-gray-400 uppercase tracking-wider mb-1">Type</label>
              <select
                name="credential_type"
                required
                ?disabled=${submitting}
                class="w-full px-3 py-2 rounded-lg bg-white/5 border border-white/10 text-sm text-gray-200 focus:outline-none focus:border-purple-500/50 disabled:opacity-50"
              >
                ${ALL_TYPES.map(t => html`<option value=${t}>${TYPE_LABELS[t]}</option>`)}
              </select>
            </div>
          ` : nothing}

          <div>
            <label class="block text-xs text-gray-400 uppercase tracking-wider mb-1">Description</label>
            <textarea
              name="description"
              rows="2"
              ?disabled=${submitting}
              .value=${editing?.description ?? ''}
              placeholder="Optional description..."
              class="w-full px-3 py-2 rounded-lg bg-white/5 border border-white/10 text-sm text-gray-200 placeholder-gray-500 focus:outline-none focus:border-purple-500/50 resize-none disabled:opacity-50"
            ></textarea>
          </div>

          <div>
            <label class="block text-xs text-gray-400 uppercase tracking-wider mb-1">Provider</label>
            <input
              type="text"
              name="provider"
              ?disabled=${submitting}
              .value=${editing?.provider ?? ''}
              placeholder="e.g. github.com"
              class="w-full px-3 py-2 rounded-lg bg-white/5 border border-white/10 text-sm text-gray-200 placeholder-gray-500 focus:outline-none focus:border-purple-500/50 disabled:opacity-50"
            />
          </div>

          <div>
            <label class="block text-xs text-gray-400 uppercase tracking-wider mb-1">Expires at</label>
            <input
              type="datetime-local"
              name="expires_at"
              ?disabled=${submitting}
              class="w-full px-3 py-2 rounded-lg bg-white/5 border border-white/10 text-sm text-gray-200 focus:outline-none focus:border-purple-500/50 disabled:opacity-50"
            />
          </div>

          <div>
            <div class="flex items-center justify-between mb-2">
              <label class="text-xs text-gray-400 uppercase tracking-wider">Secret Data (key-value pairs)</label>
              <button
                type="button"
                class="text-xs text-purple-300 hover:text-purple-200 transition-colors"
                @click=${addDataField}
              >+ Add field</button>
            </div>
            <div id="data-fields" class="space-y-2">
              ${dataFieldPair()}
            </div>
          </div>

          <div class="flex gap-2">
            <button
              type="submit"
              ?disabled=${submitting}
              class="px-4 py-2 rounded-lg bg-purple-500/20 text-purple-200 hover:bg-purple-500/30 transition-colors text-sm font-medium disabled:opacity-50"
            >${submitting ? (isEdit ? 'Saving...' : 'Creating...') : (isEdit ? 'Save Changes' : 'Create Credential')}</button>
            <button
              type="button"
              ?disabled=${submitting}
              @click=${onBack}
              class="px-4 py-2 rounded-lg bg-white/5 text-gray-400 hover:bg-white/10 transition-colors text-sm disabled:opacity-50"
            >Cancel</button>
          </div>
        </form>
      </div>
    </div>
  `;
}
