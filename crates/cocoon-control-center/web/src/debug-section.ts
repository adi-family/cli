import { LitElement, html, nothing } from 'lit';
import { customElement, state } from 'lit/decorators.js';
import type { ControlCenterCocoon, ClientProvider } from './component';
import type { AdiCocoonTerminalElement } from './terminal';

@customElement('adi-cocoon-control-center-debug')
export class AdiCocoonControlCenterDebugElement extends LitElement {
  @state() cocoons: ControlCenterCocoon[] = [];
  clientProvider: ClientProvider | null = null;

  @state() private openDeviceId: string | null = null;

  override createRenderRoot() {
    return this;
  }

  override updated(): void {
    if (!this.openDeviceId || !this.clientProvider) return;
    const termEl = this.querySelector<AdiCocoonTerminalElement>('adi-cocoon-terminal');
    if (!termEl) return;
    const client = this.clientProvider(this.openDeviceId) ?? null;
    termEl.client = client;
  }

  private openTerminal(deviceId: string): void {
    this.openDeviceId = deviceId;
  }

  private closeTerminal(): void {
    const termEl = this.querySelector<AdiCocoonTerminalElement>('adi-cocoon-terminal');
    if (termEl) termEl.client = null;
    this.openDeviceId = null;
  }

  override render() {
    if (this.openDeviceId) return this.renderTerminal(this.openDeviceId);
    return this.renderDeviceList();
  }

  private renderTerminal(deviceId: string) {
    const cocoon = this.cocoons.find(c => c.deviceId === deviceId);
    const label = cocoon?.name ?? deviceId.slice(0, 12) + '…';

    return html`
      <div style="display:flex;flex-direction:column;height:400px;">
        <div style="display:flex;align-items:center;gap:8px;padding:4px 6px;border-bottom:1px solid var(--adi-border,#333);flex-shrink:0;">
          <button
            style="padding:2px 8px;border:1px solid var(--adi-border,#333);border-radius:4px;background:transparent;color:var(--adi-text-muted,#888);font-size:11px;cursor:pointer;"
            @click=${() => this.closeTerminal()}
          >← Back</button>
          <code style="font-size:11px;color:var(--adi-text-muted,#888);">${label}</code>
          <button
            style="margin-left:auto;padding:2px 8px;border:1px solid var(--adi-border,#333);border-radius:4px;background:transparent;color:var(--adi-text-muted,#888);font-size:11px;cursor:pointer;"
            @click=${() => {
              const el = this.querySelector<AdiCocoonTerminalElement>('adi-cocoon-terminal');
              el?.restart();
            }}
          >↺</button>
        </div>
        <adi-cocoon-terminal style="flex:1;min-height:0;display:block;"></adi-cocoon-terminal>
      </div>
    `;
  }

  private renderDeviceList() {
    if (this.cocoons.length === 0) {
      return html`<div style="color:var(--adi-text-muted,#888);padding:6px 0;font-size:12px;">No cocoons tracked</div>`;
    }

    return html`
      <div>
        <div style="font-size:11px;text-transform:uppercase;color:var(--adi-text-muted,#888);font-weight:600;margin-bottom:6px;">
          Cocoons (${this.cocoons.length})
        </div>
        ${this.cocoons.map(c => html`
          <div style="border:1px solid var(--adi-border,#333);border-radius:4px;padding:5px 8px;margin-bottom:4px;font-size:12px;">
            <div style="display:flex;align-items:center;gap:6px;">
              <span style="font-size:0.65rem;color:${c.online ? 'var(--adi-accent,#6366f1)' : 'var(--adi-text-muted,#555)'};">●</span>
              <code style="font-size:0.75rem;flex:1;">${c.deviceId.slice(0, 16)}…</code>
              ${c.name ? html`<span style="color:var(--adi-text-muted,#888);font-size:11px;">${c.name}</span>` : nothing}
              ${c.online ? html`
                <button
                  style="padding:2px 8px;border:none;border-radius:3px;background:var(--brand,#6366f1);color:white;font-size:11px;cursor:pointer;"
                  @click=${() => this.openTerminal(c.deviceId)}
                >Terminal</button>
              ` : html`
                <span style="font-size:11px;color:var(--adi-text-muted,#555);">offline</span>
              `}
            </div>
            <div style="margin-top:2px;font-size:10px;color:var(--adi-text-muted,#555);word-break:break-all;">${c.signalingUrl}</div>
          </div>
        `)}
      </div>
    `;
  }
}
