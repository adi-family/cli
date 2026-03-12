import { LitElement, html, nothing } from 'lit';
import { property, state } from 'lit/decorators.js';
import { AdiSignalingBusKey } from '@adi-family/plugin-signaling';
import { CocoonBusKey, type Connection } from './bus-keys.js';
import type { CocoonPluginInterface } from './cocoon-interface.js';

export interface CocoonSelectEvent {
  cocoonId: string;
  connection: Connection;
}

interface CocoonDisplayItem {
  deviceId: string;
  online: boolean;
  connected: boolean;
  pluginInstalled: boolean;
}

export class CocoonSelectElement extends LitElement {
  @property({ attribute: 'with-plugin' }) withPlugin = '';
  @property() value = '';
  @property() label = 'Select Cocoon';

  cocoonInterface: CocoonPluginInterface | null = null;

  @state() private open = false;
  @state() private installing: string | null = null;
  @state() private error: string | null = null;
  @state() private items: CocoonDisplayItem[] = [];

  private unsubs: Array<() => void> = [];

  override createRenderRoot() { return this; }

  override connectedCallback(): void {
    super.connectedCallback();
    this.refresh();
    this.subscribeToBus();
  }

  override disconnectedCallback(): void {
    super.disconnectedCallback();
    this.unsubs.forEach(fn => fn());
    this.unsubs = [];
  }

  private subscribeToBus(): void {
    const iface = this.cocoonInterface;
    if (!iface) return;

    const bus = iface.bus;
    this.unsubs.push(
      bus.on(CocoonBusKey.ConnectionAdded, () => this.refresh(), 'cocoon-select'),
      bus.on(CocoonBusKey.ConnectionRemoved, () => this.refresh(), 'cocoon-select'),
      bus.on(AdiSignalingBusKey.Devices, () => this.refresh(), 'cocoon-select'),
    );
  }

  refresh(): void {
    const iface = this.cocoonInterface;
    if (!iface) {
      this.items = [];
      return;
    }

    const devices = iface.cocoonDevices();
    const connections = new Map(iface.allConnections().map(c => [c.id, c]));
    const pluginConns = new Set(
      this.withPlugin
        ? iface.connectionsWithPlugin(this.withPlugin).map(c => c.id)
        : [],
    );

    this.items = devices.map(d => ({
      deviceId: d.device_id,
      online: d.online,
      connected: connections.has(d.device_id),
      pluginInstalled: pluginConns.has(d.device_id),
    }));
  }

  show(): void {
    this.refresh();
    this.open = true;
    this.error = null;
  }

  close(): void {
    this.open = false;
    this.installing = null;
    this.error = null;
  }

  private select(item: CocoonDisplayItem): void {
    if (!item.pluginInstalled) return;
    const connection = this.resolveConnection(item.deviceId);
    if (!connection) return;

    this.value = item.deviceId;
    this.close();
    this.dispatchEvent(new CustomEvent<CocoonSelectEvent>('cocoon-selected', {
      detail: { cocoonId: item.deviceId, connection },
      bubbles: true,
      composed: true,
    }));
  }

  private resolveConnection(deviceId: string): Connection | undefined {
    const existing = this.cocoonInterface?.allConnections().find(c => c.id === deviceId);
    if (existing) return existing;
    return this.cocoonInterface?.connectProvider?.(deviceId) ?? undefined;
  }

  private async installPlugin(item: CocoonDisplayItem): Promise<void> {
    if (!this.withPlugin) return;

    this.installing = item.deviceId;
    this.error = null;

    try {
      const connection = this.resolveConnection(item.deviceId);
      if (!connection) throw new Error('Could not connect to cocoon');

      await connection.installPlugin(this.withPlugin);
      await connection.refreshPlugins();
      this.refresh();
    } catch (e) {
      this.error = e instanceof Error ? e.message : 'Install failed';
    } finally {
      this.installing = null;
    }
  }

  private requestSetup(): void {
    this.dispatchEvent(new CustomEvent('cocoon-setup-requested', {
      bubbles: true,
      composed: true,
    }));
  }

  override render() {
    return html`
      <button
        class="w-full px-3 py-2 rounded-lg bg-white/5 border border-white/10 text-sm text-gray-200 text-left hover:bg-white/10 transition-colors focus:outline-none focus:border-purple-500/50"
        @click=${() => this.show()}
      >
        ${this.value
          ? html`<span class="flex items-center gap-2">
              <span class="text-green-400 text-xs">&#9679;</span>
              <span class="font-mono text-xs truncate">${this.value}</span>
            </span>`
          : html`<span class="text-gray-500">${this.label}</span>`
        }
      </button>
      ${this.open ? this.renderModal() : nothing}
    `;
  }

  private renderModal() {
    return html`
      <div
        class="overlay-backdrop is-open"
        @click=${(e: Event) => { if (e.target === e.currentTarget) this.close(); }}
      >
        <div class="overlay-panel rounded-2xl" style="width: 100%; max-width: 28rem;">
          <div class="flex items-center justify-between px-4 py-3 border-b border-white/10">
            <h3 class="text-sm font-semibold text-gray-200">Select Cocoon</h3>
            <button
              class="text-gray-500 hover:text-gray-300 transition-colors text-lg leading-none"
              @click=${() => this.close()}
            >&times;</button>
          </div>

          <div class="py-1 max-h-80 overflow-y-auto">
            ${this.items.length === 0
              ? html`<div class="px-4 py-6 text-center text-sm text-gray-500">No cocoons available</div>`
              : this.items.map(item => this.renderItem(item))
            }
          </div>

          ${this.error
            ? html`<div class="px-4 py-2 text-xs text-red-400">${this.error}</div>`
            : nothing
          }

          <div class="px-4 py-3 border-t border-white/10">
            <button
              class="w-full px-3 py-2 rounded-lg bg-white/5 text-sm text-gray-400 hover:bg-white/10 hover:text-gray-200 transition-colors"
              @click=${() => this.requestSetup()}
            >+ Setup New Cocoon</button>
          </div>
        </div>
      </div>
    `;
  }

  private renderItem(item: CocoonDisplayItem) {
    const isInstalling = this.installing === item.deviceId;
    const canSelect = item.pluginInstalled;
    const isSelected = this.value === item.deviceId;

    return html`
      <div
        class="flex items-center justify-between px-4 py-2.5 ${canSelect ? 'cursor-pointer hover:bg-white/5' : ''} ${isSelected ? 'bg-white/5' : ''} transition-colors"
        @click=${() => canSelect && this.select(item)}
      >
        <div class="flex items-center gap-2 min-w-0">
          <span class="text-xs ${item.online ? 'text-green-400' : 'text-gray-600'}">&#9679;</span>
          <span class="font-mono text-xs text-gray-300 truncate">${item.deviceId}</span>
        </div>

        <div class="flex items-center gap-2 shrink-0 ml-2">
          ${!item.online
            ? html`<span class="text-xs text-gray-600">offline</span>`
            : isSelected
              ? html`<span class="text-xs text-purple-400">selected</span>`
              : nothing
          }
          ${!item.pluginInstalled && this.withPlugin
            ? html`
                <button
                  class="px-2 py-1 rounded text-xs bg-purple-500/20 text-purple-300 hover:bg-purple-500/30 transition-colors disabled:opacity-50"
                  ?disabled=${isInstalling || !item.online}
                  @click=${(e: Event) => { e.stopPropagation(); this.installPlugin(item); }}
                >${isInstalling ? 'Installing...' : 'Install Plugin'}</button>
              `
            : nothing
          }
        </div>
      </div>
    `;
  }
}
