import { LitElement, html, css } from "lit";
import { customElement, property } from "lit/decorators.js";

@customElement("morphing-blob")
export class MorphingBlob extends LitElement {
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
    }

    .container.sm { width: 3rem; height: 3rem; }
    .container.md { width: 5rem; height: 5rem; }
    .container.lg { width: 7rem; height: 7rem; }

    svg {
      width: 100%;
      height: 100%;
      filter: drop-shadow(0 0 12px rgba(139, 92, 246, 0.5));
    }

    .blob {
      fill: url(#blob-gradient);
    }

    .label {
      font-size: 0.875rem;
      color: #9ca3af;
    }
  `;

  render() {
    return html`
      <div class="container ${this.size}">
        <svg viewBox="0 0 100 100">
          <defs>
            <linearGradient id="blob-gradient" x1="0%" y1="0%" x2="100%" y2="100%">
              <stop offset="0%" stop-color="#a78bfa" />
              <stop offset="50%" stop-color="#8b5cf6" />
              <stop offset="100%" stop-color="#7c3aed" />
            </linearGradient>
          </defs>
          <path class="blob">
            <animate
              attributeName="d"
              dur="3s"
              repeatCount="indefinite"
              values="
                M50,20 C70,20 80,30 80,50 C80,70 70,80 50,80 C30,80 20,70 20,50 C20,30 30,20 50,20 Z;
                M50,15 C75,25 85,40 80,55 C75,75 60,85 45,80 C25,75 15,55 20,40 C25,20 35,15 50,15 Z;
                M55,20 C80,25 85,45 75,60 C65,80 45,85 30,75 C15,60 15,40 25,25 C40,15 45,18 55,20 Z;
                M45,18 C65,15 82,30 82,50 C82,72 68,82 48,82 C28,82 18,68 18,48 C18,28 28,18 45,18 Z;
                M50,20 C70,20 80,30 80,50 C80,70 70,80 50,80 C30,80 20,70 20,50 C20,30 30,20 50,20 Z
              "
              calcMode="spline"
              keySplines="0.4 0 0.2 1; 0.4 0 0.2 1; 0.4 0 0.2 1; 0.4 0 0.2 1"
            />
          </path>
        </svg>
      </div>
      ${this.label ? html`<span class="label">${this.label}</span>` : ""}
    `;
  }
}

declare global {
  interface HTMLElementTagNameMap {
    "morphing-blob": MorphingBlob;
  }
}
