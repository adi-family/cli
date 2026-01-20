import { LitElement, html, css } from "lit";
import { customElement, property } from "lit/decorators.js";

@customElement("wave-bar")
export class WaveBar extends LitElement {
  @property({ type: String }) size: "sm" | "md" | "lg" = "md";
  @property({ type: String }) label = "";
  @property({ type: Number }) bars = 5;

  static styles = css`
    :host {
      display: flex;
      flex-direction: column;
      align-items: center;
      gap: 0.75rem;
    }

    .container {
      display: flex;
      align-items: center;
    }

    .container.sm { gap: 2px; }
    .container.md { gap: 4px; }
    .container.lg { gap: 6px; }

    .bar {
      border-radius: 9999px;
      background: linear-gradient(to top, #7c3aed, #a78bfa);
      animation: wave-bounce 1s ease-in-out infinite;
    }

    .container.sm .bar { width: 4px; height: 24px; }
    .container.md .bar { width: 6px; height: 40px; }
    .container.lg .bar { width: 8px; height: 64px; }

    .bar:nth-child(1) { animation-delay: 0s; }
    .bar:nth-child(2) { animation-delay: 0.1s; }
    .bar:nth-child(3) { animation-delay: 0.2s; }
    .bar:nth-child(4) { animation-delay: 0.3s; }
    .bar:nth-child(5) { animation-delay: 0.4s; }
    .bar:nth-child(6) { animation-delay: 0.5s; }
    .bar:nth-child(7) { animation-delay: 0.6s; }
    .bar:nth-child(8) { animation-delay: 0.7s; }
    .bar:nth-child(9) { animation-delay: 0.8s; }

    .label {
      font-size: 0.875rem;
      color: #9ca3af;
    }

    @keyframes wave-bounce {
      0%, 100% {
        transform: scaleY(0.3);
        opacity: 0.5;
      }
      50% {
        transform: scaleY(1);
        opacity: 1;
      }
    }
  `;

  render() {
    const barElements = Array.from({ length: this.bars }, () => html`<div class="bar"></div>`);
    
    return html`
      <div class="container ${this.size}">
        ${barElements}
      </div>
      ${this.label ? html`<span class="label">${this.label}</span>` : ""}
    `;
  }
}

declare global {
  interface HTMLElementTagNameMap {
    "wave-bar": WaveBar;
  }
}
