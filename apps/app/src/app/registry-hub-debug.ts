import { LitElement, html, nothing } from 'lit';
import { customElement, state } from 'lit/decorators.js';
import type { RegistryHub } from './registry-hub';
import type { RegistryState } from './registry-server';

interface RegistryDebugEntry {
  url: string;
  state: RegistryState;
  pluginCount: number;
  protected: boolean;
}

@customElement('adi-registry-hub-debug')
export class AdiRegistryHubDebugElement extends LitElement {
  @state() entries: RegistryDebugEntry[] = [];

  override createRenderRoot() {
    return this;
  }

  override render() {
    return html`
      <div style="display:flex;flex-direction:column;gap:1rem">
        <section>
          <div class="text-xs uppercase" style="color:var(--adi-text-muted);font-weight:600;margin-bottom:0.5rem">
            Registries (${this.entries.length})
          </div>
          ${this.entries.length === 0
            ? html`<p class="text-sm" style="color:var(--adi-text-muted)">No registries configured.</p>`
            : html`
              <table class="dr-table text-sm">
                <thead>
                  <tr>
                    <th class="dr-th">URL</th>
                    <th class="dr-th">State</th>
                    <th class="dr-th">Plugins</th>
                    <th class="dr-th">Protected</th>
                  </tr>
                </thead>
                <tbody>
                  ${this.entries.map(
                    (e) => html`
                      <tr class="dr-row">
                        <td class="dr-td"><code>${e.url}</code></td>
                        <td class="dr-td">${this.renderState(e.state)}</td>
                        <td class="dr-td">${e.pluginCount}</td>
                        <td class="dr-td">${e.protected ? html`<span style="color:var(--adi-accent)">●</span>` : nothing}</td>
                      </tr>
                    `,
                  )}
                </tbody>
              </table>
            `}
        </section>
      </div>
    `;
  }

  private renderState(state: RegistryState) {
    const colors: Record<RegistryState, string> = {
      connected: 'var(--adi-accent)',
      connecting: 'var(--adi-text-muted)',
      disconnected: 'var(--adi-error, #ef4444)',
    };
    return html`<span style="color:${colors[state]}">${state}</span>`;
  }
}

const SYNC_INTERVAL_MS = 2_000;

export function createRegistryHubDebugSync(hub: RegistryHub): {
  init: () => HTMLElement;
  dispose: () => void;
} {
  let el: AdiRegistryHubDebugElement | null = null;
  let timer: ReturnType<typeof setInterval> | null = null;

  function sync() {
    if (!el) return;
    const entries: RegistryDebugEntry[] = [];
    for (const [url, server] of hub.allServers()) {
      entries.push({
        url,
        state: server.getState(),
        pluginCount: server.getPlugins().length,
        protected: hub.isProtected(url),
      });
    }
    el.entries = entries;
  }

  return {
    init: () => {
      el = document.createElement('adi-registry-hub-debug') as AdiRegistryHubDebugElement;
      sync();
      timer = setInterval(sync, SYNC_INTERVAL_MS);
      return el;
    },
    dispose: () => {
      if (timer) clearInterval(timer);
      el = null;
    },
  };
}
