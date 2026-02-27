import { LitElement, html, nothing } from 'lit';
import { customElement, state } from 'lit/decorators.js';
import type { EventBus } from '@adi-family/sdk-plugin';

interface EventLogEntry {
  id: number;
  time: string;
  phase: 'before' | 'after' | 'both';
  event: string;
  payload: unknown;
}

let seq = 0;

@customElement('app-ops-log')
export class AppOpsLog extends LitElement {
  @state() private open = false;
  @state() private eventLog: EventLogEntry[] = [];
  @state() private filter = '';
  @state() private paused = false;

  private unsub: (() => void) | null = null;

  override createRenderRoot() { return this; }

  override connectedCallback(): void {
    super.connectedCallback();
    window.addEventListener('keydown', this.#onKeyDown);
    if ((window as { sdk?: unknown }).sdk) {
      this.#subscribe();
    } else {
      window.addEventListener('sdk-ready', () => this.#subscribe(), { once: true });
    }
  }

  override disconnectedCallback(): void {
    super.disconnectedCallback();
    window.removeEventListener('keydown', this.#onKeyDown);
    this.unsub?.();
    this.unsub = null;
  }

  #subscribe(): void {
    const bus = window.sdk.bus as EventBus;
    this.unsub = bus.use({
      before: (event, payload) => this.#push('before', event, payload),
      after:  (event, payload) => this.#push('after',  event, payload),
    });

    bus.on('command:execute', ({ id }) => {
      if (id === 'app:ops-log') this.open = !this.open;
    });
  }

  #push(phase: 'before' | 'after', event: string, payload: unknown): void {
    if (this.paused) return;
    // Merge "after" into existing "before" for the same event name
    if (phase === 'after') {
      const idx = this.eventLog.findIndex(e => e.event === event && e.phase === 'before');
      if (idx !== -1) {
        const next = [...this.eventLog];
        next[idx] = { ...next[idx], phase: 'both' };
        this.eventLog = next;
        return;
      }
    }
    this.eventLog = [{
      id: ++seq,
      time: new Date().toLocaleTimeString([], { hour12: false, hour: '2-digit', minute: '2-digit', second: '2-digit', fractionalSecondDigits: 3 } as Intl.DateTimeFormatOptions),
      phase,
      event,
      payload,
    }, ...this.eventLog].slice(0, 500);
  }

  #filtered(): EventLogEntry[] {
    const q = this.filter.trim().toLowerCase();
    if (!q) return this.eventLog;
    return this.eventLog.filter(e => e.event.toLowerCase().includes(q));
  }

