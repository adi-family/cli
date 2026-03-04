import { LitElement, html } from 'lit';
import { state } from 'lit/decorators.js';
import { unsafeHTML } from 'lit/directives/unsafe-html.js';
import type { ActionCard } from './types.js';
import { actionStore } from './plugin.js';

export class AdiActionsElement extends LitElement {
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
    window.sdk.bus.emit('actions:dismiss', { id }, 'actions-loop');
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
      <div class="min-h-screen bg-bg p-6 space-y-1">
        <div class="mb-2">
          <h1 class="text-xl font-semibold text-text">Actions</h1>
          <p class="text-sm text-text-muted">
            ${this.actions.length} pending action${this.actions.length !== 1 ? 's' : ''}
          </p>
        </div>

        ${sorted.length > 0
          ? html`<div class="flex flex-col gap-2">${sorted.map((card) => this.#renderCard(card))}</div>`
          : html`<div class="flex items-center justify-center py-24 text-text-muted text-sm">No pending actions.</div>`}
      </div>
    `;
  }
}
