import { LitElement, html } from "lit";
import { customElement, property, state } from "lit/decorators.js";
import type { AsyncClickHandler } from "./base-button.js";

/// Square icon-only button with variant support. Sizing via ADID AX system.
@customElement("adi-icon-button")
export class AdiIconButton extends LitElement {
  @property({ type: String }) icon = "";
  @property({ type: String }) label = "";
  @property({ type: Boolean }) disabled = false;
  @property({ type: Boolean }) loading = false;
  @property({ type: String }) variant: "default" | "primary" | "danger" = "default";
  @property({ attribute: false }) onClick?: AsyncClickHandler;

  @state() private _internalLoading = false;

  createRenderRoot() {
    return this;
  }

  private get isLoading(): boolean {
    return this.loading || this._internalLoading;
  }

  private get isDisabled(): boolean {
    return this.disabled || this.isLoading;
  }

  private async handleClick(e: MouseEvent) {
    if (this.isDisabled || !this.onClick) return;

    this._internalLoading = true;
    try {
      await this.onClick(e);
    } finally {
      this._internalLoading = false;
    }
  }

  private getVariantStyles(): string {
    switch (this.variant) {
      case "primary":
        return "background: var(--adi-accent-soft); color: var(--adi-accent);";
      case "danger":
        return "background: var(--adi-error-soft); color: var(--adi-error);";
      default:
        return "background: color-mix(in srgb, var(--adi-text) 5%, transparent); color: var(--adi-text-muted);";
    }
  }

  render() {
    return html`
      <button
        class="adi-btn adi-icon-btn"
        ?disabled=${this.isDisabled}
        title=${this.label}
        aria-label=${this.label}
        @click=${this.handleClick}
        style="
          display: inline-flex;
          align-items: center;
          justify-content: center;
          width: calc(var(--l) * 2.5);
          height: calc(var(--l) * 2.5);
          font-size: var(--t);
          border: none;
          border-radius: var(--r);
          cursor: pointer;
          transition: background-color 200ms, color 200ms, transform 200ms;
          ${this.getVariantStyles()}
        "
      >
        ${this.isLoading
          ? html`<span class="adi-btn-spinner"></span>`
          : html`<span style="display:flex;align-items:center;justify-content:center;">
              <slot>${this.icon}</slot>
            </span>`}
      </button>
    `;
  }
}

declare global {
  interface HTMLElementTagNameMap {
    "adi-icon-button": AdiIconButton;
  }
}
