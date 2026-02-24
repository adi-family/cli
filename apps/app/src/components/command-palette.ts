import { LitElement, html, nothing } from 'lit';
import { customElement, state } from 'lit/decorators.js';
import type { EventBus } from '@adi-family/sdk-plugin';

interface Command {
  id: string;
  label: string;
  shortcut?: string;
}

@customElement('app-command-palette')
export class AppCommandPalette extends LitElement {
  @state() private commands: Command[] = [];
  @state() private open = false;
  @state() private query = '';
  @state() private selectedIndex = 0;

  override createRenderRoot() { return this; }

  override connectedCallback(): void {
    super.connectedCallback();
    window.addEventListener('keydown', this.#onGlobalKeyDown);
    if ((window as { sdk?: unknown }).sdk) {
      this.#subscribe();
    } else {
      window.addEventListener('sdk-ready', () => this.#subscribe(), { once: true });
    }
  }

  override disconnectedCallback(): void {
    super.disconnectedCallback();
    window.removeEventListener('keydown', this.#onGlobalKeyDown);
  }

  override updated(changed: Map<string | number | symbol, unknown>): void {
    if (changed.has('open') && this.open) {
      // Focus search input on next frame after render
      requestAnimationFrame(() => {
        (this.querySelector('input') as HTMLInputElement | null)?.focus();
      });
    }
  }

  #subscribe(): void {
    const bus = window.sdk.bus as EventBus;

    bus.on('command:register', ({ id, label, shortcut }) => {
      if (!this.commands.find(c => c.id === id)) {
        this.commands = [...this.commands, { id, label, shortcut }];
      }
    });

    bus.on('command-palette:open', ({ query }) => {
      this.open = true;
      this.query = query ?? '';
      this.selectedIndex = 0;
    });
  }

  readonly #onGlobalKeyDown = (e: KeyboardEvent): void => {
    if ((e.metaKey || e.ctrlKey) && e.key === 'k') {
      e.preventDefault();
      if (this.open) {
        this.open = false;
      } else {
        this.open = true;
        this.query = '';
        this.selectedIndex = 0;
      }
      return;
    }

    if (!this.open) return;

    const filtered = this.#filtered();
    switch (e.key) {
      case 'Escape':
        this.open = false;
        break;
      case 'ArrowDown':
        e.preventDefault();
        this.selectedIndex = Math.min(this.selectedIndex + 1, filtered.length - 1);
        break;
      case 'ArrowUp':
        e.preventDefault();
        this.selectedIndex = Math.max(this.selectedIndex - 1, 0);
        break;
      case 'Enter':
        e.preventDefault();
        if (filtered[this.selectedIndex]) this.#execute(filtered[this.selectedIndex]);
        break;
    }
  };

  #filtered(): Command[] {
    if (!this.query.trim()) return this.commands;
    const q = this.query.toLowerCase();
    return this.commands.filter(c =>
      c.label.toLowerCase().includes(q) || c.id.toLowerCase().includes(q)
    );
  }

  #execute(cmd: Command): void {
    this.open = false;
    if ((window as { sdk?: unknown }).sdk) {
      window.sdk.bus.emit('command:execute', { id: cmd.id });
    }
  }

  override render() {
    if (!this.open) return nothing;

    const filtered = this.#filtered();

    return html`
      <div
        class="fixed inset-0 z-50 flex items-start justify-center pt-[15vh]"
        @click=${(e: Event) => { if (e.target === e.currentTarget) this.open = false; }}
      >
        <div class="absolute inset-0 bg-black/50 backdrop-blur-sm" @click=${() => { this.open = false; }}></div>
        <div class="relative w-full max-w-xl mx-4 bg-surface border border-border rounded-xl shadow-2xl overflow-hidden">

          <!-- Search bar -->
          <div class="flex items-center gap-3 px-4 py-3 border-b border-border">
            <svg class="w-4 h-4 text-text-muted shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"/>
            </svg>
            <input
              type="text"
              placeholder="Search commands..."
              class="flex-1 bg-transparent text-text placeholder:text-text-muted text-sm outline-none"
              .value=${this.query}
              @input=${(e: Event) => {
                this.query = (e.target as HTMLInputElement).value;
                this.selectedIndex = 0;
              }}
            />
            <kbd class="text-xs text-text-muted bg-surface-alt px-1.5 py-0.5 rounded border border-border font-mono">ESC</kbd>
          </div>

          <!-- Command list -->
          <div class="max-h-80 overflow-y-auto py-1">
            ${filtered.length === 0
              ? html`<p class="px-4 py-8 text-center text-text-muted text-sm">No commands found</p>`
              : filtered.map((cmd, i) => html`
                <button
                  type="button"
                  class=${[
                    'w-full flex items-center justify-between gap-3 px-4 py-2.5 text-left text-sm transition-colors cursor-pointer',
                    i === this.selectedIndex
                      ? 'bg-accent/15 text-accent'
                      : 'text-text hover:bg-surface-alt',
                  ].join(' ')}
                  @click=${() => this.#execute(cmd)}
                  @mouseenter=${() => { this.selectedIndex = i; }}
                >
                  <span>${cmd.label}</span>
                  ${cmd.shortcut
                    ? html`<kbd class="text-xs text-text-muted bg-surface-alt px-1.5 py-0.5 rounded border border-border font-mono shrink-0">${cmd.shortcut}</kbd>`
                    : nothing}
                </button>
              `)
            }
          </div>

          <!-- Footer hints -->
          <div class="px-4 py-2 border-t border-border flex items-center gap-4 text-xs text-text-muted">
            <span>↑↓ navigate</span>
            <span>↵ select</span>
            <span>ESC close</span>
          </div>

        </div>
      </div>
    `;
  }
}
