import { LitElement, html, css } from "lit";
import { customElement, property } from "lit/decorators.js";

@customElement("ghost-button")
export class GhostButton extends LitElement {
  @property({ type: String }) size: "sm" | "md" | "lg" = "md";
  @property({ type: String }) label = "Button";
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
      background: transparent;
      color: #9ca3af;
      position: relative;
    }

    button:hover:not(:disabled) {
      background: rgba(255, 255, 255, 0.05);
      color: white;
    }

    button:active:not(:disabled) {
      background: rgba(255, 255, 255, 0.08);
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
      border: 2px solid rgba(156, 163, 175, 0.3);
      border-top-color: #9ca3af;
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
    "ghost-button": GhostButton;
  }
}
