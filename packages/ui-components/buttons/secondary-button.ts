import { html } from "lit";
import { customElement } from "lit/decorators.js";
import { BaseButton } from "./base-button.js";

@customElement("adi-secondary-button")
export class AdiSecondaryButton extends BaseButton {
  render() {
    return html`
      <button
        class="adi-btn adi-btn-secondary"
        ?disabled=${this.isDisabled}
        @click=${this.handleClick}
        style="
          display: inline-flex;
          align-items: center;
          justify-content: center;
          gap: calc(1rem * 0.5);
          padding: calc(1rem * 0.75) calc(1rem * 1.75);
          font-size: calc(1rem * 0.875);
          font-weight: 500;
          line-height: 1;
          border: 1px solid var(--adi-border);
          border-radius: 0.75rem;
          background: transparent;
          color: var(--adi-text-muted);
          cursor: pointer;
          transition: background-color 200ms, border-color 200ms, color 200ms;
          text-decoration: none;
        "
      >
        ${this.isLoading ? html`<span class="adi-btn-spinner"></span>` : ""}
        <slot>${this.displayText}</slot>
      </button>
    `;
  }
}

declare global {
  interface HTMLElementTagNameMap {
    "adi-secondary-button": AdiSecondaryButton;
  }
}
