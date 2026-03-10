import { LitElement, html, nothing } from 'lit';
import { state } from 'lit/decorators.js';
import type { AdiCocoonTerminalElement } from './terminal';
import type { CocoonClient } from '@adi-family/plugin-cocoon';

export interface ControlCenterCocoon {
  deviceId: string;
  name?: string;
  online: boolean;
  signalingUrl: string;
}

export type ClientProvider = (deviceId: string) => CocoonClient | undefined;

export class AdiCocoonControlCenterElement extends LitElement {
  @state() cocoons: ControlCenterCocoon[] = [];
  clientProvider: ClientProvider | null = null;

  @state() private activeDeviceId: string | null = null;

  override createRenderRoot() { return this; }

  override updated(): void {
    if (!this.activeDeviceId || !this.clientProvider) return;
    const termEl = this.querySelector<AdiCocoonTerminalElement>('adi-cocoon-terminal');
    if (!termEl) return;
    const client = this.clientProvider(this.activeDeviceId) ?? null;
    termEl.client = client;
  }

  private openTerminal(deviceId: string): void {
    this.activeDeviceId = deviceId;
  }

  private closeTerminal(): void {
    const termEl = this.querySelector<AdiCocoonTerminalElement>('adi-cocoon-terminal');
    if (termEl) termEl.client = null;
    this.activeDeviceId = null;
  }

  override render() {
    if (this.activeDeviceId) {
      return this.renderTerminalView(this.activeDeviceId);
    }
    return this.renderCocoonList();
  }

  private renderTerminalView(deviceId: string) {
    const cocoon = this.cocoons.find(c => c.deviceId === deviceId);
    const label = cocoon?.name ?? deviceId.slice(0, 12) + '…';

    return html`
      <div style="display:flex;flex-direction:column;height:100vh;background:#0d0d0d;">
        <div style="display:flex;align-items:center;gap:10px;padding:8px 12px;border-bottom:1px solid #222;flex-shrink:0;background:#111;">
          <button
            style="padding:4px 10px;border:1px solid #333;border-radius:5px;background:transparent;color:#888;font-size:12px;cursor:pointer;"
            @click=${() => this.closeTerminal()}
          >← Back</button>
          <span style="color:#d4d4d4;font-size:13px;font-weight:500;">${label}</span>
          <div style="flex:1;"></div>
          <button
            style="padding:4px 10px;border:1px solid #333;border-radius:5px;background:transparent;color:#888;font-size:12px;cursor:pointer;"
            @click=${() => {
              const el = this.querySelector<AdiCocoonTerminalElement>('adi-cocoon-terminal');
              el?.restart();
            }}
          >↺ Restart</button>
        </div>
        <adi-cocoon-terminal style="flex:1;min-height:0;display:block;"></adi-cocoon-terminal>
      </div>
    `;
  }

  private renderCocoonList() {
    const onlineCocoons = this.cocoons.filter(c => c.online);
    const offlineCocoons = this.cocoons.filter(c => !c.online);

    return html`
      <div style="padding:16px;max-width:640px;">
        <h2 style="margin:0 0 16px;font-size:1.25rem;font-weight:600;">Control Center</h2>

        ${this.cocoons.length === 0 ? html`
          <div style="color:var(--text-muted,#888);font-size:0.9rem;">
            No cocoons connected. Start a cocoon and connect it to the signaling server.
          </div>
        ` : nothing}

        ${onlineCocoons.length > 0 ? html`
          <div style="display:flex;flex-direction:column;gap:8px;margin-bottom:16px;">
            ${onlineCocoons.map(c => this.renderCocoonRow(c))}
          </div>
        ` : nothing}

        ${offlineCocoons.length > 0 ? html`
          <div style="margin-top:8px;">
            <div style="font-size:0.75rem;color:var(--text-muted,#888);margin-bottom:6px;text-transform:uppercase;letter-spacing:0.05em;">Offline</div>
            <div style="display:flex;flex-direction:column;gap:6px;">
              ${offlineCocoons.map(c => this.renderCocoonRow(c))}
            </div>
          </div>
        ` : nothing}
      </div>
    `;
  }

  private renderCocoonRow(cocoon: ControlCenterCocoon) {
    const label = cocoon.name ?? cocoon.deviceId.slice(0, 16) + '…';
    const dotColor = cocoon.online ? 'var(--text-success,#4ade80)' : 'var(--text-muted,#555)';

    return html`
      <div style="display:flex;align-items:center;gap:10px;padding:10px 14px;border:1px solid var(--border-color,#333);border-radius:8px;">
        <span style="font-size:0.7rem;color:${dotColor};">●</span>
        <span style="flex:1;font-weight:500;font-size:0.9rem;">${label}</span>
        <span style="font-family:monospace;font-size:0.75rem;color:var(--text-muted,#888);">${cocoon.deviceId.slice(0, 8)}…</span>
        ${cocoon.online ? html`
          <button
            style="padding:5px 14px;border:none;border-radius:5px;background:var(--brand,#6366f1);color:white;font-size:0.8rem;cursor:pointer;"
            @click=${() => this.openTerminal(cocoon.deviceId)}
          >Terminal</button>
        ` : html`
          <span style="font-size:0.8rem;color:var(--text-muted,#555);padding:5px 14px;">Offline</span>
        `}
      </div>
    `;
  }
}
