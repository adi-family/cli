import { LitElement, html, nothing } from 'lit';
import { customElement, state } from 'lit/decorators.js';
import { AdiPlugin } from '@adi-family/sdk-plugin';
import { getEnabledWebPluginIds, setEnabledWebPluginIds } from '../plugin-prefs.ts';

interface PluginEntry {
  id: string;
  installedVersion: string;
  pluginTypes?: string[];
}

interface DebugInfo {
  loaded?: string[];
  failed?: string[];
  timedOut?: string[];
}

@customElement('app-plugins-page')
export class AppPluginsPage extends LitElement {
  override createRenderRoot() { return this; }

  @state() private plugins: PluginEntry[] = [];
  @state() private loadedIds = new Set<string>();
  @state() private failedIds = new Set<string>();
  @state() private timedOutIds = new Set<string>();
  @state() private enabledIds: Set<string> = new Set();
  @state() private dirty = false;

  private unsub: (() => void) | null = null;

  override connectedCallback(): void {
    super.connectedCallback();
    this.#load();
    this.unsub = window.sdk.bus.on('loading-finished', () => this.#load(), 'plugins-page');
  }

  override disconnectedCallback(): void {
    super.disconnectedCallback();
    this.unsub?.();
    this.unsub = null;
  }

  #load(): void {
    const w = window as unknown as Record<string, unknown>;
    const all = w['__adiAllPlugins'];
    if (Array.isArray(all)) this.plugins = all as PluginEntry[];

    const debug = w['__adiDebug'] as DebugInfo | undefined;
    if (debug) {
      this.loadedIds = new Set(debug.loaded ?? []);
      this.failedIds = new Set(debug.failed ?? []);
      this.timedOutIds = new Set(debug.timedOut ?? []);
    }

    this.enabledIds = getEnabledWebPluginIds() ?? new Set();
  }

  #toggle(id: string, enabled: boolean): void {
    const next = new Set(this.enabledIds);
    if (enabled) next.add(id); else next.delete(id);
    this.enabledIds = next;
    setEnabledWebPluginIds(next);
    this.dirty = true;
  }

  #statusBadge(id: string) {
    if (this.loadedIds.has(id))   return html`<span class="text-[10px] font-medium text-green-400 bg-green-400/10 px-1.5 py-0.5 rounded">loaded</span>`;
    if (this.failedIds.has(id))   return html`<span class="text-[10px] font-medium text-red-400 bg-red-400/10 px-1.5 py-0.5 rounded">failed</span>`;
    if (this.timedOutIds.has(id)) return html`<span class="text-[10px] font-medium text-yellow-400 bg-yellow-400/10 px-1.5 py-0.5 rounded">timed out</span>`;
    return nothing;
  }

  #renderPlugin(p: PluginEntry, toggleable: boolean) {
    const enabled = this.enabledIds.has(p.id);
    return html`
      <div class="flex items-center gap-3 px-3 py-2.5 bg-surface-alt rounded border border-border">
        ${toggleable ? html`
          <input
            type="checkbox"
            class="shrink-0 accent-accent w-3.5 h-3.5 cursor-pointer"
            .checked=${enabled}
            @change=${(e: Event) => this.#toggle(p.id, (e.target as HTMLInputElement).checked)}
          />
        ` : html`<span class="shrink-0 w-3.5"></span>`}
        <code class="text-sm font-mono text-text flex-1 truncate">${p.id}</code>
        <span class="text-xs text-text-muted font-mono shrink-0">v${p.installedVersion}</span>
        <div class="flex items-center gap-1 shrink-0">
          ${(p.pluginTypes ?? []).map(t => html`
            <span class="text-[10px] font-medium text-accent bg-accent/10 px-1.5 py-0.5 rounded border border-accent/20">${t}</span>
          `)}
        </div>
        ${this.#statusBadge(p.id)}
      </div>
    `;
  }

  override render() {
    const webPlugins   = this.plugins.filter(p =>  p.pluginTypes?.includes('web'));
    const otherPlugins = this.plugins.filter(p => !p.pluginTypes?.includes('web'));

    return html`
      <div class="min-h-screen bg-bg p-6 space-y-1">

        <div class="flex items-center justify-between mb-2">
          <div>
            <h1 class="text-xl font-semibold text-text">Plugins</h1>
            <p class="text-sm text-text-muted">${this.plugins.length} plugin${this.plugins.length !== 1 ? 's' : ''} discovered</p>
          </div>
          ${this.dirty ? html`
            <div class="flex items-center gap-3 px-3 py-2 bg-yellow-400/10 border border-yellow-400/30 rounded text-yellow-400 text-xs">
              <span>Reload to apply changes.</span>
              <button
                type="button"
                class="px-2.5 py-1 rounded bg-yellow-400/20 hover:bg-yellow-400/30 transition-colors font-medium"
                @click=${() => location.reload()}
              >Reload now</button>
            </div>
          ` : nothing}
        </div>

        ${webPlugins.length > 0 ? html`
          <div>
            <h2 class="text-xs font-semibold text-text-muted uppercase tracking-wider mb-2">Web plugins</h2>
            <div class="space-y-0_25">
              ${webPlugins.map(p => this.#renderPlugin(p, true))}
            </div>
          </div>
        ` : nothing}

        ${otherPlugins.length > 0 ? html`
          <div>
            <h2 class="text-xs font-semibold text-text-muted uppercase tracking-wider mb-2">Other plugins</h2>
            <div class="space-y-0_25">
              ${otherPlugins.map(p => this.#renderPlugin(p, false))}
            </div>
          </div>
        ` : nothing}

        ${this.plugins.length === 0 ? html`
          <div class="flex items-center justify-center py-24 text-text-muted text-sm">
            No plugins discovered.
          </div>
        ` : nothing}

      </div>
    `;
  }
}

export class PluginsPlugin extends AdiPlugin {
  readonly id = 'app.plugins';
  readonly version = '1.0.0';

  override async onRegister(): Promise<void> {
    if (!customElements.get('app-plugins-page')) {
      customElements.define('app-plugins-page', AppPluginsPage);
    }

    this.bus.emit('route:register', { path: '/plugins', element: 'app-plugins-page', label: 'Plugins' }, 'plugins-page');
    this.bus.send('nav:add', { id: 'app.plugins', label: 'Plugins', path: '/plugins' }, 'plugins-page').handle(() => {});

    this.bus.emit('command:register', { id: 'app:plugins', label: 'Open Plugins page', shortcut: '⌘⇧P' }, 'plugins-page');
    this.bus.on('command:execute', ({ id }) => {
      if (id === 'app:plugins') this.bus.emit('router:navigate', { path: '/plugins' }, 'plugins-page');
    }, 'plugins-page');
  }
}
