import { LitElement, html, css } from "lit";
import { customElement, property } from "lit/decorators.js";

@customElement("gradient-sweep")
export class GradientSweep extends LitElement {
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
    }

    .container.sm { width: 40px; height: 40px; }
    .container.md { width: 64px; height: 64px; }
    .container.lg { width: 88px; height: 88px; }

    .track {
      position: absolute;
      inset: 0;
      border-radius: 50%;
      background: rgba(139, 92, 246, 0.15);
    }

    .sweep {
      position: absolute;
      inset: 0;
      border-radius: 50%;
      background: conic-gradient(
        from 0deg,
        transparent 0deg,
        #c4b5fd 60deg,
        #8b5cf6 120deg,
        #7c3aed 180deg,
        transparent 180deg
      );
      animation: sweep-rotate 1.5s linear infinite;
      mask: radial-gradient(
        farthest-side,
        transparent calc(100% - 6px),
        black calc(100% - 6px)
      );
      -webkit-mask: radial-gradient(
        farthest-side,
        transparent calc(100% - 6px),
        black calc(100% - 6px)
      );
    }

    .glow {
      position: absolute;
      inset: -4px;
      border-radius: 50%;
      background: conic-gradient(
        from 0deg,
        transparent 0deg,
        rgba(139, 92, 246, 0.3) 90deg,
        transparent 180deg
      );
      animation: sweep-rotate 1.5s linear infinite;
      filter: blur(8px);
    }

    .inner {
      position: absolute;
      inset: 8px;
      border-radius: 50%;
      background: #0d0a14;
      display: flex;
      align-items: center;
      justify-content: center;
    }

    .dot {
      width: 6px;
      height: 6px;
      border-radius: 50%;
      background: #8b5cf6;
      animation: dot-pulse 1.5s ease-in-out infinite;
    }

    .label {
      font-size: 0.875rem;
      color: #9ca3af;
    }

    @keyframes sweep-rotate {
      from { transform: rotate(0deg); }
      to { transform: rotate(360deg); }
    }

    @keyframes dot-pulse {
      0%, 100% { transform: scale(1); opacity: 1; }
      50% { transform: scale(0.6); opacity: 0.5; }
    }
  `;

  render() {
    return html`
      <div class="container ${this.size}">
        <div class="track"></div>
        <div class="glow"></div>
        <div class="sweep"></div>
        <div class="inner">
          <div class="dot"></div>
        </div>
      </div>
      ${this.label ? html`<span class="label">${this.label}</span>` : ""}
    `;
  }
}

declare global {
  interface HTMLElementTagNameMap {
    "gradient-sweep": GradientSweep;
  }
}
