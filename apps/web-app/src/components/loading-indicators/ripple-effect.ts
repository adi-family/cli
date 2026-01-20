import { LitElement, html, css } from "lit";
import { customElement, property } from "lit/decorators.js";

@customElement("ripple-effect")
export class RippleEffect extends LitElement {
  @property({ type: String }) size: "sm" | "md" | "lg" = "md";
  @property({ type: String }) label = "";

  static styles = css`
    :host {
      display: flex;
      flex-direction: column;
      align-items: center;
      gap: 0.75rem;
    }

    .container {
      position: relative;
      border-radius: 50%;
      background: radial-gradient(circle, rgba(139, 92, 246, 0.1), transparent);
    }

    .container.sm { width: 50px; height: 50px; }
    .container.md { width: 80px; height: 80px; }
    .container.lg { width: 110px; height: 110px; }

    .ripple {
      position: absolute;
      inset: 0;
      border-radius: 50%;
      border: 2px solid #8b5cf6;
      animation: ripple-expand 2s ease-out infinite;
      opacity: 0;
    }

    .ripple:nth-child(1) { animation-delay: 0s; }
    .ripple:nth-child(2) { animation-delay: 0.5s; }
    .ripple:nth-child(3) { animation-delay: 1s; }
    .ripple:nth-child(4) { animation-delay: 1.5s; }

    .center {
      position: absolute;
      inset: 35%;
      border-radius: 50%;
      background: linear-gradient(135deg, #c4b5fd, #7c3aed);
      box-shadow: 0 0 20px rgba(139, 92, 246, 0.6);
      animation: center-pulse 2s ease-in-out infinite;
    }

    .center::after {
      content: "";
      position: absolute;
      inset: 20%;
      border-radius: 50%;
      background: rgba(255, 255, 255, 0.3);
    }

    .label {
      font-size: 0.875rem;
      color: #9ca3af;
    }

    @keyframes ripple-expand {
      0% {
        transform: scale(0.3);
        opacity: 1;
        border-width: 4px;
      }
      100% {
        transform: scale(1);
        opacity: 0;
        border-width: 1px;
      }
    }

    @keyframes center-pulse {
      0%, 100% {
        transform: scale(1);
        box-shadow: 0 0 20px rgba(139, 92, 246, 0.6);
      }
      50% {
        transform: scale(0.9);
        box-shadow: 0 0 30px rgba(139, 92, 246, 0.8);
      }
    }
  `;

  render() {
    return html`
      <div class="container ${this.size}">
        <div class="ripple"></div>
        <div class="ripple"></div>
        <div class="ripple"></div>
        <div class="ripple"></div>
        <div class="center"></div>
      </div>
      ${this.label ? html`<span class="label">${this.label}</span>` : ""}
    `;
  }
}

declare global {
  interface HTMLElementTagNameMap {
    "ripple-effect": RippleEffect;
  }
}
