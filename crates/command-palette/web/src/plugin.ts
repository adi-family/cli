import './styles.css';
import { LitElement, html, nothing } from 'lit';
import { customElement, state } from 'lit/decorators.js';
import { AdiPlugin } from '@adi-family/sdk-plugin';
import { CommandBusKey, CommandPaletteBusKey } from './bus';
import type { CommandRegisterEvent } from './bus';
import { SlotsBusKey } from '@adi/slots-web-plugin/bus';
import { PLUGIN_ID, PLUGIN_VERSION } from './config';

interface Command {
  id: string;
  label: string;
  shortcut?: string;
}

@customElement('adi-command-palette')
export class CommandPaletteElement extends LitElement {
  @state() commands: Command[] = [];
  @state() open = false;
  @state() query = '';
  @state() selectedIndex = 0;

  onExecute?: (cmd: Command) => void;
  onClose?: () => void;

  override createRenderRoot() {
    return this;
  }

  override connectedCallback(): void {
    super.connectedCallback();
    window.addEventListener('keydown', this.onGlobalKeyDown);
  }

  override disconnectedCallback(): void {
    super.disconnectedCallback();
    window.removeEventListener('keydown', this.onGlobalKeyDown);
  }

  override updated(changed: Map<string | number | symbol, unknown>): void {
    if (changed.has('open') && this.open) {
      requestAnimationFrame(() => {
        (this.querySelector('input') as HTMLInputElement | null)?.focus();
      });
    }
  }

  private readonly onGlobalKeyDown = (e: KeyboardEvent): void => {
    if ((e.metaKey || e.ctrlKey) && e.key === 'k') {
      e.preventDefault();
      if (this.open) this.close();
      else this.show();
      return;
    }

    if (!this.open) return;

    const filtered = this.filtered();
    switch (e.key) {
      case 'Escape':
        this.close();
        break;
      case 'ArrowDown':
        e.preventDefault();
        this.selectedIndex = Math.min(
          this.selectedIndex + 1,
          filtered.length - 1,
        );
        break;
      case 'ArrowUp':
        e.preventDefault();
        this.selectedIndex = Math.max(this.selectedIndex - 1, 0);
        break;
      case 'Enter':
        e.preventDefault();
        if (filtered[this.selectedIndex])
          this.execute(filtered[this.selectedIndex]);
        break;
    }
  };

  show(query?: string): void {
    this.open = true;
    this.query = query ?? '';
    this.selectedIndex = 0;
  }

  close(): void {
    this.open = false;
    this.onClose?.();
  }

  private filtered(): Command[] {
    if (!this.query.trim()) return this.commands;
    const q = this.query.toLowerCase();
    return this.commands.filter(
      (c) =>
        c.label.toLowerCase().includes(q) || c.id.toLowerCase().includes(q),
    );
  }

  private execute(cmd: Command): void {
    this.close();
    this.onExecute?.(cmd);
  }

  override render() {
    if (!this.open) return nothing;

    const filtered = this.filtered();

    return html`
      <div
        class="overlay-backdrop is-open"
        style="align-items: flex-start; padding-top: 15vh;"
        @click=${(e: Event) => {
          if (e.target === e.currentTarget) this.close();
        }}
      >
        <div
          class="overlay-panel rounded-2xl"
          style="width: 100%; max-width: 36rem; overflow: hidden;"
        >
          <!-- Search bar -->
          <div class="cp-search-bar px-4 py-3 gap-3">
            <svg
              class="size-4 cp-icon-muted"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path
                stroke-linecap="round"
                stroke-linejoin="round"
                stroke-width="2"
                d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"
              />
            </svg>
            <input
              type="text"
              placeholder="Search commands..."
              class="cp-search-input text-sm"
              .value=${this.query}
              @input=${(e: Event) => {
                this.query = (e.target as HTMLInputElement).value;
                this.selectedIndex = 0;
              }}
            />
            <kbd class="cp-kbd text-xs">ESC</kbd>
          </div>

          <!-- Command list -->
          <div class="cp-list py-1">
            ${filtered.length === 0
              ? html`<p class="px-4 py-8 text-sm cp-empty">
                  No commands found
                </p>`
              : filtered.map(
                  (cmd, i) => html`
                    <button
                      type="button"
                      class=${[
                        'cp-item px-4 py-2 gap-3 text-sm',
                        i === this.selectedIndex ? 'cp-item--active' : '',
                      ].join(' ')}
                      @click=${() => this.execute(cmd)}
                      @mouseenter=${() => {
                        this.selectedIndex = i;
                      }}
                    >
                      <span>${cmd.label}</span>
                      ${cmd.shortcut
                        ? html`<kbd class="cp-kbd text-xs"
                            >${cmd.shortcut}</kbd
                          >`
                        : nothing}
                    </button>
                  `,
                )}
          </div>

          <!-- Footer hints -->
          <div class="cp-footer px-4 py-2 gap-4 text-xs">
            <span>↑↓ navigate</span>
            <span>↵ select</span>
            <span>ESC close</span>
          </div>
        </div>
      </div>
    `;
  }
}

export class CommandPalettePlugin extends AdiPlugin {
  readonly id = PLUGIN_ID;
  readonly version = PLUGIN_VERSION;

  private el: CommandPaletteElement | null = null;
  get api() {
    return this;
  }

  override onRegister(): void {
    this.el = document.createElement('adi-command-palette') as CommandPaletteElement;
    this.el.onExecute = (cmd) => {
      this.bus.emit(CommandBusKey.Execute, { id: cmd.id }, PLUGIN_ID);
    };

    this.bus.emit(
      SlotsBusKey.Place,
      {
        slot: 'overlays',
        elementRef: this.el,
        priority: 0,
        pluginId: PLUGIN_ID,
      },
      PLUGIN_ID,
    );

    this.bus.on(
      CommandBusKey.Register,
      ({ id, label, shortcut }: CommandRegisterEvent) => {
        if (!this.el) return;
        if (this.el.commands.some((c: Command) => c.id === id)) return;
        this.el.commands = [...this.el.commands, { id, label, shortcut }];
      },
      PLUGIN_ID,
    );

    this.bus.on(
      CommandPaletteBusKey.Open,
      ({ query }) => {
        this.el?.show(query);
      },
      PLUGIN_ID,
    );
  }

  override onUnregister(): void {
    if (this.el) {
      this.bus.emit(
        SlotsBusKey.Remove,
        { slot: 'overlays', elementRef: this.el },
        PLUGIN_ID,
      );
      this.el = null;
    }
  }
}
