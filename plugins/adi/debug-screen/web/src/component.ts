import './styles.css';
import { LitElement, html } from 'lit';
import { customElement, state } from 'lit/decorators.js';

export interface DebugSection {
  pluginId: string;
  init: () => HTMLElement;
  label: string;
}

@customElement('adi-debug-page')
export class AdiDebugPageElement extends LitElement {
  @state() sections: DebugSection[] = [];
  @state() activePluginId = '';

  private readonly elementCache = new Map<string, HTMLElement>();

  onNavigate?: (pluginId: string) => void;

  override createRenderRoot() {
    return this;
  }

  override render() {
    const active = this.sections.find((s) => s.pluginId === this.activePluginId);

    return html`
      <div class="dp-layout">
        <nav class="dp-sidebar">
          <div class="dp-sidebar-title text-xs uppercase">Debug Sections</div>
          ${this.sections.map(
            (s) => html`
              <button
                type="button"
                class=${[
                  'dp-sidebar-item text-sm',
                  s.pluginId === this.activePluginId ? 'dp-sidebar-item--active' : '',
                ].join(' ')}
                @click=${() => this.onNavigate?.(s.pluginId)}
              >
                ${s.label}
              </button>
            `,
          )}
        </nav>
        <main class="dp-content">
          ${active
            ? this.renderSection(active)
            : this.renderEmpty()}
        </main>
      </div>
    `;
  }

  private renderSection(section: DebugSection) {
    let el = this.elementCache.get(section.pluginId);
    if (!el) {
      el = section.init();
      this.elementCache.set(section.pluginId, el);
    }
    return html`${el}`;
  }

  private renderEmpty() {
    if (this.sections.length === 0) {
      return html`<div class="dp-empty text-sm">No debug sections registered.</div>`;
    }
    return html`<div class="dp-empty text-sm">Select a section from the sidebar.</div>`;
  }
}
