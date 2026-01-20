import { LitElement, html, css } from "lit";
import { customElement, property } from "lit/decorators.js";

@customElement("danger-button")
export class DangerButton extends LitElement {
  @property({ type: String }) size: "sm" | "md" | "lg" = "md";
  @property({ type: String }) label = "Delete";
  @property({ type: Boolean }) disabled = false;
  @property({ type: Boolean }) loading = false;

  static styles = css`
    :host {
      display: inline-flex;
    }

    button {
      display: inline-flex;
      align-items: center;
      justify-content: center;
      gap: 0.5rem;
      border: none;
      border-radius: 0.5rem;
      font-family: inherit;
      font-weight: 500;
      cursor: pointer;
      transition: all 0.2s ease;
      background: linear-gradient(135deg, #ef4444 0%, #dc2626 100%);
      color: white;
      position: relative;
      overflow: hidden;
    }

    button::before {
      content: "";
      position: absolute;
      inset: 0;
      background: linear-gradient(135deg, rgba(255,255,255,0.1) 0%, transparent 50%);
      opacity: 0;
      transition: opacity 0.2s;
    }

    button:hover:not(:disabled)::before {
      opacity: 1;
    }

    button:hover:not(:disabled) {
      transform: translateY(-1px);
      box-shadow: 0 4px 12px rgba(239, 68, 68, 0.4);
    }

    button:active:not(:disabled) {
      transform: translateY(0);
      box-shadow: 0 2px 6px rgba(239, 68, 68, 0.3);
    }

    button:disabled {
      opacity: 0.5;
      cursor: not-allowed;
    }

    button.sm {
      padding: 0.375rem 0.75rem;
      font-size: 0.75rem;
      min-height: 28px;
    }

    button.md {
      padding: 0.5rem 1rem;
      font-size: 0.875rem;
      min-height: 36px;
    }

    button.lg {
      padding: 0.75rem 1.5rem;
      font-size: 1rem;
      min-height: 44px;
    }

    .spinner {
      width: 1em;
      height: 1em;
      border: 2px solid rgba(255, 255, 255, 0.3);
      border-top-color: white;
      border-radius: 50%;
      animation: spin 0.6s linear infinite;
    }

    @keyframes spin {
      to { transform: rotate(360deg); }
    }
  `;

  render() {
    return html`
      <button class=${this.size} ?disabled=${this.disabled || this.loading}>
        ${this.loading ? html`<span class="spinner"></span>` : ""}
        <slot>${this.label}</slot>
      </button>
    `;
  }
}

declare global {
  interface HTMLElementTagNameMap {
    "danger-button": DangerButton;
  }
}
