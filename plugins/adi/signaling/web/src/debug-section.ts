import { LitElement, html, nothing } from 'lit';
import { customElement, state } from 'lit/decorators.js';
import type { WsState, DeviceInfo, RoomInfo } from './generated';

export interface SignalingServerDebugInfo {
  url: string;
  state: WsState;
  authenticated: boolean;
  userId: string | null;
  deviceId: string | null;
  peers: string[];
  devices: DeviceInfo[];
  rooms: RoomInfo[];
}

@customElement('adi-signaling-debug')
export class AdiSignalingDebugElement extends LitElement {
  @state() servers: SignalingServerDebugInfo[] = [];

  override createRenderRoot() {
    return this;
  }

  override render() {
    return html`
      <div style="display:flex;flex-direction:column;gap:1rem">
        <section>
          <div class="text-xs uppercase" style="color:var(--adi-text-muted);font-weight:600;margin-bottom:0.5rem">
            Signaling Servers (${this.servers.length})
          </div>
          ${this.servers.length === 0
            ? html`<p class="text-sm" style="color:var(--adi-text-muted)">No servers connected.</p>`
            : this.servers.map((s) => this.renderServer(s))}
        </section>
      </div>
    `;
  }

  private renderServer(s: SignalingServerDebugInfo) {
    return html`
      <div style="border:1px solid var(--adi-border);border-radius:0.5rem;padding:0.75rem;margin-bottom:0.5rem">
        <div style="display:flex;align-items:center;gap:0.5rem;margin-bottom:0.5rem">
          ${this.renderStateIndicator(s.state)}
          <code class="text-sm" style="word-break:break-all">${s.url}</code>
        </div>

        <table class="dr-table text-sm" style="width:100%">
          <tbody>
            <tr class="dr-row">
              <td class="dr-td" style="color:var(--adi-text-muted);width:120px">State</td>
              <td class="dr-td">${s.state}</td>
            </tr>
            <tr class="dr-row">
              <td class="dr-td" style="color:var(--adi-text-muted)">Authenticated</td>
              <td class="dr-td">${s.authenticated ? html`<span style="color:var(--adi-accent)">Yes</span>` : 'No'}</td>
            </tr>
            ${s.userId
              ? html`<tr class="dr-row">
                  <td class="dr-td" style="color:var(--adi-text-muted)">User ID</td>
                  <td class="dr-td"><code>${s.userId}</code></td>
                </tr>`
              : nothing}
            ${s.deviceId
              ? html`<tr class="dr-row">
                  <td class="dr-td" style="color:var(--adi-text-muted)">Device ID</td>
                  <td class="dr-td"><code>${s.deviceId}</code></td>
                </tr>`
              : nothing}
            <tr class="dr-row">
              <td class="dr-td" style="color:var(--adi-text-muted)">Peers</td>
              <td class="dr-td">
                ${s.peers.length === 0
                  ? html`<span style="color:var(--adi-text-muted)">none</span>`
                  : s.peers.map((p) => html`<code style="margin-right:0.25rem">${p}</code>`)}
              </td>
            </tr>
            <tr class="dr-row">
              <td class="dr-td" style="color:var(--adi-text-muted)">Devices</td>
              <td class="dr-td">
                ${s.devices.length === 0
                  ? html`<span style="color:var(--adi-text-muted)">none</span>`
                  : s.devices.map(
                      (d) => html`
                        <div style="margin-bottom:0.25rem">
                          ${d.online
                            ? html`<span style="color:var(--adi-accent)">●</span>`
                            : html`<span style="color:var(--adi-text-muted)">●</span>`}
                          <code style="font-size:0.75rem">${d.device_id.slice(0, 12)}…</code>
                          ${d.device_type
                            ? html`<span class="text-xs" style="background:var(--adi-accent);color:var(--adi-bg);padding:0 4px;border-radius:3px;margin-left:0.25rem">${d.device_type}</span>`
                            : nothing}
                          ${Object.keys(d.tags).length > 0
                            ? html`<span class="text-xs" style="color:var(--adi-text-muted);margin-left:0.25rem">${Object.entries(d.tags).map(([k, v]) => `${k}=${v}`).join(', ')}</span>`
                            : nothing}
                        </div>`,
                    )}
              </td>
            </tr>
            <tr class="dr-row">
              <td class="dr-td" style="color:var(--adi-text-muted)">Rooms</td>
              <td class="dr-td">
                ${s.rooms.length === 0
                  ? html`<span style="color:var(--adi-text-muted)">none</span>`
                  : s.rooms.map(
                      (r) => html`
                        <div style="margin-bottom:0.25rem">
                          <code style="font-size:0.75rem">${r.room_id}</code>
                          <span class="text-xs" style="color:var(--adi-text-muted);margin-left:0.25rem">
                            ${r.actors.length} actor${r.actors.length !== 1 ? 's' : ''},
                            ${r.granted_users.length + 1} user${r.granted_users.length !== 0 ? 's' : ''}
                          </span>
                        </div>`,
                    )}
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    `;
  }

  private renderStateIndicator(state: WsState) {
    const colors: Record<string, string> = {
      connected: 'var(--adi-accent)',
      connecting: 'var(--adi-warning, orange)',
      disconnected: 'var(--adi-text-muted)',
      error: 'var(--adi-error, red)',
    };
    return html`<span style="color:${colors[state] ?? colors.disconnected}">●</span>`;
  }
}
