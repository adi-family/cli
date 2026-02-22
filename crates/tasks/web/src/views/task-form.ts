import { html, type TemplateResult } from 'lit';
import type { Connection } from '../types.js';

interface TaskFormProps {
  connections: Connection[];
  submitting: boolean;
  onBack(): void;
  onCreate(data: { title: string; description?: string; cocoonId: string }): void;
}

export function renderTaskForm(props: TaskFormProps): TemplateResult {
  const { connections, submitting, onBack, onCreate } = props;

  let titleValue = '';
  let descValue = '';
  let cocoonIdValue = connections[0]?.id ?? '';

  const handleSubmit = (e: Event) => {
    e.preventDefault();
    const title = titleValue.trim();
    if (title && cocoonIdValue) {
      onCreate({ title, description: descValue.trim() || undefined, cocoonId: cocoonIdValue });
    }
  };

  return html`
    <div class="space-y-3">
      <button class="text-sm text-gray-400 hover:text-gray-200 transition-colors" @click=${onBack}>
        &larr; Back to list
      </button>

      <div class="bg-white/5 rounded-xl p-4">
        <h2 class="text-lg font-semibold text-gray-200 mb-4">New Task</h2>

        <form @submit=${handleSubmit} class="space-y-4">
          <div>
            <label class="block text-xs text-gray-400 uppercase tracking-wider mb-1">Connection</label>
            <select
              name="cocoonId"
              required
              ?disabled=${submitting}
              @change=${(e: Event) => { cocoonIdValue = (e.target as HTMLSelectElement).value; }}
              class="w-full px-3 py-2 rounded-lg bg-white/5 border border-white/10 text-sm text-gray-200 focus:outline-none focus:border-purple-500/50 disabled:opacity-50"
            >
              ${connections.map(c => html`<option value=${c.id}>${c.id}</option>`)}
            </select>
          </div>

          <div>
            <label class="block text-xs text-gray-400 uppercase tracking-wider mb-1">Title</label>
            <input
              type="text"
              required
              ?disabled=${submitting}
              @input=${(e: InputEvent) => { titleValue = (e.target as HTMLInputElement).value; }}
              placeholder="What needs to be done?"
              class="w-full px-3 py-2 rounded-lg bg-white/5 border border-white/10 text-sm text-gray-200 placeholder-gray-500 focus:outline-none focus:border-purple-500/50 disabled:opacity-50"
            />
          </div>

          <div>
            <label class="block text-xs text-gray-400 uppercase tracking-wider mb-1">Description</label>
            <textarea
              rows="3"
              ?disabled=${submitting}
              @input=${(e: InputEvent) => { descValue = (e.target as HTMLTextAreaElement).value; }}
              placeholder="Optional details..."
              class="w-full px-3 py-2 rounded-lg bg-white/5 border border-white/10 text-sm text-gray-200 placeholder-gray-500 focus:outline-none focus:border-purple-500/50 resize-none disabled:opacity-50"
            ></textarea>
          </div>

          <div class="flex gap-2">
            <button
              type="submit"
              ?disabled=${submitting}
              class="px-4 py-2 rounded-lg bg-purple-500/20 text-purple-200 hover:bg-purple-500/30 transition-colors text-sm font-medium disabled:opacity-50"
            >
              ${submitting ? 'Creating...' : 'Create Task'}
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
