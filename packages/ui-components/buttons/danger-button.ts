import { html } from "lit";
import { customElement } from "lit/decorators.js";
import { BaseButton } from "./base-button.js";

/// Red destructive action button. Sizing via ADID AX system.
@customElement("adi-danger-button")
export class AdiDangerButton extends BaseButton {
  constructor() {
    super();
    this.label = "Delete";
  }

  render() {
    return html`
      <button
        class="adi-btn adi-btn-danger"
        ?disabled=${this.isDisabled}
        @click=${this.handleClick}
        style="
          display: inline-flex;
          align-items: center;
          justify-content: center;
          gap: calc(var(--l) * 0.5);
          padding: calc(var(--l) * 0.75) calc(var(--l) * 1.75);
          font-size: calc(var(--t) * 0.875);
          font-weight: 500;
          line-height: 1;
          border: 1px solid color-mix(in srgb, var(--adi-error) 30%, transparent);
          border-radius: var(--r);
          background: color-mix(in srgb, var(--adi-error) 6%, transparent);
          color: var(--adi-error);
          cursor: pointer;
          transition: background-color 200ms, border-color 200ms, color 200ms, transform 200ms, box-shadow 200ms;
          position: relative;
          overflow: hidden;
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
    "adi-danger-button": AdiDangerButton;
  }
}
