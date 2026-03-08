import { html, type TemplateResult } from 'lit';
import type { Connection } from '@adi-family/cocoon-plugin-interface';

interface NodeFormProps {
  connections: Connection[];
  submitting: boolean;
  onBack(): void;
  onCreate(data: { user_said: string; derived_knowledge: string; node_type?: string; cocoonId: string }): void;
}

export function renderNodeForm(props: NodeFormProps): TemplateResult {
  const { connections, submitting, onBack, onCreate } = props;

  const handleSubmit = (e: Event) => {
    e.preventDefault();
    const form = e.target as HTMLFormElement;
    const data = new FormData(form);
    const user_said = (data.get('user_said') as string ?? '').trim();
    const derived_knowledge = (data.get('derived_knowledge') as string ?? '').trim();
    const node_type = data.get('node_type') as string;
    const cocoonId = data.get('cocoonId') as string;
    if (user_said && derived_knowledge && cocoonId) {
      onCreate({ user_said, derived_knowledge, node_type: node_type || undefined, cocoonId });
    }
  };

  return html`
    <div class="space-y-3">
      <button class="text-sm text-gray-400 hover:text-gray-200 transition-colors" @click=${onBack}>
        &larr; Back to search
      </button>

      <div class="bg-white/5 rounded-xl p-4">
        <h2 class="text-lg font-semibold text-gray-200 mb-4">Add Knowledge</h2>

        <form @submit=${handleSubmit} class="space-y-4">
          <div>
            <label class="block text-xs text-gray-400 uppercase tracking-wider mb-1">Connection</label>
            <select
              name="cocoonId"
              required
              ?disabled=${submitting}
              class="w-full px-3 py-2 rounded-lg bg-white/5 border border-white/10 text-sm text-gray-200 focus:outline-none focus:border-purple-500/50 disabled:opacity-50"
            >
              ${connections.map(c => html`<option value=${c.id}>${c.id}</option>`)}
            </select>
          </div>

          <div>
            <label class="block text-xs text-gray-400 uppercase tracking-wider mb-1">User said</label>
            <input
              type="text"
              name="user_said"
              required
              ?disabled=${submitting}
              placeholder="What the user originally stated..."
              class="w-full px-3 py-2 rounded-lg bg-white/5 border border-white/10 text-sm text-gray-200 placeholder-gray-500 focus:outline-none focus:border-purple-500/50 disabled:opacity-50"
            />
          </div>

          <div>
            <label class="block text-xs text-gray-400 uppercase tracking-wider mb-1">Derived knowledge</label>
            <textarea
              name="derived_knowledge"
              rows="4"
              required
              ?disabled=${submitting}
              placeholder="The knowledge derived from the statement..."
              class="w-full px-3 py-2 rounded-lg bg-white/5 border border-white/10 text-sm text-gray-200 placeholder-gray-500 focus:outline-none focus:border-purple-500/50 resize-none disabled:opacity-50"
            ></textarea>
          </div>

          <div>
            <label class="block text-xs text-gray-400 uppercase tracking-wider mb-1">Node type</label>
            <select
              name="node_type"
              ?disabled=${submitting}
              class="w-full px-3 py-2 rounded-lg bg-white/5 border border-white/10 text-sm text-gray-200 focus:outline-none focus:border-purple-500/50 disabled:opacity-50"
            >
              <option value="fact">Fact</option>
              <option value="decision">Decision</option>
              <option value="guide">Guide</option>
              <option value="error">Error</option>
              <option value="glossary">Glossary</option>
              <option value="context">Context</option>
              <option value="assumption">Assumption</option>
            </select>
          </div>

          <div class="flex gap-2">
            <button
              type="submit"
              ?disabled=${submitting}
              class="px-4 py-2 rounded-lg bg-purple-500/20 text-purple-200 hover:bg-purple-500/30 transition-colors text-sm font-medium disabled:opacity-50"
            >
              ${submitting ? 'Adding...' : 'Add Knowledge'}
            </button>
            <button
              type="button"
              ?disabled=${submitting}
              @click=${onBack}
              class="px-4 py-2 rounded-lg bg-white/5 text-gray-400 hover:bg-white/10 transition-colors text-sm disabled:opacity-50"
            >
              Cancel
            </button>
          </div>
        </form>
      </div>
    </div>
  `;
}
