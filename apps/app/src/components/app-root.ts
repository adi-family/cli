import { LitElement, html } from "lit";
import { customElement } from "lit/decorators.js";

@customElement("app-root")
export class AppRoot extends LitElement {
  createRenderRoot() {
    return this;
  }

  render() {
    return html`
      <main class="flex items-center justify-center min-h-screen">
        <h1 class="text-4xl font-bold text-text">App</h1>
      </main>
    `;
  }
}
