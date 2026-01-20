import { LitElement, html, css } from "lit";
import { customElement, property } from "lit/decorators.js";

@customElement("icon-button")
export class IconButton extends LitElement {
  @property({ type: String }) size: "sm" | "md" | "lg" = "md";
  @property({ type: String }) icon = ""; // SVG path or emoji
  @property({ type: String }) label = "";
  @property({ type: Boolean }) disabled = false;
  @property({ type: String }) variant: "default" | "primary" | "danger" = "default";

  static styles = css`
    :host {
      display: inline-flex;
    }

    button {
      display: inline-flex;
      align-items: center;
      justify-content: center;
      border: none;
      border-radius: 0.5rem;
      font-family: inherit;
      cursor: pointer;
      transition: all 0.2s ease;
      position: relative;
    }

    /* Default variant */
    button.default {
      background: rgba(255, 255, 255, 0.05);
      color: #9ca3af;
    }

    button.default:hover:not(:disabled) {
      background: rgba(255, 255, 255, 0.1);
      color: white;
    }

    /* Primary variant */
    button.primary {
      background: rgba(139, 92, 246, 0.15);
      color: #a78bfa;
    }

    button.primary:hover:not(:disabled) {
      background: rgba(139, 92, 246, 0.25);
      color: #c4b5fd;
    }

    /* Danger variant */
    button.danger {
      background: rgba(239, 68, 68, 0.15);
      color: #f87171;
    }

    button.danger:hover:not(:disabled) {
      background: rgba(239, 68, 68, 0.25);
      color: #fca5a5;
    }

    button:active:not(:disabled) {
      transform: scale(0.95);
    }

    button:disabled {
      opacity: 0.5;
      cursor: not-allowed;
    }

    button.sm {
      width: 28px;
      height: 28px;
      font-size: 0.875rem;
    }

    button.md {
      width: 36px;
      height: 36px;
      font-size: 1rem;
    }

    button.lg {
      width: 44px;
      height: 44px;
      font-size: 1.25rem;
    }

    .icon {
      display: flex;
      align-items: center;
      justify-content: center;
    }

    .icon svg {
      width: 1em;
      height: 1em;
    }
  `;

  render() {
    return html`
      <button 
        class="${this.size} ${this.variant}" 
        ?disabled=${this.disabled}
        title=${this.label}
        aria-label=${this.label}
      >
        <span class="icon">
          <slot>${this.icon}</slot>
        </span>
      </button>
    `;
  }
}

declare global {
  interface HTMLElementTagNameMap {
    "icon-button": IconButton;
  }
}
