import { html, nothing, type TemplateResult } from 'lit';
import type { CocoonPluginInterface, CocoonSelectEvent } from '@adi-family/cocoon-plugin-interface';
import type { CredentialType } from '../types.js';
import type { CredentialWithCocoon } from '../generated/models.js';
import { ALL_TYPES, TYPE_LABELS } from './shared.js';

export interface DataField {
  key: string;
  value: string;
}

interface CredentialFormProps {
  cocoonInterface: CocoonPluginInterface;
  selectedCocoonId: string;
  submitting: boolean;
  editing: CredentialWithCocoon | null;
  dataFields: DataField[];
  onBack(): void;
  onCocoonSelected(e: CustomEvent<CocoonSelectEvent>): void;
  onAddDataField(): void;
  onDataFieldChange(index: number, field: 'key' | 'value', val: string): void;
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

const collectDataFields = (fields: DataField[]): Record<string, unknown> => {
  const result: Record<string, unknown> = {};
  for (const { key, value } of fields) {
    const k = key.trim();
    if (k) result[k] = value;
  }
  return result;
};

const INPUT_CLASS = 'flex-1 px-3 py-2 rounded-lg bg-white/5 border border-white/10 text-sm text-gray-200 placeholder-gray-500 focus:outline-none focus:border-purple-500/50 font-mono';

const dataFieldPair = (
  field: DataField,
  index: number,
  onChange: (index: number, field: 'key' | 'value', val: string) => void,
) => html`
  <div class="flex gap-2 items-center data-pair">
    <input
      type="text"
      placeholder="Key"
      .value=${field.key}
      @input=${(e: InputEvent) => onChange(index, 'key', (e.target as HTMLInputElement).value)}
      class=${INPUT_CLASS}
    />
    <input
      type="text"
      placeholder="Value"
      .value=${field.value}
      @input=${(e: InputEvent) => onChange(index, 'value', (e.target as HTMLInputElement).value)}
      class=${INPUT_CLASS}
    />
  </div>
`;

export function renderCredentialForm(props: CredentialFormProps): TemplateResult {
  const { cocoonInterface, selectedCocoonId, submitting, editing, dataFields, onBack, onCocoonSelected, onAddDataField, onDataFieldChange, onCreate, onUpdate } = props;
  const isEdit = editing !== null;
  const cocoonId = selectedCocoonId || editing?.cocoonId || '';

  const handleSubmit = (e: Event) => {
    e.preventDefault();
    const form = e.target as HTMLFormElement;
    const fd = new FormData(form);

    const name = (fd.get('name') as string ?? '').trim();
    const description = (fd.get('description') as string ?? '').trim();
    const provider = (fd.get('provider') as string ?? '').trim();
    const expiresAt = (fd.get('expires_at') as string ?? '').trim();
    const data = collectDataFields(dataFields);

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
            <cocoon-select
              with-plugin="adi.credentials"
              .cocoonInterface=${cocoonInterface}
              .value=${cocoonId}
              label="Select cocoon..."
              @cocoon-selected=${onCocoonSelected}
            ></cocoon-select>
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
                @click=${onAddDataField}
              >+ Add field</button>
            </div>
            <div class="space-y-2">
              ${dataFields.map((f, i) => dataFieldPair(f, i, onDataFieldChange))}
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
