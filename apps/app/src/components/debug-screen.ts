import { LitElement, html, nothing } from 'lit';
import { customElement, property, state } from 'lit/decorators.js';
import { type EventBus, HttpPluginRegistry, type RegistryHealth } from '@adi-family/sdk-plugin';
import { getEnabledWebPluginIds, setEnabledWebPluginIds } from '../plugin-prefs.ts';

interface NavItem { id: string; label: string; path: string; icon?: string }
interface RouteEntry { path: string; element: string; label?: string }
interface Command { id: string; label: string; shortcut?: string }

interface EventLogEntry {
  id: number;
  time: string;
  phase: 'before' | 'after';
  event: string;
  payload: unknown;
}

interface RegistryStatus {
  registry: HttpPluginRegistry;
  checking: boolean;
  health: RegistryHealth | null;
  checkedAt: string | null;
}

let seq = 0;

type Tab = 'overview' | 'plugins' | 'routes' | 'commands' | 'connections' | 'events' | 'registries';

@customElement('app-debug-screen')
export class AppDebugScreen extends LitElement {
  @property({ attribute: false }) routes: RouteEntry[] = [];
  @property({ attribute: false }) navItems: NavItem[] = [];
  @property({ attribute: false }) commands: Command[] = [];

  @state() private activeTab: Tab = 'overview';
  @state() private eventLog: EventLogEntry[] = [];
  @state() private eventFilter = '';
  @state() private eventPaused = false;
  @state() private registries: RegistryStatus[] = [];
  @state() private allPlugins: Array<{ id: string; installedVersion: string; pluginTypes?: string[] }> = [];
  @state() private enabledWebIds: Set<string> = getEnabledWebPluginIds() ?? new Set();
  @state() private pluginsDirty = false;

  private eventUnsub: (() => void) | null = null;
  private loadingUnsub: (() => void) | null = null;

  private pluginStatus: { loaded: string[]; failed: string[]; timedOut: string[] } = {
    loaded: [],
    failed: [],
    timedOut: [],
  };

  override createRenderRoot() { return this; }

  override connectedCallback(): void {
    super.connectedCallback();
    this.#loadDebugData();
    if ((window as { sdk?: unknown }).sdk) {
      this.#subscribeEventLog();
      // Re-load after plugins finish loading since __adiAllPlugins is set async
      this.loadingUnsub = window.sdk.bus.on('loading-finished', () => this.#loadDebugData());
    } else {
      window.addEventListener('sdk-ready', () => {
        this.#loadDebugData();
        this.#subscribeEventLog();
        this.loadingUnsub = window.sdk.bus.on('loading-finished', () => this.#loadDebugData());
      }, { once: true });
    }
  }

  override disconnectedCallback(): void {
    super.disconnectedCallback();
    this.eventUnsub?.();
    this.eventUnsub = null;
    this.loadingUnsub?.();
    this.loadingUnsub = null;
  }

  #loadDebugData(): void {
    const w = window as unknown as Record<string, unknown>;
    const debug = w['__adiDebug'] as { loaded?: string[]; failed?: string[]; timedOut?: string[]; registries?: HttpPluginRegistry[] } | undefined;
    if (debug) {
      this.pluginStatus = {
        loaded: debug.loaded ?? [],
        failed: debug.failed ?? [],
        timedOut: debug.timedOut ?? [],
      };
      // Preserve existing health results for known instances; initialise new ones.
      this.registries = (debug.registries ?? []).map(registry => {
        const existing = this.registries.find(r => r.registry === registry);
        return existing ?? { registry, checking: false, health: null, checkedAt: null };
      });
    }
    const all = w['__adiAllPlugins'];
    if (Array.isArray(all)) {
      this.allPlugins = all as Array<{ id: string; installedVersion: string; pluginTypes?: string[] }>;
    }
  }

  async #checkRegistry(registry: HttpPluginRegistry): Promise<void> {
    this.registries = this.registries.map(r =>
      r.registry === registry ? { ...r, checking: true } : r
    );
    const health = await registry.checkHealth();
    const checkedAt = new Date().toLocaleTimeString([], { hour12: false, hour: '2-digit', minute: '2-digit', second: '2-digit' });
    this.registries = this.registries.map(r =>
      r.registry === registry ? { ...r, checking: false, health, checkedAt } : r
    );
  }

