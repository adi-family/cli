import { LitElement, html } from 'lit';
import { customElement, property, state } from 'lit/decorators.js';
import type { EventBus } from '@adi-family/sdk-plugin';
import type { WsState } from '../services/signaling/index.ts';

const dotColor: Record<WsState, string> = {
  connected: 'bg-green-400',
  connecting: 'bg-yellow-400 animate-pulse',
  disconnected: 'bg-border',
  error: 'bg-red-400',
};

const labelColor: Record<WsState, string> = {
  connected: 'text-green-400',
  connecting: 'text-yellow-400',
  disconnected: 'text-text-muted',
  error: 'text-red-400',
};

@customElement('signaling-status')
export class SignalingStatus extends LitElement {
  @property() url = '';
  @state() private wsState: WsState = 'disconnected';

  private unsub: (() => void) | null = null;

  override createRenderRoot() { return this; }

  override connectedCallback(): void {
    super.connectedCallback();
    if ((window as { sdk?: unknown }).sdk) {
      this.#subscribe();
    } else {
      window.addEventListener('app-ready', () => this.#subscribe(), { once: true });
    }
  }

  override disconnectedCallback(): void {
    super.disconnectedCallback();
    this.unsub?.();
    this.unsub = null;
  }

  #subscribe(): void {
    const bus = window.sdk.bus as EventBus;
    this.unsub = bus.on('signaling:state', ({ url, state }) => {
      if (url === this.url) this.wsState = state;
    }, 'signaling-status');
  }

  override render() {
    return html`
      <span class="inline-flex items-center gap-1.5 text-xs font-medium">
        <span class="w-2 h-2 rounded-full shrink-0 ${dotColor[this.wsState]}"></span>
        <span class="${labelColor[this.wsState]}">${this.wsState}</span>
      </span>
    `;
  }
}