  readonly #onKeyDown = (e: KeyboardEvent): void => {
    // Toggle on Cmd+Shift+O (Mac) or Ctrl+Shift+O
    if ((e.metaKey || e.ctrlKey) && e.shiftKey && e.key.toLowerCase() === 'o') {
      e.preventDefault();
      this.open = !this.open;
      return;
    }
    if (e.key === 'Escape' && this.open) {
      this.open = false;
    }
  };

  override render() {
    const rows = this.#filtered();

    return html`
      <!-- Floating button -->
      <button
        type="button"
        class="fixed bottom-4 left-4 z-40 flex items-center gap-2 px-3 py-2 rounded-lg bg-surface border border-border shadow-lg text-sm text-text hover:bg-surface-alt transition-colors cursor-pointer"
        @click=${() => { this.open = !this.open; }}
      >
        <span class="text-text-muted">Ops Log</span>
        ${this.eventLog.length > 0
          ? html`<span class="text-[10px] font-bold bg-accent/15 text-accent px-1.5 py-0.5 rounded-full">${this.eventLog.length}</span>`
          : nothing}
      </button>

      <!-- Drawer panel -->
      ${this.open ? html`
        <div
          class="fixed bottom-14 left-4 z-40 w-[90vw] max-h-[90vh] flex flex-col bg-surface border border-border rounded-xl shadow-2xl overflow-hidden"
          style="animation: ops-slide-up .15s ease-out"
        >
          <!-- Header -->
          <div class="shrink-0 border-b border-border px-4 py-2.5 flex items-center gap-2">
            <span class="text-sm font-semibold text-text">Operations</span>
            <span class="text-xs text-text-muted">${this.eventLog.length} event${this.eventLog.length !== 1 ? 's' : ''}</span>
            <kbd class="ml-auto text-[10px] text-text-muted bg-surface-alt px-1.5 py-0.5 rounded border border-border font-mono">⌘⇧O</kbd>
          </div>

          <!-- Toolbar -->
          <div class="shrink-0 border-b border-border px-3 py-1.5 flex items-center gap-2">
            <svg class="w-3.5 h-3.5 text-text-muted shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"/>
            </svg>
            <input
              type="text"
              placeholder="Filter events…"
              class="flex-1 bg-transparent text-text placeholder:text-text-muted text-xs outline-none"
              .value=${this.filter}
              @input=${(e: Event) => { this.filter = (e.target as HTMLInputElement).value; }}
            />
            <button
              type="button"
              class=${[
                'text-[10px] px-2 py-0.5 rounded border transition-colors cursor-pointer',
                this.paused
                  ? 'border-accent text-accent bg-accent/10 hover:bg-accent/20'
                  : 'border-border text-text-muted hover:text-text hover:bg-surface-alt',
              ].join(' ')}
              @click=${() => { this.paused = !this.paused; }}
            >${this.paused ? '▶ Resume' : '⏸ Pause'}</button>
            <button
              type="button"
              class="text-[10px] px-2 py-0.5 rounded border border-border text-text-muted hover:text-text hover:bg-surface-alt transition-colors cursor-pointer"
              @click=${() => { this.eventLog = []; }}
            >Clear</button>
          </div>

          <!-- Event table -->
          <div class="flex-1 overflow-auto font-mono text-[11px]">
            ${rows.length === 0
              ? html`
                  <div class="flex flex-col items-center justify-center py-12 gap-1 text-text-muted text-xs">
                    <span>${this.eventLog.length === 0 ? 'Waiting for events…' : 'No events match filter'}</span>
                  </div>
                `
              : html`
                  <table class="w-full border-collapse">
                    <thead class="sticky top-0 bg-surface-alt">
                      <tr class="text-[9px] uppercase tracking-wider text-text-muted">
                        <th class="text-center px-2 py-1.5 font-semibold w-10">Phase</th>
                        <th class="text-left px-2 py-1.5 font-semibold w-24">Time</th>
                        <th class="text-left px-2 py-1.5 font-semibold">Event</th>
                        <th class="text-left px-2 py-1.5 font-semibold">Payload</th>
                      </tr>
                    </thead>
                    <tbody>
                      ${rows.map(entry => html`
                        <tr class="border-b border-border/40 hover:bg-surface-alt/50 transition-colors">
                          <td class=${[
                            'text-center px-2 py-1.5 text-[9px] font-bold uppercase tracking-wider whitespace-nowrap',
                            entry.phase === 'both' ? 'text-green-400' : entry.phase === 'before' ? 'text-blue-400' : 'text-purple-400',
                          ].join(' ')}>${entry.phase === 'both' ? 'b/a' : entry.phase}</td>
                          <td class="px-2 py-1.5 text-text-muted whitespace-nowrap">${entry.time}</td>
                          <td class="px-2 py-1.5 text-accent font-bold whitespace-nowrap">${entry.event}</td>
                          <td class="px-2 py-1.5 text-text-muted break-all">
                            ${entry.payload == null
                              ? html`<span class="italic">—</span>`
                              : JSON.stringify(entry.payload)}
                          </td>
                        </tr>
                      `)}
                    </tbody>
                  </table>
                `
            }
          </div>
        </div>

        <style>
          @keyframes ops-slide-up {
            from { opacity: 0; transform: translateY(8px); }
            to   { opacity: 1; transform: translateY(0); }
          }
        </style>
      ` : nothing}
    `;
  }
}