  #checkAllRegistries(): void {
    for (const { registry } of this.registries) void this.#checkRegistry(registry);
  }

  #subscribeEventLog(): void {
    const bus = window.sdk.bus as EventBus;
    this.eventUnsub = bus.use({
      before: (event, payload) => this.#pushEvent('before', event, payload),
      after:  (event, payload) => this.#pushEvent('after',  event, payload),
    });
  }

  #pushEvent(phase: 'before' | 'after', event: string, payload: unknown): void {
    if (this.eventPaused) return;
    this.eventLog = [{
      id: ++seq,
      time: new Date().toLocaleTimeString([], { hour12: false, hour: '2-digit', minute: '2-digit', second: '2-digit', fractionalSecondDigits: 3 } as Intl.DateTimeFormatOptions),
      phase,
      event,
      payload,
    }, ...this.eventLog].slice(0, 500);
  }

  #filteredEvents(): EventLogEntry[] {
    const q = this.eventFilter.trim().toLowerCase();
    if (!q) return this.eventLog;
    return this.eventLog.filter(e => e.event.toLowerCase().includes(q));
  }

  #connections(): Map<string, { id: string; services: string[] }> {
    if ((window as { sdk?: unknown }).sdk) {
      return window.sdk.getConnections() as Map<string, { id: string; services: string[] }>;
    }
    return new Map();
  }

  #renderTab(id: Tab, label: string, count?: number) {
    return html`
      <button
        type="button"
        class=${[
          'px-4 py-2 text-sm font-medium border-b-2 transition-colors whitespace-nowrap',
          this.activeTab === id
            ? 'border-accent text-accent'
            : 'border-transparent text-text-muted hover:text-text',
        ].join(' ')}
        @click=${() => {
          this.activeTab = id;
          if (id === 'registries') {
            this.#loadDebugData();
            if (this.registries.every(r => r.health === null)) this.#checkAllRegistries();
          }
        }}
      >
        ${label}
        ${count !== undefined
          ? html`<span class="ml-1.5 text-xs bg-surface-alt px-1.5 py-0.5 rounded-full">${count}</span>`
          : nothing}
      </button>
    `;
  }

  #renderOverview() {
    const connections = this.#connections();
    const rows = [
      ['Plugins loaded', String(this.pluginStatus.loaded.length)],
      ['Plugins failed', String(this.pluginStatus.failed.length)],
      ['Plugins timed out', String(this.pluginStatus.timedOut.length)],
      ['Routes registered', String(this.routes.length)],
      ['Nav items', String(this.navItems.length)],
      ['Commands', String(this.commands.length)],
      ['Connections', String(connections.size)],
      ['Registries', String(this.registries.length)],
      ['Events captured', String(this.eventLog.length)],
    ];
    return html`
      <div class="grid grid-cols-2 sm:grid-cols-4 gap-4 p-6">
        ${rows.map(([label, value]) => html`
          <div class="bg-surface-alt rounded-lg p-4 border border-border">
            <p class="text-xs text-text-muted mb-1">${label}</p>
            <p class="text-2xl font-mono font-bold text-text">${value}</p>
          </div>
        `)}
      </div>
    `;
  }

  #statusBadge(id: string) {
    const { loaded, failed, timedOut } = this.pluginStatus;
    if (loaded.includes(id))   return html`<span class="text-[10px] font-medium text-green-400 bg-green-400/10 px-1.5 py-0.5 rounded">loaded</span>`;
    if (failed.includes(id))   return html`<span class="text-[10px] font-medium text-red-400 bg-red-400/10 px-1.5 py-0.5 rounded">failed</span>`;
    if (timedOut.includes(id)) return html`<span class="text-[10px] font-medium text-yellow-400 bg-yellow-400/10 px-1.5 py-0.5 rounded">timed out</span>`;
    return nothing;
  }

  #typeBadges(types: string[] | undefined) {
    if (!types?.length) return nothing;
    return types.map(t => html`
      <span class="text-[10px] font-medium text-accent bg-accent/10 px-1.5 py-0.5 rounded border border-accent/20">${t}</span>
    `);
  }

  #toggleWebPlugin(id: string, enabled: boolean): void {
    const next = new Set(this.enabledWebIds);
    if (enabled) next.add(id); else next.delete(id);
    this.enabledWebIds = next;
    setEnabledWebPluginIds(next);
    this.pluginsDirty = true;
  }

  #renderPlugins() {
    const plugins = this.allPlugins;

    if (plugins.length === 0) {
      return html`<div class="p-6"><p class="text-text-muted text-sm">No registry plugins discovered yet.</p></div>`;
    }

    return html`
      <div class="p-6 space-y-0_25">
        ${this.pluginsDirty ? html`
          <div class="flex items-center justify-between px-3 py-2 mb-2 bg-yellow-400/10 border border-yellow-400/30 rounded text-yellow-400 text-xs">
            <span>Reload the page to apply plugin changes.</span>
            <button
              type="button"
              class="ml-3 px-2.5 py-1 rounded bg-yellow-400/20 hover:bg-yellow-400/30 transition-colors font-medium"
              @click=${() => location.reload()}
            >Reload now</button>
          </div>
        ` : nothing}
        ${plugins.map(p => {
          const isWeb = p.pluginTypes?.includes('web') ?? false;
          const enabled = this.enabledWebIds.has(p.id);
          return html`
            <div class="flex items-center gap-3 px-3 py-2.5 bg-surface-alt rounded border border-border">
              ${isWeb ? html`
                <input
                  type="checkbox"
                  class="shrink-0 accent-accent w-3.5 h-3.5 cursor-pointer"
                  .checked=${enabled}
                  @change=${(e: Event) => this.#toggleWebPlugin(p.id, (e.target as HTMLInputElement).checked)}
                />
              ` : html`<span class="shrink-0 w-3.5"></span>`}
              <code class="text-sm font-mono text-text flex-1 truncate">${p.id}</code>
              <span class="text-xs text-text-muted font-mono shrink-0">v${p.installedVersion}</span>
              <div class="flex items-center gap-1 shrink-0">
                ${this.#typeBadges(p.pluginTypes)}
              </div>
              ${this.#statusBadge(p.id)}
            </div>
          `;
        })}
      </div>
    `;
  }

  #renderRoutes() {
    return html`
      <div class="p-6">
        ${this.routes.length === 0
          ? html`<p class="text-text-muted text-sm">No routes registered.</p>`
          : html`
            <table class="w-full text-sm border-collapse">
              <thead>
                <tr class="text-left border-b border-border">
                  <th class="pb-2 pr-4 text-xs font-semibold text-text-muted uppercase tracking-wider">Path</th>
                  <th class="pb-2 pr-4 text-xs font-semibold text-text-muted uppercase tracking-wider">Element</th>
                  <th class="pb-2 text-xs font-semibold text-text-muted uppercase tracking-wider">Label</th>
                </tr>
              </thead>
              <tbody>
                ${this.routes.map(r => html`
                  <tr class="border-b border-border/50 hover:bg-surface-alt transition-colors">
                    <td class="py-2.5 pr-4">
                      <code class="text-accent font-mono">${r.path}</code>
                    </td>
                    <td class="py-2.5 pr-4">
                      <code class="text-text-muted font-mono text-xs">&lt;${r.element}&gt;</code>
                    </td>
                    <td class="py-2.5 text-text-muted">${r.label ?? '—'}</td>
                  </tr>
                `)}
              </tbody>
            </table>
          `}
      </div>
    `;
  }

  #renderCommands() {
    return html`
      <div class="p-6">
        ${this.commands.length === 0
          ? html`<p class="text-text-muted text-sm">No commands registered.</p>`
          : html`
            <table class="w-full text-sm border-collapse">
              <thead>
                <tr class="text-left border-b border-border">
                  <th class="pb-2 pr-4 text-xs font-semibold text-text-muted uppercase tracking-wider">ID</th>
                  <th class="pb-2 pr-4 text-xs font-semibold text-text-muted uppercase tracking-wider">Label</th>
                  <th class="pb-2 text-xs font-semibold text-text-muted uppercase tracking-wider">Shortcut</th>
                </tr>
              </thead>
              <tbody>
                ${this.commands.map(c => html`
                  <tr class="border-b border-border/50 hover:bg-surface-alt transition-colors">
                    <td class="py-2.5 pr-4">
                      <code class="text-text-muted font-mono text-xs">${c.id}</code>
                    </td>
                    <td class="py-2.5 pr-4 text-text">${c.label}</td>
                    <td class="py-2.5">
                      ${c.shortcut
                        ? html`<kbd class="text-xs bg-surface-alt border border-border rounded px-1.5 py-0.5 font-mono">${c.shortcut}</kbd>`
                        : html`<span class="text-text-muted">—</span>`}
                    </td>
                  </tr>
                `)}
              </tbody>
            </table>
          `}
      </div>
    `;
  }

  #renderConnections() {
    const connections = this.#connections();
    return html`
      <div class="p-6">
        ${connections.size === 0
          ? html`<p class="text-text-muted text-sm">No connections available.</p>`
          : html`
            <div class="space-y-1">
              ${[...connections.values()].map(conn => html`
                <div class="border border-border rounded-lg p-4 bg-surface-alt">
                  <div class="flex items-center gap-2 mb-3">
                    <span class="w-2 h-2 rounded-full bg-green-400"></span>
                    <code class="text-sm font-mono font-bold text-text">${conn.id}</code>
                  </div>
                  <div class="flex flex-wrap gap-2">
                    ${conn.services.map(s => html`
                      <span class="text-xs bg-surface border border-border rounded px-2 py-1 font-mono text-text-muted">${s}</span>
                    `)}
                  </div>
                </div>
              `)}
            </div>
          `}
      </div>
    `;
  }

  #renderRegistries() {
    if (this.registries.length === 0) {
      return html`<div class="p-6"><p class="text-text-muted text-sm">No registries configured.</p></div>`;
    }
    return html`
      <div class="p-6 space-y-0_75">
        <div class="flex justify-end mb-2">
          <button
            type="button"
            class="text-xs px-3 py-1.5 rounded border border-border text-text-muted hover:text-text hover:bg-surface-alt transition-colors"
            @click=${() => this.#checkAllRegistries()}
          >Check all</button>
        </div>
        ${this.registries.map(r => html`
          <div class="border border-border rounded-lg p-4 bg-surface-alt">
            <div class="flex items-center gap-3 mb-3">

              <!-- Status dot -->
              ${r.checking
                ? html`<span class="w-2.5 h-2.5 rounded-full bg-yellow-400 animate-pulse shrink-0"></span>`
                : r.health === null
                  ? html`<span class="w-2.5 h-2.5 rounded-full bg-border shrink-0"></span>`
                  : r.health.online
                    ? html`<span class="w-2.5 h-2.5 rounded-full bg-green-400 shrink-0"></span>`
                    : html`<span class="w-2.5 h-2.5 rounded-full bg-red-400 shrink-0"></span>`}

              <!-- URL -->
              <code class="text-sm font-mono text-text flex-1 truncate">${r.registry.url}</code>

              <!-- Check button -->
              <button
                type="button"
                class="text-xs px-2.5 py-1 rounded border border-border text-text-muted hover:text-text hover:bg-surface transition-colors shrink-0"
                ?disabled=${r.checking}
                @click=${() => this.#checkRegistry(r.registry)}
              >${r.checking ? 'Checking…' : 'Check'}</button>
            </div>

            ${r.health !== null ? html`
              <div class="grid grid-cols-3 gap-3 mt-2">
                <div>
                  <p class="text-[10px] text-text-muted uppercase tracking-wider mb-0.5">Status</p>
                  <p class="text-sm font-medium ${r.health.online ? 'text-green-400' : 'text-red-400'}">
                    ${r.health.online ? 'Online' : 'Offline'}
                  </p>
                </div>
                <div>
                  <p class="text-[10px] text-text-muted uppercase tracking-wider mb-0.5">Plugins</p>
                  <p class="text-sm font-mono text-text">${r.health.online ? String(r.health.pluginCount) : '—'}</p>
                </div>
                <div>
                  <p class="text-[10px] text-text-muted uppercase tracking-wider mb-0.5">Latency</p>
                  <p class="text-sm font-mono text-text">${r.health.online ? `${r.health.latencyMs} ms` : '—'}</p>
                </div>
                ${r.health.version ? html`
                  <div>
                    <p class="text-[10px] text-text-muted uppercase tracking-wider mb-0.5">Version</p>
                    <p class="text-sm font-mono text-text">${r.health.version}</p>
                  </div>
                ` : nothing}
              </div>
            ` : nothing}

            ${r.checkedAt ? html`
              <p class="text-[10px] text-text-muted mt-3">Last checked ${r.checkedAt}</p>
            ` : nothing}
          </div>
        `)}
      </div>
    `;
  }

  #renderEventLog() {
    const rows = this.#filteredEvents();
    return html`
      <div class="flex flex-col h-full">

        <!-- Event log toolbar -->
        <div class="shrink-0 border-b border-border px-4 py-2 flex items-center gap-3">
          <div class="flex items-center gap-2 flex-1 max-w-xs">
            <svg class="w-3.5 h-3.5 text-text-muted shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"/>
            </svg>
            <input
              type="text"
              placeholder="Filter events…"
              class="flex-1 bg-transparent text-text placeholder:text-text-muted text-sm outline-none"
              .value=${this.eventFilter}
              @input=${(e: Event) => { this.eventFilter = (e.target as HTMLInputElement).value; }}
            />
          </div>
          <span class="text-xs text-text-muted ml-auto">${rows.length} event${rows.length !== 1 ? 's' : ''}</span>
          <button
            type="button"
            class=${[
              'text-xs px-3 py-1 rounded border transition-colors',
              this.eventPaused
                ? 'border-accent text-accent bg-accent/10 hover:bg-accent/20'
                : 'border-border text-text-muted hover:text-text hover:bg-surface-alt',
            ].join(' ')}
            @click=${() => { this.eventPaused = !this.eventPaused; }}
          >${this.eventPaused ? '▶ Resume' : '⏸ Pause'}</button>
          <button
            type="button"
            class="text-xs px-3 py-1 rounded border border-border text-text-muted hover:text-text hover:bg-surface-alt transition-colors"
            @click=${() => { this.eventLog = []; }}
          >Clear</button>
        </div>

        <!-- Rows -->
        <div class="flex-1 overflow-auto font-mono text-xs">
          ${rows.length === 0
            ? html`
                <div class="flex flex-col items-center justify-center py-24 gap-2 text-text-muted">
                  <span class="text-2xl">📭</span>
                  <span>${this.eventLog.length === 0 ? 'Waiting for events…' : 'No events match the filter'}</span>
                </div>
              `
            : rows.map(entry => html`
                <div class="flex items-start border-b border-border/40 hover:bg-surface transition-colors">
                  <span class=${[
                    'shrink-0 w-14 text-center py-2 text-[10px] font-bold uppercase tracking-wider',
                    entry.phase === 'before' ? 'text-blue-400' : 'text-purple-400',
                  ].join(' ')}>${entry.phase}</span>
                  <span class="shrink-0 w-28 py-2 text-text-muted">${entry.time}</span>
                  <span class="shrink-0 w-56 py-2 text-accent font-bold truncate pr-4">${entry.event}</span>
                  <span class="py-2 text-text-muted truncate flex-1 pr-4">
                    ${entry.payload == null
                      ? html`<span class="italic">—</span>`
                      : JSON.stringify(entry.payload)}
                  </span>
                </div>
              `)
          }
        </div>
      </div>
    `;
  }

  override render() {
    return html`
      <div class="min-h-screen bg-bg">

        <!-- Header -->
        <div class="border-b border-border bg-surface px-6 py-4 flex items-center gap-3">
          <svg class="w-5 h-5 text-text-muted shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M12 18a6 6 0 1 0 0-12 6 6 0 0 0 0 12Z"/>
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M12 6V2m0 16v4M4.93 4.93l2.83 2.83m8.48 8.48 2.83 2.83M2 12h4m12 0h4M4.93 19.07l2.83-2.83m8.48-8.48 2.83-2.83"/>
            </svg>
          <div>
            <h1 class="text-base font-semibold text-text">Debug Screen</h1>
            <p class="text-xs text-text-muted">App internals &amp; event monitor</p>
          </div>
          <div class="ml-auto">
            <button
              type="button"
              class="text-xs text-text-muted hover:text-text px-3 py-1.5 rounded border border-border hover:bg-surface-alt transition-colors"
              @click=${() => { this.requestUpdate(); this.#loadDebugData(); }}
            >
              Refresh
            </button>
          </div>
        </div>

        <!-- Tabs -->
        <div class="flex items-center gap-0 border-b border-border bg-surface px-4 overflow-x-auto">
          ${this.#renderTab('overview', 'Overview')}
          ${this.#renderTab('plugins', 'Plugins')}
          ${this.#renderTab('routes', 'Routes')}
          ${this.#renderTab('commands', 'Commands')}
          ${this.#renderTab('connections', 'Connections')}
          ${this.#renderTab('registries', 'Registries')}
          ${this.#renderTab('events', 'Operations')}
        </div>

        <!-- Tab content -->
        <div class="overflow-auto">
          ${this.activeTab === 'overview' ? this.#renderOverview() : nothing}
          ${this.activeTab === 'plugins' ? this.#renderPlugins() : nothing}
          ${this.activeTab === 'routes' ? this.#renderRoutes() : nothing}
          ${this.activeTab === 'commands' ? this.#renderCommands() : nothing}
          ${this.activeTab === 'connections' ? this.#renderConnections() : nothing}
          ${this.activeTab === 'registries' ? this.#renderRegistries() : nothing}
          ${this.activeTab === 'events' ? this.#renderEventLog() : nothing}
        </div>

      </div>
    `;
  }
}
