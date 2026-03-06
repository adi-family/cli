import { LitElement, html } from 'lit';
import { customElement } from 'lit/decorators.js';
import { App } from '../app/app.ts';

@customElement('app-root')
export class AppRoot extends LitElement {
  override createRenderRoot() {
    return this;
  }

  override connectedCallback(): void {
    super.connectedCallback();
    if (App.instance) {
      this.#subscribe();
    } else {
      window.addEventListener('app-ready', () => this.#subscribe(), {
        once: true,
      });
    }
  }

  #subscribe(): void {
    const bus = App.reqInstance.bus;

    bus.use({
      before: (event, payload, meta) =>
        console.debug(
          `%c[event:before] ${event}`,
          'color: #7c9ef8; font-weight: bold',
          payload,
          meta,
        ),
      after: (event, payload, meta) =>
        console.debug(
          `%c[event:after]  ${event}`,
          'color: #a78bfa; font-weight: bold',
          payload,
          meta,
        ),
      ignored: (event, payload, meta) =>
        console.debug(
          `%c[event:ignored] ${event}`,
          'color: #f87171; font-weight: bold',
          payload,
          meta,
        ),
    });
  }

  override render() {
    return html`
      <div class="flex flex-col min-h-screen">
        <adi-slot name="top"></adi-slot>

        <div class="flex flex-1 min-h-0">
          <adi-slot name="left"></adi-slot>

          <main class="flex-1 min-w-0">
            <adi-slot name="center"></adi-slot>
          </main>

          <adi-slot name="right"></adi-slot>
        </div>

        <adi-slot name="bottom"></adi-slot>
      </div>

      <adi-slot name="overlays"></adi-slot>
    `;
  }
}
