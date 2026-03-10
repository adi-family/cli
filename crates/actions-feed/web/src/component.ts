import { LitElement, html } from 'lit';
import { state } from 'lit/decorators.js';
import { unsafeHTML } from 'lit/directives/unsafe-html.js';
import type { ActionCard } from './types.js';
import { actionStore } from './plugin.js';

export class AdiActionsFeedElement extends LitElement {
  @state() private actions: ActionCard[] = [];

  private unsub: (() => void) | null = null;

  override createRenderRoot() {
    return this;
  }

  override connectedCallback(): void {
    super.connectedCallback();
    this.actions = actionStore.actions;
    const listener = () => { this.actions = actionStore.actions; };
    actionStore.listeners.add(listener);
    this.unsub = () => actionStore.listeners.delete(listener);
  }

  override disconnectedCallback(): void {
    super.disconnectedCallback();
    this.unsub?.();
    this.unsub = null;
  }

  #dismiss(id: string): void {
    actionStore.bus!.emit('adi.actions-feed:dismiss', { id }, 'actions-feed');
  }

  #renderCard(card: ActionCard) {
    const renderer = actionStore.renderers.get(`${card.plugin}::${card.kind}`);
    const borderColor = card.priority === 'urgent' ? 'border-red-500/60' : 'border-border';

    const body = renderer
      ? unsafeHTML(renderer(card.data, card.id))
      : html`
          <div class="text-xs text-text-muted">
            <span class="font-medium text-text">${card.kind}</span>
            <span class="ml-1 opacity-60">(${card.plugin})</span>
          </div>
        `;

    return html`
      <div class="relative bg-surface border ${borderColor} rounded-lg p-3">
        <button
          type="button"
          class="absolute top-1.5 right-1.5 w-5 h-5 flex items-center justify-center rounded text-text-muted hover:text-text hover:bg-surface-alt transition-colors text-xs"
          @click=${() => this.#dismiss(card.id)}
          aria-label="Dismiss"
        >&times;</button>
        ${body}
      </div>
    `;
  }

  override render() {
    const sorted = [...this.actions].sort((a, b) => {
      const order = { urgent: 0, normal: 1, low: 2 };
      return order[a.priority] - order[b.priority];
    });

    return html`
      <div class="h-full bg-bg p-4 space-y-3 overflow-y-auto border-l border-border w-72">
        <div class="mb-1">
          <h2 class="text-sm font-semibold text-text">Actions</h2>
          <p class="text-xs text-text-muted">
            ${this.actions.length} pending
          </p>
        </div>

        ${sorted.length > 0
          ? html`<div class="flex flex-col gap-2">${sorted.map((card) => this.#renderCard(card))}</div>`
          : html`<div class="flex items-center justify-center py-12 text-text-muted text-xs">No pending actions.</div>`}
      </div>
    `;
  }
}
